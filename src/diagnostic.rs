// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use serde::Serialize;
use serde::Serializer;

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
  /// The 0-indexed line index.
  #[serde(rename(serialize = "line"))]
  #[serde(serialize_with = "to_one_indexed")]
  pub line_index: usize,
  /// The 0-indexed column index.
  #[serde(rename(serialize = "col"))]
  pub column_index: usize,
  pub byte_pos: usize,
}

impl Position {
  pub fn new(
    byte_pos: swc_common::BytePos,
    loc: ast_view::LineAndColumnIndex,
  ) -> Self {
    Position {
      line_index: loc.line_index,
      column_index: loc.column_index,
      byte_pos: byte_pos.0 as usize,
    }
  }
}

fn to_one_indexed<S>(x: &usize, s: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    s.serialize_u32((x + 1) as u32)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Range {
  pub start: Position,
  pub end: Position,
}

#[derive(Clone, Debug, Serialize)]
pub struct LintDiagnostic {
  pub range: Range,
  pub filename: String,
  pub message: String,
  pub code: String,
  pub hint: Option<String>,
}
