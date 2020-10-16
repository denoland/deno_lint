// Copyright 2020 the Deno authors. All rights reserved. MIT license.
#[cfg(feature = "json")]
use serde::Serialize;
use std::convert::TryInto;

#[derive(Debug, Clone, PartialEq, Copy)]
#[cfg_attr(feature = "json", derive(Serialize))]
pub struct Position {
  pub line: usize,
  pub col: usize,
  pub byte_pos: usize,
}

impl Position {
  pub fn new(byte_pos: swc_common::BytePos, loc: swc_common::Loc) -> Self {
    Position {
      line: loc.line,
      // Using loc.col instead of loc.col_display
      // because it leads to out-of-bounds columns if file
      // contains non-narrow chars (like tabs).
      // See: https://github.com/denoland/deno_lint/issues/139
      col: loc.col.0,
      byte_pos: byte_pos.0.try_into().expect("Failed to convert byte_pos"),
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
  pub hint: Option<String>,
}
