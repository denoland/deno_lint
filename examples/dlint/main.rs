// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use clap::App;
use clap::Arg;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::linter::LinterBuilder;
use deno_lint::rules::get_recommended_rules;
use deno_lint::swc_util::get_default_ts_config;
use std::fmt;
use std::io::Write;
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

pub fn format_diagnostic(diagnostic: &LintDiagnostic) -> String {
  let pretty_error = format!(
    "({}) {}",
    gray(diagnostic.code.to_string()),
    diagnostic.message
  );

  let file_name = &diagnostic.location.filename;
  let location = if file_name.contains('/')
    || file_name.contains('\\')
    || file_name.starts_with("./")
  {
    file_name.to_string()
  } else {
    format!("./{}", file_name)
  };

  let line_str_len = diagnostic.location.line.to_string().len();
  let pretty_location = cyan(format!(
    "{}--> {}:{}:{}",
    " ".repeat(line_str_len),
    location,
    diagnostic.location.line,
    diagnostic.location.col
  ))
  .to_string();

  let dummy = format!("{} |", " ".repeat(line_str_len));
  let pretty_line_src =
    format!("{} | {}", diagnostic.location.line, diagnostic.line_src);
  let red_glyphs = format!(
    "{} | {}{}",
    " ".repeat(line_str_len),
    " ".repeat(diagnostic.location.col),
    red("^".repeat(diagnostic.snippet_length))
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
}

fn main() {
  #[cfg(windows)]
  enable_ansi();

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
  let file_names = matches.values_of("FILES").unwrap();

  let mut error_counts = 0;

  for file_name in file_names {
    let source_code =
      std::fs::read_to_string(&file_name).expect("Failed to read file");

    let mut linter = LinterBuilder::default().build();

    let rules = get_recommended_rules();
    let syntax = get_default_ts_config();

    let file_diagnostics = linter
      .lint(file_name.to_string(), source_code, syntax, rules)
      .expect("Failed to lint");

    error_counts += file_diagnostics.len();
    for d in file_diagnostics.iter() {
      eprintln!("{}", format_diagnostic(d));
    }
  }

  if error_counts > 0 {
    eprintln!("Found {} problems", error_counts);
  }
}
