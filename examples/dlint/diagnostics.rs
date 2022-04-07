// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use deno_ast::swc::common::BytePos;
use deno_ast::SourceTextInfo;
use deno_lint::diagnostic::LintDiagnostic;
use std::fmt::Display;

pub fn display_diagnostics(
  diagnostics: &[LintDiagnostic],
  source_file: &SourceTextInfo,
  filename: &str,
) {
  let reporter = miette::GraphicalReportHandler::new();
  let miette_source_code = MietteSourceCode {
    source: source_file,
    filename,
  };

  for diagnostic in diagnostics {
    let mut s = String::new();
    let miette_diag = MietteDiagnostic {
      source_code: &miette_source_code,
      lint_diagnostic: diagnostic,
    };
    reporter.render_report(&mut s, &miette_diag).unwrap();
    eprintln!("{}", s);
  }
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

  fn severity(&self) -> Option<miette::Severity> {
    None
  }

  fn help<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
    None
  }

  fn url<'a>(&'a self) -> Option<Box<dyn Display + 'a>> {
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
    let len = self.lint_diagnostic.range.end.byte_pos
      - self.lint_diagnostic.range.start.byte_pos;
    let start =
      miette::SourceOffset::from(self.lint_diagnostic.range.start.byte_pos);
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
    let lo = span.offset();
    let hi = lo + span.len();

    let start_line_column = self
      .source
      .line_and_column_index(BytePos(lo.try_into().unwrap()));

    let start_line_index =
      if context_lines_before > start_line_column.line_index {
        0
      } else {
        start_line_column.line_index - context_lines_before
      };
    let src_start = self.source.line_start(start_line_index).0 as usize;
    let end_line_column = self
      .source
      .line_and_column_index(BytePos(hi.try_into().unwrap()));
    let line_count = self.source.lines_count();
    let end_line_index = std::cmp::min(
      end_line_column.line_index + context_lines_after,
      self.source.text().len(),
    );
    let src_end = self.source.line_end(end_line_index).0 as usize;
    let src = &self.source.text_str()[src_start..src_end];
    let name = Some(self.filename.to_string());
    let start = miette::SourceOffset::from(src_start);
    let len = miette::SourceOffset::from(src_end - src_start);
    let span = miette::SourceSpan::new(start, len);

    Ok(Box::new(SpanContentsImpl {
      data: src,
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
