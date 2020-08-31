// Copyright 2020 the Deno authors. All rights reserved. MIT license.
#[cfg(feature = "json")]
use serde::Serialize;

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Position {
  pub line: usize,
  pub col: usize,
}

impl Into<Position> for swc_common::Loc {
  fn into(self) -> Position {
    Position {
      line: self.line,
      // Using self.col instead of self.col_display
      // because it leads to out-of-bounds columns if file
      // contains non-narrow chars (like tabs).
      // See: https://github.com/denoland/deno_lint/issues/139
      col: self.col.0,
    }
  }
}

#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Range {
  pub start: Position,
  pub end: Position,
}

#[derive(Clone, Debug)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct LintDiagnostic {
  pub range: Range,
  pub filename: String,
  pub message: String,
  pub code: String,
}
