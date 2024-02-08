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
use deno_ast::SourceTextInfo;
use serde::Serialize;
use serde::Serializer;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Position {
  /// The 0-indexed line index.
  #[serde(rename(serialize = "line"))]
  #[serde(serialize_with = "to_one_indexed")]
  pub line_index: usize,
  /// The 0-indexed column index.
  #[serde(rename(serialize = "col"))]
  pub column_index: usize,
  #[serde(rename(serialize = "bytePos"))]
  pub byte_index: usize,
}

impl Position {
  pub fn new(byte_index: usize, loc: deno_ast::LineAndColumnIndex) -> Self {
    Position {
      line_index: loc.line_index,
      column_index: loc.column_index,
      byte_index,
    }
  }
}

fn to_one_indexed<S>(x: &usize, s: S) -> Result<S::Ok, S::Error>
where
  S: Serializer,
{
  s.serialize_u32((x + 1) as u32)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Range {
  pub start: Position,
  pub end: Position,
}

#[derive(Clone, Serialize)]
pub struct LintDiagnostic {
  pub specifier: ModuleSpecifier,
  pub range: Range,
  #[serde(skip)]
  pub text_info: SourceTextInfo,
  pub message: String,
  pub code: String,
  pub hint: Option<String>,
}

impl std::fmt::Debug for LintDiagnostic {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("LintDiagnostic")
      .field("specifier", &self.specifier)
      .field("range", &self.range)
      .field("text_info", &"<omitted>")
      .field("message", &self.message)
      .field("code", &self.code)
      .field("hint", &self.hint)
      .finish()
  }
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
      source_pos: DiagnosticSourcePos::ByteIndex(self.range.start.byte_index),
    }
  }

  fn snippet(&self) -> Option<DiagnosticSnippet<'_>> {
    let range = DiagnosticSourceRange {
      start: DiagnosticSourcePos::ByteIndex(self.range.start.byte_index),
      end: DiagnosticSourcePos::ByteIndex(self.range.end.byte_index),
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
    Some(Cow::Owned(format!("https://lint.deno.land/#{}", &self.code)))
  }
}
