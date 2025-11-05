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
pub struct LintDiagnosticRange {
  pub text_info: SourceTextInfo,
  pub range: SourceRange,
  /// Additional information displayed beside the highlighted range.
  pub description: Option<String>,
}

#[derive(Clone, Default)]
pub enum LintDocsUrl {
  #[default]
  Default,
  None,
  Custom(String),
}

#[derive(Clone)]
pub struct LintDiagnosticDetails {
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
  pub custom_docs_url: LintDocsUrl,
  /// Displays additional information at the end of a diagnostic.
  pub info: Vec<Cow<'static, str>>,
}

#[derive(Clone)]
pub struct LintDiagnostic {
  pub specifier: ModuleSpecifier,
  /// Optional range within the file.
  ///
  /// Diagnostics that don't have a range mean there's something wrong with
  /// the whole file.
  pub range: Option<LintDiagnosticRange>,
  pub details: LintDiagnosticDetails,
}

impl Diagnostic for LintDiagnostic {
  fn level(&self) -> DiagnosticLevel {
    DiagnosticLevel::Error
  }

  fn code(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.details.code)
  }

  fn message(&self) -> Cow<'_, str> {
    Cow::Borrowed(&self.details.message)
  }

  fn location(&self) -> DiagnosticLocation<'_> {
    match &self.range {
      Some(range) => DiagnosticLocation::ModulePosition {
        specifier: Cow::Borrowed(&self.specifier),
        text_info: Cow::Borrowed(&range.text_info),
        source_pos: DiagnosticSourcePos::SourcePos(range.range.start),
      },
      None => DiagnosticLocation::Module {
        specifier: Cow::Borrowed(&self.specifier),
      },
    }
  }

  fn snippet(&self) -> Option<DiagnosticSnippet<'_>> {
    let range = self.range.as_ref()?;
    Some(DiagnosticSnippet {
      source: Cow::Borrowed(&range.text_info),
      highlights: vec![DiagnosticSnippetHighlight {
        range: DiagnosticSourceRange {
          start: DiagnosticSourcePos::SourcePos(range.range.start),
          end: DiagnosticSourcePos::SourcePos(range.range.end),
        },
        style: DiagnosticSnippetHighlightStyle::Error,
        description: range.description.as_deref().map(Cow::Borrowed),
      }],
    })
  }

  fn hint(&self) -> Option<Cow<'_, str>> {
    self
      .details
      .hint
      .as_ref()
      .map(|s| Cow::Borrowed(s.as_str()))
  }

  fn snippet_fixed(&self) -> Option<DiagnosticSnippet<'_>> {
    None // todo
  }

  fn info(&self) -> Cow<'_, [std::borrow::Cow<'_, str>]> {
    Cow::Borrowed(&self.details.info)
  }

  fn docs_url(&self) -> Option<Cow<'_, str>> {
    match &self.details.custom_docs_url {
      LintDocsUrl::Default => Some(Cow::Owned(format!(
        "https://docs.deno.com/lint/rules/{}",
        &self.details.code
      ))),
      LintDocsUrl::Custom(url) => Some(Cow::Borrowed(url)),
      LintDocsUrl::None => None,
    }
  }
}
