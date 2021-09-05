// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use annotate_snippets::display_list;
use annotate_snippets::snippet;
use deno_ast::SourceTextInfo;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::diagnostic::Range;

pub fn display_diagnostics(
  diagnostics: &[LintDiagnostic],
  source_file: &SourceTextInfo,
) {
  for diagnostic in diagnostics {
    let (slice_source, char_range) =
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
          range: char_range.as_tuple(),
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

#[derive(Debug, PartialEq, Eq)]
struct CharRange {
  /// 0-indexed number that represents what index this range starts at in the
  /// snippet.
  /// Counted on a character basis, not UTF-8 bytes.
  start_index: usize,

  /// 0-indexed number that represents what index this range ends at in the
  /// snippet.
  /// Counted on a character basis, not UTF-8 bytes.
  end_index: usize,
}

impl CharRange {
  fn as_tuple(&self) -> (usize, usize) {
    (self.start_index, self.end_index)
  }
}

// Return slice of source code covered by diagnostic
// and adjusted range of diagnostic (ie. original range - start line
// of sliced source code).
fn get_slice_source_and_range<'a>(
  source_file: &'a SourceTextInfo,
  range: &Range,
) -> (&'a str, CharRange) {
  let first_line_start =
    source_file.line_start(range.start.line_index).0 as usize;
  let last_line_end = source_file.line_end(range.end.line_index).0 as usize;
  let text = source_file.text_str();
  let start_index =
    text[first_line_start..range.start.byte_pos].chars().count();
  let end_index = text[first_line_start..range.end.byte_pos].chars().count();
  let slice_str = &text[first_line_start..last_line_end];
  (
    slice_str,
    CharRange {
      start_index,
      end_index,
    },
  )
}

#[cfg(test)]
mod tests {
  use super::*;
  use deno_ast::swc::common::BytePos;
  use deno_lint::diagnostic::{Position, Range};

  fn into_text_info(source_code: impl Into<String>) -> SourceTextInfo {
    SourceTextInfo::from_string(source_code.into())
  }

  fn position(byte: u32, info: &SourceTextInfo) -> Position {
    let b = BytePos(byte);
    Position::new(b, info.line_and_column_index(b))
  }

  #[test]
  fn slice_range_a() {
    let text_info = into_text_info("const a = 42;");
    // 'a'
    let range = Range {
      start: position(6, &text_info),
      end: position(7, &text_info),
    };

    let (slice, char_range) = get_slice_source_and_range(&text_info, &range);
    assert_eq!(slice, "const a = 42;");
    assert_eq!(
      char_range,
      CharRange {
        start_index: 6,
        end_index: 7,
      }
    );
  }

  #[test]
  fn slice_range_あ() {
    let text_info = into_text_info("const あ = 42;");
    // 'あ', which takes up three bytes
    let range = Range {
      start: position(6, &text_info),
      end: position(9, &text_info),
    };

    let (slice, char_range) = get_slice_source_and_range(&text_info, &range);
    assert_eq!(slice, "const あ = 42;");
    assert_eq!(
      char_range,
      CharRange {
        start_index: 6,
        end_index: 7,
      }
    );
  }

  #[test]
  fn slice_range_あい() {
    let text_info = into_text_info("const あい = 42;");
    // 'い', which takes up three bytes
    let range = Range {
      start: position(9, &text_info),
      end: position(12, &text_info),
    };

    let (slice, char_range) = get_slice_source_and_range(&text_info, &range);
    assert_eq!(slice, "const あい = 42;");
    assert_eq!(
      char_range,
      CharRange {
        start_index: 7,
        end_index: 8,
      }
    );
  }

  #[test]
  fn slice_range_across_lines() {
    let src = r#"
const a = `あいうえお
かきくけこ`;
const b = 42;
"#;
    let text_info = into_text_info(src);
    // "えお\nかきく"
    let range = Range {
      start: position(21, &text_info),
      end: position(37, &text_info),
    };

    let (slice, char_range) = get_slice_source_and_range(&text_info, &range);
    assert_eq!(
      slice,
      r#"const a = `あいうえお
かきくけこ`;"#
    );
    assert_eq!(
      char_range,
      CharRange {
        start_index: 14,
        end_index: 20,
      }
    );
  }
}
