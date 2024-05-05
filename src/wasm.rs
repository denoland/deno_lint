// Copyright 2020-2023 the Deno authors. All rights reserved. MIT license.

use crate::diagnostic::LintDiagnostic;
use crate::linter::LintFileOptions;
use crate::linter::LinterBuilder;
use crate::rules::get_recommended_rules;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_ast::SourceRange;
use deno_ast::SourceRanged as _;
use deno_ast::SourceTextInfo;
use deno_ast::StartSourcePos;
use std::fmt::Display;
use wasm_bindgen::prelude::*;

#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn run(filename: String, source_code: String) -> Result<String, String> {
  let linter = LinterBuilder::default()
    .rules(get_recommended_rules())
    .build();
  let specifier = ModuleSpecifier::parse(&format!("file:///{filename}"))
    .map_err(|e| e.to_string())?;
  let media_type = MediaType::from_specifier(&specifier);
  let (parsed_source, diagnostics) = linter
    .lint_file(LintFileOptions {
      specifier,
      source_code,
      media_type,
    })
    .map_err(|e| e.to_string())?;
  let file_diagnostics = FileDiagnostics {
    filename,
    text_info: parsed_source.text_info().clone(),
    diagnostics,
  };

  Ok(display_diagnostics(file_diagnostics))
}

struct FileDiagnostics {
  filename: String,
  text_info: SourceTextInfo,
  diagnostics: Vec<LintDiagnostic>,
}

fn display_diagnostics(file_diagnostics: FileDiagnostics) -> String {
  let mut displayed_diagnostics =
    Vec::with_capacity(file_diagnostics.diagnostics.len());
  let reporter = miette::GraphicalReportHandler::new();

  for diagnostic in &file_diagnostics.diagnostics {
    let miette_source_code = MietteSourceCode {
      source: &file_diagnostics.text_info,
      filename: file_diagnostics.filename.as_str(),
    };

    let mut s = String::new();
    let miette_diag = MietteDiagnostic {
      source_code: &miette_source_code,
      lint_diagnostic: diagnostic,
    };
    reporter.render_report(&mut s, &miette_diag).unwrap();
    displayed_diagnostics.push(s);
  }

  displayed_diagnostics.join("\n\n")
}

#[derive(Debug)]
struct MietteDiagnostic<'a> {
  source_code: &'a MietteSourceCode<'a>,
  lint_diagnostic: &'a LintDiagnostic,
}

impl std::error::Error for MietteDiagnostic<'_> {}

impl Display for MietteDiagnostic<'_> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.write_str(&self.lint_diagnostic.message)
  }
}

impl miette::Diagnostic for MietteDiagnostic<'_> {
  fn code<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
    Some(Box::new(self.lint_diagnostic.code.to_string()))
  }

  fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
    Some(Box::new(format!(
      "https://lint.deno.land/#{}",
      self.lint_diagnostic.code
    )))
  }

  fn source_code(&self) -> Option<&dyn miette::SourceCode> {
    Some(self.source_code)
  }

  fn labels(
    &self,
  ) -> Option<Box<dyn Iterator<Item = miette::LabeledSpan> + '_>> {
    let len = self
      .lint_diagnostic
      .range
      .end
      .as_byte_index(StartSourcePos::START_SOURCE_POS)
      - self
        .lint_diagnostic
        .range
        .start
        .as_byte_index(StartSourcePos::START_SOURCE_POS);
    let start = miette::SourceOffset::from(
      self
        .lint_diagnostic
        .range
        .start
        .as_byte_index(StartSourcePos::START_SOURCE_POS),
    );
    let len = miette::SourceOffset::from(len);
    let span = miette::SourceSpan::new(start, len);
    let text = self
      .lint_diagnostic
      .hint
      .as_ref()
      .map(|help| help.to_string());
    let labels = vec![miette::LabeledSpan::new_with_span(text, span)];
    Some(Box::new(labels.into_iter()))
  }
}

#[derive(Debug)]
struct MietteSourceCode<'a> {
  source: &'a SourceTextInfo,
  filename: &'a str,
}

impl miette::SourceCode for MietteSourceCode<'_> {
  fn read_span<'a>(
    &'a self,
    span: &miette::SourceSpan,
    context_lines_before: usize,
    context_lines_after: usize,
  ) -> Result<Box<dyn miette::SpanContents<'a> + 'a>, miette::MietteError> {
    let start_pos = self.source.range().start;
    let lo = start_pos + span.offset();
    let hi = lo + span.len();

    let start_line_column = self.source.line_and_column_index(lo);

    let start_line_index =
      if context_lines_before > start_line_column.line_index {
        0
      } else {
        start_line_column.line_index - context_lines_before
      };
    let src_start = self.source.line_start(start_line_index);
    let end_line_column = self.source.line_and_column_index(hi);
    let line_count = self.source.lines_count();
    let end_line_index = std::cmp::min(
      end_line_column.line_index + context_lines_after,
      self.source.text_str().len(),
    );
    let src_end = self
      .source
      .line_end(std::cmp::min(end_line_index, line_count - 1));
    let range = SourceRange::new(src_start, src_end);
    let src_text = range.text_fast(self.source);
    let byte_range = range.as_byte_range(start_pos);
    let name = Some(self.filename.to_string());
    let start = miette::SourceOffset::from(byte_range.start);
    let len = miette::SourceOffset::from(byte_range.len());
    let span = miette::SourceSpan::new(start, len);

    Ok(Box::new(SpanContentsImpl {
      data: src_text,
      span,
      line: start_line_column.line_index,
      column: start_line_column.column_index,
      line_count,
      name,
    }))
  }
}

struct SpanContentsImpl<'a> {
  data: &'a str,
  span: miette::SourceSpan,
  line: usize,
  column: usize,
  line_count: usize,
  name: Option<String>,
}

impl<'a> miette::SpanContents<'a> for SpanContentsImpl<'a> {
  fn data(&self) -> &'a [u8] {
    self.data.as_bytes()
  }

  fn span(&self) -> &miette::SourceSpan {
    &self.span
  }

  fn line(&self) -> usize {
    self.line
  }

  fn column(&self) -> usize {
    self.column
  }

  fn line_count(&self) -> usize {
    self.line_count
  }

  fn name(&self) -> Option<&str> {
    self.name.as_deref()
  }
}
