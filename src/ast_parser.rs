// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use deno_ast::swc::common::Globals;
use deno_ast::swc::common::Mark;
use deno_ast::swc::parser::Syntax;
use deno_ast::swc::transforms::resolver::ts_resolver;
use deno_ast::swc::visit::FoldWith;
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

/// Low-level utility structure with common AST parsing functions.
///
/// Allows to build more complicated parser by providing a callback
/// to `parse_module`.
pub(crate) struct AstParser {
  pub(crate) globals: Globals,
  /// The marker passed to the resolver (from swc).
  ///
  /// This mark is applied to top level bindings and unresolved references.
  pub(crate) top_level_mark: Mark,
}

impl AstParser {
  pub(crate) fn new() -> Self {
    let globals = Globals::new();
    let top_level_mark = deno_ast::swc::common::GLOBALS
      .set(&globals, || Mark::fresh(Mark::root()));

    AstParser {
      globals,
      top_level_mark,
    }
  }

  pub(crate) fn parse_program(
    &self,
    file_name: &str,
    syntax: Syntax,
    source_code: String,
  ) -> Result<ParsedSource, SwcDiagnostic> {
    deno_ast::parse_program_with_post_process(
      deno_ast::ParseParams {
        specifier: file_name.to_string(),
        media_type: MediaType::Unknown,
        source: deno_ast::SourceTextInfo::from_string(source_code),
        capture_tokens: true,
        maybe_syntax: Some(syntax),
      },
      |program| {
        // This is used to apply proper "syntax context" to all AST elements. When SWC performs
        // transforms/folding it might change some of those context and "ts_resolver" ensures
        // that all elements end up in proper lexical scope.
        deno_ast::swc::common::GLOBALS.set(&self.globals, || {
          program.fold_with(&mut ts_resolver(self.top_level_mark))
        })
      },
    )
    .map_err(|diagnostic| SwcDiagnostic::from_diagnostic(&diagnostic))
  }
}

impl Default for AstParser {
  fn default() -> Self {
    Self::new()
  }
}
