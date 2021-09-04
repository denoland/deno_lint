// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use annotate_snippets::display_list;
use annotate_snippets::snippet;
use ast_view::SourceFile;
use ast_view::SourceFileTextInfo;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::diagnostic::Range;

pub fn display_diagnostics(
  diagnostics: &[LintDiagnostic],
  source_file: &SourceFileTextInfo,
) {
  for diagnostic in diagnostics {
    let (slice_source, range) =
      get_slice_source_and_range(source_file, &diagnostic.range);
    let footer = if let Some(hint) = &diagnostic.hint {
      vec![snippet::Annotation {
        label: Some(hint),
        id: None,
        annotation_type: snippet::AnnotationType::Help,
      }]
    } else {
      vec![]
    };

    let snippet = snippet::Snippet {
      title: Some(snippet::Annotation {
        label: Some(&diagnostic.message),
        id: Some(&diagnostic.code),
        annotation_type: snippet::AnnotationType::Error,
      }),
      footer,
      slices: vec![snippet::Slice {
        source: slice_source,
        line_start: diagnostic.range.start.line_index + 1, // make 1-indexed
        origin: Some(&diagnostic.filename),
        fold: false,
        annotations: vec![snippet::SourceAnnotation {
          range,
          label: "",
          annotation_type: snippet::AnnotationType::Error,
        }],
      }],
      opt: display_list::FormatOptions {
        color: true,
        anonymized_line_numbers: false,
        margin: None,
      },
    };
    let display_list = display_list::DisplayList::from(snippet);
    eprintln!("{}", display_list);
  }
}

// Return slice of source code covered by diagnostic
// and adjusted range of diagnostic (ie. original range - start line
// of sliced source code).
fn get_slice_source_and_range<'a>(
  source_file: &'a SourceFileTextInfo,
  range: &Range,
) -> (&'a str, (usize, usize)) {
  let first_line_start =
    source_file.line_start(range.start.line_index).0 as usize;
  let last_line_end = source_file.line_end(range.end.line_index).0 as usize;
  let adjusted_start = range.start.byte_pos - first_line_start;
  let adjusted_end = range.end.byte_pos - first_line_start;
  let adjusted_range = (adjusted_start, adjusted_end);
  let slice_str = &source_file.text()[first_line_start..last_line_end];
  (slice_str, adjusted_range)
}
