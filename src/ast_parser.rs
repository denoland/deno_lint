// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use deno_ast::swc::parser::Syntax;
use deno_ast::MediaType;
use deno_ast::ParsedSource;
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug)]
pub struct SwcDiagnostic {
  pub filename: String,
  pub line_display: usize,
  pub column_display: usize,
  pub message: String,
}

impl Error for SwcDiagnostic {}

impl fmt::Display for SwcDiagnostic {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&format!(
      "{} at {}:{}:{}",
      self.message, self.filename, self.line_display, self.column_display
    ))
  }
}

impl SwcDiagnostic {
  pub(crate) fn from_diagnostic(diagnostic: &deno_ast::Diagnostic) -> Self {
    SwcDiagnostic {
      line_display: diagnostic.display_position.line_number,
      column_display: diagnostic.display_position.column_number,
      filename: diagnostic.specifier.clone(),
      message: diagnostic.message.clone(),
    }
  }
}

pub(crate) fn parse_program(
  file_name: &str,
  syntax: Syntax,
  source_code: String,
) -> Result<ParsedSource, SwcDiagnostic> {
  deno_ast::parse_program(deno_ast::ParseParams {
    specifier: file_name.to_string(),
    media_type: MediaType::Unknown,
    source: deno_ast::SourceTextInfo::from_string(source_code),
    capture_tokens: true,
    maybe_syntax: Some(syntax),
    scope_analysis: true,
  })
  .map_err(|diagnostic| SwcDiagnostic::from_diagnostic(&diagnostic))
}
