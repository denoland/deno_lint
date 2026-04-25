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
use serde::Deserialize;
use serde::Serialize;

/// Severity for a [`LintDiagnostic`].
///
/// `Error` is the default to preserve backwards-compatible behavior with
/// pre-severity versions of `deno_lint` where every diagnostic was treated
/// as an error.
#[derive(
  Debug, Default, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize,
)]
#[serde(rename_all = "lowercase")]
pub enum LintDiagnosticSeverity {
  #[default]
  Error,
  Warning,
}

impl LintDiagnosticSeverity {
  /// Returns the canonical lowercase string label for the severity
  /// (`"error"` or `"warning"`). Useful for compact text formatters.
  pub fn as_str(&self) -> &'static str {
    match self {
      LintDiagnosticSeverity::Error => "error",
      LintDiagnosticSeverity::Warning => "warning",
    }
  }
}

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
  /// Severity for this diagnostic. Defaults to [`LintDiagnosticSeverity::Error`]
  /// so existing rules that don't opt in keep their current "error" behavior.
  pub severity: LintDiagnosticSeverity,
}

impl Diagnostic for LintDiagnostic {
  fn level(&self) -> DiagnosticLevel {
    match self.severity {
      LintDiagnosticSeverity::Error => DiagnosticLevel::Error,
      LintDiagnosticSeverity::Warning => DiagnosticLevel::Warning,
    }
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
    let style = match self.severity {
      LintDiagnosticSeverity::Error => DiagnosticSnippetHighlightStyle::Error,
      LintDiagnosticSeverity::Warning => {
        DiagnosticSnippetHighlightStyle::Warning
      }
    };
    Some(DiagnosticSnippet {
      source: Cow::Borrowed(&range.text_info),
      highlights: vec![DiagnosticSnippetHighlight {
        range: DiagnosticSourceRange {
          start: DiagnosticSourcePos::SourcePos(range.range.start),
          end: DiagnosticSourcePos::SourcePos(range.range.end),
        },
        style,
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn severity_default_is_error() {
    assert_eq!(LintDiagnosticSeverity::default(), LintDiagnosticSeverity::Error);
  }

  #[test]
  fn severity_as_str_labels() {
    assert_eq!(LintDiagnosticSeverity::Error.as_str(), "error");
    assert_eq!(LintDiagnosticSeverity::Warning.as_str(), "warning");
  }

  #[test]
  fn severity_serializes_to_lowercase_json() {
    let json = serde_json::to_string(&LintDiagnosticSeverity::Warning).unwrap();
    assert_eq!(json, "\"warning\"");
    let json = serde_json::to_string(&LintDiagnosticSeverity::Error).unwrap();
    assert_eq!(json, "\"error\"");
  }

  #[test]
  fn severity_round_trips_through_serde() {
    let s: LintDiagnosticSeverity =
      serde_json::from_str("\"warning\"").unwrap();
    assert_eq!(s, LintDiagnosticSeverity::Warning);
    let s: LintDiagnosticSeverity = serde_json::from_str("\"error\"").unwrap();
    assert_eq!(s, LintDiagnosticSeverity::Error);
  }

  #[test]
  fn level_maps_severity_to_diagnostic_level() {
    let specifier = ModuleSpecifier::parse("file:///main.ts").unwrap();
    let mk = |severity| LintDiagnostic {
      specifier: specifier.clone(),
      range: None,
      details: LintDiagnosticDetails {
        message: "msg".to_string(),
        code: "code".to_string(),
        hint: None,
        fixes: vec![],
        custom_docs_url: LintDocsUrl::Default,
        info: vec![],
      },
      severity,
    };
    assert!(matches!(
      mk(LintDiagnosticSeverity::Error).level(),
      DiagnosticLevel::Error
    ));
    assert!(matches!(
      mk(LintDiagnosticSeverity::Warning).level(),
      DiagnosticLevel::Warning
    ));
  }
}
