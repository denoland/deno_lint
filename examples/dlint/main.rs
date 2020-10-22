// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use annotate_snippets::display_list;
use annotate_snippets::snippet;
use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::diagnostic::Range;
use deno_lint::linter::LinterBuilder;
use deno_lint::rules::get_recommended_rules;
use rayon::prelude::*;
use serde_json::json;
use serde_json::Value;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

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
        .arg(Arg::with_name("json").long("json")),
    )
    .subcommand(
      SubCommand::with_name("run").arg(
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
  let (last_line_no, _) = line_start_indexes[range.end.line - 1];
  let last_line_end = line_start_indexes[last_line_no + 1].1 - 1;
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

fn run_linter(paths: Vec<String>) {
  let error_counts = Arc::new(AtomicUsize::new(0));
  let output_lock = Arc::new(Mutex::new(())); // prevent threads outputting at the same time

  paths.par_iter().for_each(|file_path| {
    let source_code =
      std::fs::read_to_string(&file_path).expect("Failed to read file");

    let mut linter = LinterBuilder::default()
      .rules(get_recommended_rules())
      .build();
    let mut rt = js::create_js_runtime();
    let (parse_result, _) = linter
      .ast_parser
      .parse_module(&file_path, linter.syntax, &source_code);
    let module = parse_result.expect("Failed to parse ast");
   
    let file_diagnostics = linter
      .lint(file_path.to_string(), source_code.clone())
      .expect("Failed to lint");

    error_counts.fetch_add(file_diagnostics.len(), Ordering::Relaxed);
    let _g = output_lock.lock().unwrap();

    for diagnostic in file_diagnostics {
      display_diagnostic(&diagnostic, &source_code);
    }
    js::run_visitor(module, &mut rt);
  });

  let err_count = error_counts.load(Ordering::Relaxed);
  if err_count > 0 {
    eprintln!("Found {} problems", err_count);
    std::process::exit(1);
  }
}

fn print_rule_info_json(maybe_rule_name: Option<&str>) {
  let rules = get_recommended_rules();

  if maybe_rule_name.is_none() {
    let rules_json = rules
      .iter()
      .map(|r| {
        json!({
          "code": r.code(),
          "docs": r.docs(),
        })
      })
      .collect::<Vec<Value>>();

    let json_str = serde_json::to_string_pretty(&rules_json).unwrap();
    println!("{}", json_str);
    return;
  }

  let rule_name = maybe_rule_name.unwrap();
  let maybe_rule = rules.into_iter().find(|r| r.code() == rule_name);

  if let Some(rule) = maybe_rule {
    let rule_json = json!({
      "code": rule.code(),
      "docs": rule.docs(),
    });
    let json_str = serde_json::to_string_pretty(&rule_json).unwrap();
    println!("{}", json_str);
  } else {
    eprintln!("Rule not found!");
    std::process::exit(1);
  }
}

fn print_rule_info(maybe_rule_name: Option<&str>) {
  let rules = get_recommended_rules();

  if maybe_rule_name.is_none() {
    let mut rule_names = rules
      .iter()
      .map(|r| r.code())
      .map(|name| format!(" - {}", name))
      .collect::<Vec<String>>();

    rule_names.sort();
    rule_names.insert(0, "Available rules:".to_string());

    let rule_list = rule_names.join("\n");
    println!("{}", rule_list);
    return;
  }

  let rule_name = maybe_rule_name.unwrap();
  let maybe_rule = rules.into_iter().find(|r| r.code() == rule_name);

  if let Some(rule) = maybe_rule {
    println!("- {}", rule.code());
    println!();
    let mut docs = rule.docs();
    if docs.is_empty() {
      docs = "documentation not available"
    }
    println!("{}", docs);
  } else {
    eprintln!("Rule not found!");
    std::process::exit(1);
  }
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
      run_linter(paths);
    }
    ("rules", Some(rules_matches)) => {
      let json = rules_matches.is_present("json");
      let maybe_rule_name = rules_matches.value_of("RULE_NAME");
      if json {
        print_rule_info_json(maybe_rule_name);
      } else {
        print_rule_info(maybe_rule_name);
      }
    }
    _ => unreachable!(),
  };
}
