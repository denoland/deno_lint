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
    eprintln!(
      "{}: line {}, col {}, Error - {} ({})",
      diagnostic.specifier,
      diagnostic.range.start.line_index + 1,
      diagnostic.range.start.column_index + 1,
      diagnostic.message,
      diagnostic.code
    )
  }
}

fn print_pretty(diagnostics: &[LintDiagnostic]) {
  for diagnostic in diagnostics {
    eprintln!("{}", diagnostic.display());
  }
}
