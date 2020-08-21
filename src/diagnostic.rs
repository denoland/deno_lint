// Copyright 2020 the Deno authors. All rights reserved. MIT license.
#[cfg(feature = "json")]
use serde::Serialize;

// TODO(bartlomieju): find a better name
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Location {
  // pub filename: String,
  pub line: usize,
  pub col: usize,
}

impl Into<Location> for swc_common::Loc {
  fn into(self) -> Location {
    Location {
      line: self.line,
      // Using self.col instead of self.col_display
      // because it leads to out-of-bounds columns if file
      // contains non-narrow chars (like tabs).
      // See: https://github.com/denoland/deno_lint/issues/139
      col: self.col.0,
    }
  }
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct LintDiagnostic {
  // TODO(bartlomieju): store range of `Location`s
  // it may be multiline, reporters must keep that in mind
  pub location: Location,
  pub filename: String,
  pub message: String,
  pub code: String,
  // TODO(bartlomieju): remove this field, reporters
  // should look it up
  pub line_src: String,
  // TODO(bartlomieju): remove this field
  pub snippet_length: usize,
}
