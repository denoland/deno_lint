// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use annotate_snippets::display_list;
use annotate_snippets::snippet;
use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::diagnostic::Range;
use deno_lint::linter::FileType;
use deno_lint::linter::LinterBuilder;
use deno_lint::rules::{get_all_rules, get_recommended_rules, LintRule};
use rayon::prelude::*;
use serde::Serialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

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
          Arg::with_name("script")
            .long("script")
            .help("Treat files as scripts instead of modules"),
        )
        .arg(
          Arg::with_name("FILES")
            .help("Sets the input file to use")
            .required(true)
            .multiple(true),
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

fn display_diagnostic(diagnostic: &LintDiagnostic, source: &str) {
  let line_start_indexes = std::iter::once(0)
    .chain(source.match_indices('\n').map(|l| l.0 + 1))
    .enumerate()
    .collect::<Vec<_>>();
  let (slice_source, range) =
    get_slice_source_and_range(&line_start_indexes, source, &diagnostic.range);

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

fn run_linter(paths: Vec<String>, is_script: bool) {
  let error_counts = Arc::new(AtomicUsize::new(0));
  let output_lock = Arc::new(Mutex::new(())); // prevent threads outputting at the same time

  paths.par_iter().for_each(|file_path| {
    let file_type = if is_script {
      FileType::Script
    } else {
      FileType::Module
    };

    let source_code =
      std::fs::read_to_string(&file_path).expect("Failed to read file");

    let mut linter = LinterBuilder::default()
      .rules(get_recommended_rules())
      .build();

    let file_diagnostics = linter
      .lint(file_path.to_string(), source_code.clone(), file_type)
      .expect("Failed to lint");

    error_counts.fetch_add(file_diagnostics.len(), Ordering::Relaxed);
    let _g = output_lock.lock().unwrap();

    for diagnostic in file_diagnostics {
      display_diagnostic(&diagnostic, &source_code);
    }
  });

  let err_count = error_counts.load(Ordering::Relaxed);
  if err_count > 0 {
    eprintln!("Found {} problems", err_count);
    std::process::exit(1);
  }
}

#[derive(Clone, Copy, Serialize)]
struct Rule {
  code: &'static str,
  docs: &'static str,
  #[serde(skip_serializing)]
  recommended: bool,
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
      recommended: rule.tags().contains(&"recommended"),
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
      if r.recommended {
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

fn main() {
  env_logger::init();

  let cli_app = create_cli_app();
  let matches = cli_app.get_matches();

  match matches.subcommand() {
    ("run", Some(run_matches)) => {
      let paths: Vec<String> = run_matches
        .values_of("FILES")
        .unwrap()
        .map(|p| p.to_string())
        .collect();
      let is_script = run_matches.is_present("script");
      run_linter(paths, is_script);
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
}
