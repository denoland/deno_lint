// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::borrow::Cow;

use deno_ast::diagnostics::Diagnostic;
use deno_ast::diagnostics::DiagnosticLevel;
use deno_ast::diagnostics::DiagnosticLocation;
use deno_ast::diagnostics::DiagnosticSnippet;
use deno_ast::diagnostics::DiagnosticSnippetHighlight;
use deno_ast::diagnostics::DiagnosticSnippetHighlightStyle;
use deno_ast::diagnostics::DiagnosticSourcePos;
use deno_ast::diagnostics::DiagnosticSourceRange;
use deno_ast::ModuleSpecifier;
use deno_ast::SourceRange;
use deno_ast::SourceTextInfo;

#[derive(Debug, Clone)]
pub struct LintFixChange {
  pub new_text: Cow<'static, str>,
  pub range: SourceRange,
}

#[derive(Debug, Clone)]
pub struct LintFix {
  pub description: Cow<'static, str>,
  pub changes: Vec<LintFixChange>,
}

#[derive(Clone)]
pub struct LintDiagnostic {
  pub specifier: ModuleSpecifier,
  pub range: SourceRange,
  pub text_info: SourceTextInfo,
  pub message: String,
  pub code: String,
  pub hint: Option<String>,
  /// Fixes that should be shown in the Deno LSP and also
  /// used for the `deno lint --fix` flag.
  ///
  /// Note: If there are multiple fixes for a diagnostic then
  /// only the first fix will be used for the `--fix` flag, but
  /// multiple will be shown in the LSP.
  pub fixes: Vec<LintFix>,
  /// URL to the lint rule documentation. By default, the url uses the
  /// code to link to lint.deno.land
  pub custom_docs_url: Option<String>,
  /// Additional information displayed beside the highlighted range.
  pub range_description: Option<String>,
  /// Displays additional information at the end of a diagnostic.
  pub info: Vec<Cow<'static, str>>
}

impl Diagnostic for LintDiagnostic {
  fn level(&self) -> DiagnosticLevel {
    DiagnosticLevel::Error
  }

  fn code(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.code)
  }

  fn message(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.message)
  }

  fn location(&self) -> DiagnosticLocation {
    DiagnosticLocation::ModulePosition {
      specifier: Cow::Borrowed(&self.specifier),
      text_info: Cow::Borrowed(&self.text_info),
      source_pos: DiagnosticSourcePos::SourcePos(self.range.start),
    }
  }

  fn snippet(&self) -> Option<DiagnosticSnippet<'_>> {
    let range = DiagnosticSourceRange {
      start: DiagnosticSourcePos::SourcePos(self.range.start),
      end: DiagnosticSourcePos::SourcePos(self.range.end),
    };
    Some(DiagnosticSnippet {
      source: Cow::Borrowed(&self.text_info),
      highlights: vec![DiagnosticSnippetHighlight {
        range,
        style: DiagnosticSnippetHighlightStyle::Error,
        description: self.range_description.as_deref().map(Cow::Borrowed),
      }],
    })
  }

  fn hint(&self) -> Option<Cow<'_, str>> {
    self.hint.as_ref().map(|s| Cow::Borrowed(s.as_str()))
  }

  fn snippet_fixed(&self) -> Option<DiagnosticSnippet<'_>> {
    None // todo
  }

  fn info(&self) -> Cow<'_, [std::borrow::Cow<'_, str>]> {
    Cow::Borrowed(&self.info)
  }

  fn docs_url(&self) -> Option<Cow<'_, str>> {
    if let Some(custom_docs_url) = &self.custom_docs_url {
      Some(Cow::Borrowed(custom_docs_url))
    } else {
      Some(Cow::Owned(format!(
        "https://lint.deno.land/rules/{}",
        &self.code
      )))
    }
  }
}
