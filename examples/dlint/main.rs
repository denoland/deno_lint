// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use annotate_snippets::display_list;
use annotate_snippets::snippet;
use anyhow::bail;
use anyhow::Error as AnyError;
use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::diagnostic::Range;
use deno_lint::linter::LinterBuilder;
use deno_lint::linter::SourceFile;
use deno_lint::rules::{get_all_rules, get_recommended_rules, LintRule};
use log::debug;
use rayon::prelude::*;
use serde::Serialize;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

mod config;
mod js;

fn create_cli_app<'a, 'b>() -> App<'a, 'b> {
  App::new("dlint")
    .setting(AppSettings::SubcommandRequiredElseHelp)
    .subcommand(
      SubCommand::with_name("rules")
        .arg(
          Arg::with_name("RULE_NAME")
            .help("Show detailed information about rule"),
        )
        .arg(Arg::with_name("json").long("json"))
        .arg(Arg::with_name("all").long("all")),
    )
    .subcommand(
      SubCommand::with_name("run")
        .arg(
          Arg::with_name("FILES")
            .help("Set the input file to use")
            .multiple(true),
        )
        .arg(
          Arg::with_name("RULE_CODE")
            .long("rule")
            .help("Run a certain rule")
            .takes_value(true),
        )
        .arg(
          Arg::with_name("CONFIG")
            .long("config")
            .help("Load config from file")
            .takes_value(true),
        )
        .arg(
          Arg::with_name("PLUGIN")
            .long("plugin")
            .help("Specify plugin paths")
            .multiple(true)
            .takes_value(true),
        ),
    )
}

// Return slice of source code covered by diagnostic
// and adjusted range of diagnostic (ie. original range - start line
// of sliced source code).
fn get_slice_source_and_range<'a>(
  line_start_indexes: &[(usize, usize)],
  source: &'a str,
  range: &Range,
) -> (&'a str, (usize, usize)) {
  let (_, first_line_start) = line_start_indexes[range.start.line - 1];
  let last_line_end = if range.end.line == line_start_indexes.len() {
    source.len() - 1
  } else {
    let (last_line_no, _) = line_start_indexes[range.end.line - 1];
    line_start_indexes[last_line_no + 1].1 - 1
  };
  let adjusted_start = range.start.byte_pos - first_line_start;
  let adjusted_end = range.end.byte_pos - first_line_start;
  let adjusted_range = (adjusted_start, adjusted_end);
  let slice_str = &source[first_line_start..last_line_end];
  (slice_str, adjusted_range)
}

fn display_diagnostics(
  diagnostics: &[LintDiagnostic],
  source_file: Rc<SourceFile>,
) {
  let source_code = &source_file.src;
  let line_start_indexes = source_file
    .lines
    .iter()
    .map(|pos| pos.0 as usize)
    .enumerate()
    .collect::<Vec<_>>();

  for diagnostic in diagnostics {
    let (slice_source, range) = get_slice_source_and_range(
      &line_start_indexes,
      source_code,
      &diagnostic.range,
    );
    let footer = if let Some(hint) = &diagnostic.hint {
      vec![snippet::Annotation {
        label: Some(hint),
        id: None,
        annotation_type: snippet::AnnotationType::Help,
      }]
    } else {
      vec![]
    };

    let snippet = snippet::Snippet {
      title: Some(snippet::Annotation {
        label: Some(&diagnostic.message),
        id: Some(&diagnostic.code),
        annotation_type: snippet::AnnotationType::Error,
      }),
      footer,
      slices: vec![snippet::Slice {
        source: &slice_source,
        line_start: diagnostic.range.start.line,
        origin: Some(&diagnostic.filename),
        fold: false,
        annotations: vec![snippet::SourceAnnotation {
          range,
          label: "",
          annotation_type: snippet::AnnotationType::Error,
        }],
      }],
      opt: display_list::FormatOptions {
        color: true,
        anonymized_line_numbers: false,
        margin: None,
      },
    };
    let display_list = display_list::DisplayList::from(snippet);
    eprintln!("{}", display_list);
  }
}

fn run_linter(
  paths: Vec<String>,
  filter_rule_name: Option<&str>,
  maybe_config: Option<Arc<config::Config>>,
  plugin_paths: Vec<&str>,
) -> Result<(), AnyError> {
  let mut paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

  if let Some(config) = maybe_config.clone() {
    paths.extend(config.get_files()?);
  }

  let error_counts = Arc::new(AtomicUsize::new(0));
  let output_lock = Arc::new(Mutex::new(())); // prevent threads outputting at the same time

  paths.par_iter().for_each(|file_path| {
    let source_code =
      std::fs::read_to_string(&file_path).expect("Failed to load file");

    let mut rules = if let Some(config) = maybe_config.clone() {
      config.get_rules()
    } else {
      get_recommended_rules()
    };

    if let Some(rule_name) = filter_rule_name {
      rules = rules
        .into_iter()
        .filter(|r| r.code() == rule_name)
        .collect()
    };

    debug!("Configured rules: {}", rules.len());

    let mut linter_builder = LinterBuilder::default()
      .rules(rules)
      .lint_unknown_rules(true)
      .lint_unused_ignore_directives(true);

    for plugin_path in &plugin_paths {
      let js_runner = js::JsRuleRunner::new(plugin_path);
      linter_builder = linter_builder.add_plugin(js_runner);
    }

    let mut linter = linter_builder.build();

    let (source_file, file_diagnostics) = linter
      .lint(file_path.to_string_lossy().to_string(), source_code)
      .expect("Failed to lint");

    error_counts.fetch_add(file_diagnostics.len(), Ordering::Relaxed);
    let _g = output_lock.lock().unwrap();

    display_diagnostics(&file_diagnostics, source_file);
  });

  let err_count = error_counts.load(Ordering::Relaxed);
  if err_count > 0 {
    eprintln!("Found {} problems", err_count);
    std::process::exit(1);
  }

  Ok(())
}

