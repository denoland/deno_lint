// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;
use deno_ast::{diagnostics::Diagnostic, SourceTextInfo};
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

pub(crate) fn apply_lint_fixes(
  text_info: &SourceTextInfo,
  diagnostics: &[LintDiagnostic],
) -> Option<String> {
  if diagnostics.is_empty() {
    return None;
  }

  let file_start = text_info.range().start;
  let mut quick_fixes = diagnostics
    .iter()
    // use the first quick fix
    .filter_map(|d| d.details.fixes.first())
    .flat_map(|fix| fix.changes.iter())
    .map(|change| deno_ast::TextChange {
      range: change.range.as_byte_range(file_start),
      new_text: change.new_text.to_string(),
    })
    .collect::<Vec<_>>();
  if quick_fixes.is_empty() {
    return None;
  }

  let mut import_fixes = HashSet::new();
  // remove any overlapping text changes, we'll circle
  // back for another pass to fix the remaining
  quick_fixes.sort_by_key(|change| change.range.start);
  for i in (1..quick_fixes.len()).rev() {
    let cur = &quick_fixes[i];
    let previous = &quick_fixes[i - 1];
    // hack: deduplicate import fixes to avoid creating errors
    if previous.new_text.trim_start().starts_with("import ") {
      import_fixes.insert(previous.new_text.trim().to_string());
    }
    let is_overlapping = cur.range.start <= previous.range.end;
    if is_overlapping
      || (cur.new_text.trim_start().starts_with("import ")
        && import_fixes.contains(cur.new_text.trim()))
    {
      quick_fixes.remove(i);
    }
  }
  let new_text =
    deno_ast::apply_text_changes(text_info.text_str(), quick_fixes);
  Some(new_text)
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
