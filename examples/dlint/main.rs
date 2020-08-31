// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use clap::App;
use clap::Arg;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::linter::LinterBuilder;
use deno_lint::rules::get_recommended_rules;
use rayon::prelude::*;
use std::fmt;
use std::io::Write;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use termcolor::Color::{Ansi256, Red};
use termcolor::{Ansi, ColorSpec, WriteColor};

#[cfg(windows)]
use termcolor::{BufferWriter, ColorChoice};

#[allow(unused)]
#[cfg(windows)]
fn enable_ansi() {
  BufferWriter::stdout(ColorChoice::AlwaysAnsi);
}

fn gray(s: String) -> impl fmt::Display {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Ansi256(8)));
  style(&s, style_spec)
}

fn red(s: String) -> impl fmt::Display {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Red));
  style(&s, style_spec)
}

fn bold(s: String) -> impl fmt::Display {
  let mut style_spec = ColorSpec::new();
  style_spec.set_bold(true);
  style(&s, style_spec)
}

fn cyan(s: String) -> impl fmt::Display {
  let mut style_spec = ColorSpec::new();
  style_spec.set_fg(Some(Ansi256(14)));
  style(&s, style_spec)
}

fn style(s: &str, colorspec: ColorSpec) -> impl fmt::Display {
  let mut v = Vec::new();
  let mut ansi_writer = Ansi::new(&mut v);
  ansi_writer.set_color(&colorspec).unwrap();
  ansi_writer.write_all(s.as_bytes()).unwrap();
  ansi_writer.reset().unwrap();
  String::from_utf8_lossy(&v).into_owned()
}

fn create_cli_app<'a, 'b>(rule_list: &'b str) -> App<'a, 'b> {
  App::new("dlint").after_help(rule_list).arg(
    Arg::with_name("FILES")
      .help("Sets the input file to use")
      .required(true)
      .multiple(true),
  )
}

pub fn format_diagnostic(diagnostic: &LintDiagnostic, source: &str) -> String {
  let pretty_error = format!(
    "({}) {}",
    gray(diagnostic.code.to_string()),
    diagnostic.message
  );

  let file_name = &diagnostic.filename;
  let location = if file_name.contains('/')
    || file_name.contains('\\')
    || file_name.starts_with("./")
  {
    file_name.to_string()
  } else {
    format!("./{}", file_name)
  };

  let line_str_len = diagnostic.range.end.line.to_string().len();
  let pretty_location = cyan(format!(
    "{}--> {}:{}:{}",
    " ".repeat(line_str_len),
    location,
    diagnostic.range.start.line,
    diagnostic.range.start.col
  ))
  .to_string();

  let dummy = format!("{} |", " ".repeat(line_str_len));

  if diagnostic.range.start.line == diagnostic.range.end.line {
    let snippet_length = diagnostic.range.end.col - diagnostic.range.start.col;
    let source_lines: Vec<&str> = source.split('\n').collect();
    let line = source_lines[diagnostic.range.start.line - 1];
    let pretty_line_src = format!("{} | {}", diagnostic.range.start.line, line);
    let red_glyphs = format!(
      "{} | {}{}",
      " ".repeat(line_str_len),
      " ".repeat(diagnostic.range.start.col),
      red("^".repeat(snippet_length))
    );

    let lines = vec![
      pretty_error,
      pretty_location,
      dummy.clone(),
      pretty_line_src,
      red_glyphs,
      dummy,
    ];

    lines.join("\n")
  } else {
    let mut lines = vec![pretty_error, pretty_location, dummy.clone()];
    let source_lines: Vec<&str> = source.split('\n').collect();

    for i in diagnostic.range.start.line..(diagnostic.range.end.line + 1) {
      let line = source_lines[i - 1];
      let is_first = i == diagnostic.range.start.line;
      let is_last = i == diagnostic.range.end.line;

      if is_first {
        let (rest, snippet) = line.split_at(diagnostic.range.start.col);
        lines.push(format!("{} |   {}{}", i, rest, bold(snippet.to_string())));
      } else if is_last {
        let (snippet, rest) = line.split_at(diagnostic.range.end.col);
        lines.push(format!(
          "{} | {} {}{}",
          i,
          red("|".to_string()),
          bold(snippet.to_string()),
          rest
        ));
      } else {
        lines.push(format!(
          "{} | {} {}",
          i,
          red("|".to_string()),
          bold(line.to_string())
        ));
      }

      // If this is the first line, render the ∨ symbols
      if is_first {
        lines.push(format!(
          "{} |  {}{}",
          " ".repeat(line_str_len),
          red("_".repeat(diagnostic.range.start.col + 1)),
          red("^".to_string())
        ));
      }

      // If this is the last line, render the ∨ symbols
      if is_last {
        lines.push(format!(
          "{} | {}{}{}",
          " ".repeat(line_str_len),
          red("|".to_string()),
          red("_".repeat(diagnostic.range.end.col)),
          red("^".to_string())
        ));
      }
    }

    lines.push(dummy);

    lines.join("\n")
  }
}

fn main() {
  #[cfg(windows)]
  enable_ansi();

  env_logger::init();

  let rules = get_recommended_rules();

  let mut rule_names = rules
    .iter()
    .map(|r| r.code())
    .map(|name| format!(" - {}", name))
    .collect::<Vec<String>>();

  rule_names.sort();
  rule_names.insert(0, "Available rules:".to_string());

  let rule_list = rule_names.join("\n");
  let cli_app = create_cli_app(&rule_list);
  let matches = cli_app.get_matches();
  let paths: Vec<String> = matches
    .values_of("FILES")
    .unwrap()
    .map(|p| p.to_string())
    .collect();

  let error_counts = Arc::new(AtomicUsize::new(0));
  let output_lock = Arc::new(Mutex::new(())); // prevent threads outputting at the same time

  paths.par_iter().for_each(|file_path| {
    let source_code =
      std::fs::read_to_string(&file_path).expect("Failed to read file");

    let mut linter = LinterBuilder::default()
      .rules(get_recommended_rules())
      .build();

    let file_diagnostics = linter
      .lint(file_path.to_string(), source_code.clone())
      .expect("Failed to lint");

    error_counts.fetch_add(file_diagnostics.len(), Ordering::Relaxed);
    let _g = output_lock.lock().unwrap();
    for d in file_diagnostics.iter() {
      eprintln!("{}", format_diagnostic(d, &source_code));
    }
  });

  let err_count = error_counts.load(Ordering::Relaxed);
  if err_count > 0 {
    eprintln!("Found {} problems", err_count);
    std::process::exit(1);
  }
}
