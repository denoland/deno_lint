// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::colors;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
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

#[derive(Clone, Debug, Serialize)]
pub struct LintDiagnostic {
  pub location: Location,
  pub message: String,
  pub code: String,
  pub line_src: String,
  pub glyphes_length: usize,
}

impl LintDiagnostic {
  pub fn to_pretty_string(&self) -> String {
    let pretty_error =
      format!("({}) {}", colors::gray(self.code.to_string()), self.message);

    let file_name = &self.location.filename;
    let location = if file_name.contains('/')
      || file_name.contains('\\')
      || file_name.starts_with("./")
    {
      file_name.to_string()
    } else {
      format!("./{}", file_name)
    };

    let pretty_location = colors::cyan(format!(
      " --> {}:{}:{}",
      location, self.location.line, self.location.col
    ))
    .to_string();

    let line_str_len = self.location.line.to_string().len();
    let dummy = format!("{}|", " ".repeat(line_str_len + 1));
    let pretty_line_src = format!("{} | {}", self.location.line, self.line_src);
    let red_glyphs = format!(
      "{}|{}{}",
      " ".repeat(line_str_len + 1),
      " ".repeat(self.location.col + 1),
      colors::red("^".repeat(self.glyphes_length))
    );

    let lines = vec![
      pretty_error,
      pretty_location,
      dummy.clone(),
      pretty_line_src,
      red_glyphs,
      dummy,
    ];

    lines.join("\n")
  }
}
