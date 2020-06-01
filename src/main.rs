// Copyright 2020 the Deno authors. All rights reserved. MIT license.
mod linter;
mod rules;
mod swc_util;

use linter::Linter;
use rules::get_all_rules;

mod report_util;

#[cfg(test)]
mod test_util;

fn main() {
  #[cfg(windows)]
  report_util::enable_ansi();

  let args: Vec<String> = std::env::args().collect();

  if args.len() < 2 {
    eprintln!("Missing file name");
    std::process::exit(1);
  }

  let file_names: Vec<String> = args[1..].to_vec();

  let mut error_counts = 0;
  for file_name in file_names {
    let source_code =
      std::fs::read_to_string(&file_name).expect("Failed to read file");

    let mut linter: Linter = Default::default();

    let rules = get_all_rules();

    let file_diagnostics = linter
      .lint(file_name, source_code, rules)
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
