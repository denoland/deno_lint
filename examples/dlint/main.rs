// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use annotate_snippets::display_list;
use annotate_snippets::snippet;
use anyhow::bail;
use anyhow::Error as AnyError;
use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use deno_lint::ast_parser::{get_default_es_config, get_default_ts_config};
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::diagnostic::Range;
use deno_lint::linter::LinterBuilder;
use deno_lint::rules::{get_all_rules, get_recommended_rules};
use log::debug;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use swc_common::BytePos;
use swc_ecmascript::parser::{EsConfig, Syntax, TsConfig};

mod color;
mod config;
mod js;
mod lexer;
mod rules;

fn create_cli_app<'a, 'b>() -> App<'a, 'b> {
  App::new("dlint")
    .version(clap::crate_version!())
    .setting(AppSettings::SubcommandRequiredElseHelp)
    .subcommand(
      SubCommand::with_name("rules")
        .arg(
          Arg::with_name("RULE_NAME")
            .help("Show detailed information about rule. If omitted, show the list of all rules."),
        )
        .arg(Arg::with_name("json").long("json")),
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
  source_code: &str,
  lines: &[BytePos],
) {
  let line_start_indexes = lines
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
        source: slice_source,
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

  struct FileDiagnostics {
    source_code: String,
    lines: Vec<BytePos>,
    diagnostics: Vec<LintDiagnostic>,
  }

  let file_diagnostics = Arc::new(Mutex::new(BTreeMap::new()));
  paths
    .par_iter()
    .try_for_each(|file_path| -> Result<(), AnyError> {
      let source_code = std::fs::read_to_string(&file_path)?;

      let rules = if let Some(config) = maybe_config.clone() {
        config.get_rules()
      } else if let Some(rule_name) = filter_rule_name {
        get_all_rules()
          .into_iter()
          .filter(|r| r.code() == rule_name)
          .collect()
      } else {
        get_recommended_rules()
      };

      debug!("Configured rules: {}", rules.len());

      if rules.is_empty() {
        bail!("There's no rule to be run!");
      }

      let mut linter_builder = LinterBuilder::default()
        .rules(rules)
        .syntax(determine_syntax(file_path));

      for plugin_path in &plugin_paths {
        let js_runner = js::JsRuleRunner::new(plugin_path);
        linter_builder = linter_builder.add_plugin(js_runner);
      }

      let linter = linter_builder.build();

      let (source_file, diagnostics) =
        linter.lint(file_path.to_string_lossy().to_string(), source_code)?;

      error_counts.fetch_add(diagnostics.len(), Ordering::Relaxed);

      let mut lock = file_diagnostics.lock().unwrap();

      lock.insert(
        file_path,
        FileDiagnostics {
          diagnostics,
          lines: source_file.lines.clone(),
          source_code: source_file.src.to_string(),
        },
      );

      Ok(())
    })?;

  for d in file_diagnostics.lock().unwrap().values() {
    display_diagnostics(&d.diagnostics, &d.source_code, &d.lines);
  }

  let err_count = error_counts.load(Ordering::Relaxed);
  if err_count > 0 {
    eprintln!("Found {} problems", err_count);
    std::process::exit(1);
  }

  Ok(())
}

/// Determine what syntax should be used as parse config from the file path.
fn determine_syntax(path: &Path) -> Syntax {
  match path.extension() {
    Some(os_str) => match os_str.to_str() {
      Some("ts") => get_default_ts_config(),
      Some("js") | Some("mjs") | Some("cjs") => get_default_es_config(),
      Some("tsx") => Syntax::Typescript(TsConfig {
        tsx: true,
        dynamic_import: true,
        decorators: true,
        ..Default::default()
      }),
      Some("jsx") => Syntax::Es(EsConfig {
        jsx: true,
        num_sep: true,
        class_private_props: false,
        class_private_methods: false,
        class_props: false,
        export_default_from: true,
        export_namespace_from: true,
        dynamic_import: true,
        nullish_coalescing: true,
        optional_chaining: true,
        import_meta: true,
        top_level_await: true,
        ..Default::default()
      }),
      _ => get_default_ts_config(),
    },
    None => get_default_ts_config(),
  }
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
      let rules = if let Some(rule_name) = rules_matches.value_of("RULE_NAME") {
        rules::get_specific_rule_metadata(rule_name)
      } else {
        rules::get_all_rules_metadata()
      };
      if rules_matches.is_present("json") {
        rules::print_rules::<rules::JsonFormatter>(rules);
      } else {
        rules::print_rules::<rules::PrettyFormatter>(rules);
      }
    }
    _ => unreachable!(),
  };

  Ok(())
}