#[derive(Clone, Copy, Serialize)]
struct Rule {
  code: &'static str,
  docs: &'static str,
  tags: &'static [&'static str],
}

enum RuleTag {
  Recommended,
  All,
}

fn get_rules_by_tag(tag: RuleTag) -> Vec<Rule> {
  fn to_rule(rule: Box<dyn LintRule>) -> Rule {
    Rule {
      code: rule.code(),
      docs: rule.docs(),
      tags: rule.tags(),
    }
  }

  match tag {
    RuleTag::Recommended => {
      get_recommended_rules().into_iter().map(to_rule).collect()
    }
    RuleTag::All => get_all_rules().into_iter().map(to_rule).collect(),
  }
}

trait RuleFormatter {
  fn format(rules: &mut [Rule]) -> Result<String, &'static str>;
}

enum JsonFormatter {}
enum PrettyFormatter {}

impl RuleFormatter for JsonFormatter {
  fn format(rules: &mut [Rule]) -> Result<String, &'static str> {
    if rules.is_empty() {
      return Err("Rule not found!");
    }
    serde_json::to_string_pretty(rules).map_err(|_| "failed to format!")
  }
}

impl RuleFormatter for PrettyFormatter {
  fn format(rules: &mut [Rule]) -> Result<String, &'static str> {
    if rules.is_empty() {
      return Err("Rule not found!");
    }

    if rules.len() == 1 {
      let rule = &rules[0];
      let docs = if rule.docs.is_empty() {
        "documentation not available"
      } else {
        rule.docs
      };
      return Ok(format!("- {code}\n\n{docs}", code = rule.code, docs = docs));
    }

    rules.sort_by_key(|r| r.code);
    let mut list = Vec::with_capacity(1 + rules.len());
    list.push("Available rules (trailing ✔️ mark indicates it is included in the recommended rule set):".to_string());
    list.extend(rules.iter().map(|r| {
      let mut s = format!(" - {}", r.code);
      if r.tags.contains(&"recommended") {
        s += " ✔️";
      }
      s
    }));
    Ok(list.join("\n"))
  }
}

fn print_rules<F: RuleFormatter>(rules: &mut [Rule]) {
  match F::format(rules) {
    Err(e) => {
      eprintln!("{}", e);
      std::process::exit(1);
    }
    Ok(text) => {
      println!("{}", text);
    }
  }
}

fn filter_rules(rules: Vec<Rule>, rule_name: &str) -> Vec<Rule> {
  rules.into_iter().filter(|r| r.code == rule_name).collect()
}

fn main() -> Result<(), AnyError> {
  env_logger::init();

  let cli_app = create_cli_app();
  let matches = cli_app.get_matches();

  match matches.subcommand() {
    ("run", Some(run_matches)) => {
      let maybe_config = if let Some(p) = run_matches.value_of("CONFIG") {
        let path = PathBuf::from(p);

        let c = match path.extension().and_then(|s| s.to_str()) {
          Some("json") => config::load_from_json(&path)?,
          ext => bail!("Unknown extension: \"{:#?}\". Use .json instead.", ext),
        };
        Some(Arc::new(c))
      } else {
        None
      };

      debug!("Config: {:#?}", maybe_config);

      let plugins: Vec<&str> = run_matches
        .values_of("PLUGIN")
        .unwrap_or_default()
        .collect();

      let paths: Vec<String> = run_matches
        .values_of("FILES")
        .unwrap_or_default()
        .map(|p| p.to_string())
        .collect();
      run_linter(
        paths,
        run_matches.value_of("RULE_CODE"),
        maybe_config,
        plugins,
      )?;
    }
    ("rules", Some(rules_matches)) => {
      let json = rules_matches.is_present("json");
      let tag = if rules_matches.is_present("all") {
        RuleTag::All
      } else {
        RuleTag::Recommended
      };
      let mut rules =
        if let Some(rule_name) = rules_matches.value_of("RULE_NAME") {
          filter_rules(get_rules_by_tag(tag), rule_name)
        } else {
          get_rules_by_tag(tag)
        };
      if json {
        print_rules::<JsonFormatter>(&mut rules);
      } else {
        print_rules::<PrettyFormatter>(&mut rules);
      }
    }
    _ => unreachable!(),
  };

  Ok(())
}
