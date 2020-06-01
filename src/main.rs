// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use clap::App;
use clap::Arg;

mod linter;
mod rules;
mod swc_util;

use linter::Linter;
use rules::get_all_rules;

mod report_util;

#[cfg(test)]
mod test_util;

fn create_cli_app<'a, 'b>() -> App<'a, 'b> {
  App::new("deno lint").arg(
    Arg::with_name("FILES")
      .help("Sets the input file to use")
      .required(true)
      .multiple(true),
  )
}

fn main() {
  #[cfg(windows)]
  report_util::enable_ansi();

  let cli_app = create_cli_app();

  let matches = cli_app.get_matches();

  let file_names = matches.values_of("FILES").unwrap();

  let mut error_counts = 0;
  for file_name in file_names {
    let source_code =
      std::fs::read_to_string(&file_name).expect("Failed to read file");

    let mut linter = Linter::default();

    let rules = get_all_rules();

    let file_diagnostics = linter
      .lint(file_name.to_string(), source_code, rules)
      .expect("Failed to lint");

    error_counts += file_diagnostics.len();
    if !file_diagnostics.is_empty() {
      for d in file_diagnostics.iter() {
        report_util::report_error(&d.code, &d.message);
        report_util::report_location(
          &file_diagnostics[0].location.filename,
          d.location.line,
          d.location.col,
        );
        report_util::report_line_src(d.location.line, &d.line_src);
        report_util::place_glyphes(
          d.location.line,
          d.location.col,
          d.snippet_length,
        );
      }
    }
  }
  eprintln!("Found {} problems", error_counts);
}
