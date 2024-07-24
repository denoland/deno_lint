// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::diagnostics::Diagnostic;
use deno_lint::diagnostic::LintDiagnostic;

pub fn display_diagnostics(
  diagnostics: &[LintDiagnostic],
  format: Option<&str>,
) {
  match format {
    Some("compact") => print_compact(diagnostics),
    Some("pretty") => print_pretty(diagnostics),
    _ => unreachable!("Invalid output format specified"),
  }
}

fn print_compact(diagnostics: &[LintDiagnostic]) {
  for diagnostic in diagnostics {
    match &diagnostic.range {
      Some(range) => {
        let display_index =
          range.text_info.line_and_column_display(range.range.start);
        eprintln!(
          "{}: line {}, col {}, Error - {} ({})",
          diagnostic.specifier,
          display_index.line_number,
          display_index.column_number,
          diagnostic.details.message,
          diagnostic.details.code
        )
      }
      None => {
        eprintln!(
          "{}: {} ({})",
          diagnostic.specifier,
          diagnostic.message(),
          diagnostic.code()
        )
      }
    }
  }
}

fn print_pretty(diagnostics: &[LintDiagnostic]) {
  for diagnostic in diagnostics {
    eprintln!("{}\n", diagnostic.display());
  }
}
