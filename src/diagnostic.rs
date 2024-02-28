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
pub struct LintQuickFixChange {
  pub new_text: String,
  pub range: SourceRange,
}

#[derive(Debug, Clone)]
pub struct LintQuickFix {
  pub description: String,
  pub changes: Vec<LintQuickFixChange>,
}

#[derive(Clone)]
pub struct LintDiagnostic {
  pub specifier: ModuleSpecifier,
  pub range: SourceRange,
  pub text_info: SourceTextInfo,
  pub message: String,
  pub code: String,
  pub hint: Option<String>,
  pub quick_fixes: Vec<LintQuickFix>,
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
      highlight: DiagnosticSnippetHighlight {
        range,
        style: DiagnosticSnippetHighlightStyle::Error,
        description: None,
      },
    })
  }

  fn hint(&self) -> Option<Cow<'_, str>> {
    self.hint.as_ref().map(|s| Cow::Borrowed(s.as_str()))
  }

  fn snippet_fixed(&self) -> Option<DiagnosticSnippet<'_>> {
    None // todo
  }

  fn info(&self) -> Cow<'_, [std::borrow::Cow<'_, str>]> {
    Cow::Borrowed(&[])
  }

  fn docs_url(&self) -> Option<Cow<'_, str>> {
    Some(Cow::Owned(format!(
      "https://lint.deno.land/#{}",
      &self.code
    )))
  }
}
