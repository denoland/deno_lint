// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#[derive(Debug, Clone)]
pub struct Location {
  pub filename: String,
  pub line: usize,
  pub col: usize,
}

impl Into<Location> for swc_common::Loc {
  fn into(self) -> Location {
    use swc_common::FileName::*;

    let filename = match &self.file.name {
      Real(path_buf) => path_buf.to_string_lossy().to_string(),
      Custom(str_) => str_.to_string(),
      _ => panic!("invalid filename"),
    };

    Location {
      filename,
      line: self.line,
      col: self.col_display,
    }
  }
}

#[derive(Clone, Debug)]
pub struct LintDiagnostic {
  pub location: Location,
  pub message: String,
  pub code: String,
  pub line_src: String,
  pub snippet_length: usize,
}
