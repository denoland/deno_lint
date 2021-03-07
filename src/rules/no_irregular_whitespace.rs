// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::{Matches, Regex};
use swc_common::{hygiene::SyntaxContext, BytePos, Span};
use swc_ecmascript::ast::Str;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoIrregularWhitespace;

const CODE: &str = "no-irregular-whitespace";
const HINT: &str = "Change to a normal space or tab";

#[derive(Display)]
enum NoIrregularWhitespaceMessage {
  #[display(fmt = "Irregular whitespace not allowed.")]
  NotAllowed,
}

static IRREGULAR_WHITESPACE: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"[\f\v\u0085\ufeff\u00a0\u1680\u180e\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u200b\u202f\u205f\u3000]+").unwrap()
});
static IRREGULAR_LINE_TERMINATORS: Lazy<Regex> =
  Lazy::new(|| Regex::new(r"[\u2028\u2029]").unwrap());

fn test_for_whitespace(value: &str) -> Option<Vec<Matches>> {
  let mut matches_vector: Vec<Matches> = vec![];
  if IRREGULAR_WHITESPACE.is_match(value) {
    let matches = IRREGULAR_WHITESPACE.find_iter(value);
    matches_vector.push(matches);
  }
  if IRREGULAR_LINE_TERMINATORS.is_match(value) {
    let matches = IRREGULAR_LINE_TERMINATORS.find_iter(value);
    matches_vector.push(matches);
  }
  if !matches_vector.is_empty() {
    Some(matches_vector)
  } else {
    None
  }
}

impl LintRule for NoIrregularWhitespace {
  fn new() -> Box<Self> {
    Box::new(NoIrregularWhitespace)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = NoIrregularWhitespaceVisitor::default();
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }

    let excluded_ranges = visitor.ranges.iter();

    let span = match program {
      ProgramRef::Module(ref m) => m.span,
      ProgramRef::Script(ref s) => s.span,
    };
    let file_and_lines = context.source_map.span_to_lines(span).unwrap();
    let file = file_and_lines.file;

    for line_index in 0..file.count_lines() {
      let line = file.get_line(line_index).unwrap();
      let (byte_pos, _hi) = file.line_bounds(line_index);

      if let Some(whitespace_results) = test_for_whitespace(&line) {
        for whitespace_matches in whitespace_results.into_iter() {
          for whitespace_match in whitespace_matches {
            let range = whitespace_match.range();
            let span = Span::new(
              byte_pos + BytePos(range.start as u32),
              byte_pos + BytePos(range.end as u32),
              SyntaxContext::empty(),
            );
            let is_excluded =
              excluded_ranges.clone().any(|range| range.contains(span));
            if !is_excluded {
              context.add_diagnostic_with_hint(
                span,
                CODE,
                NoIrregularWhitespaceMessage::NotAllowed,
                HINT,
              );
            }
          }
        }
      }
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of non-space or non-tab whitespace characters

Non-space or non-tab whitespace characters can be very difficult to spot in your
code as editors will often render them invisibly.  These invisible characters can 
cause issues or unexpected behaviors.  Sometimes these characters are added
inadvertently through copy/paste or incorrect keyboard shortcuts.

The following characters are disallowed:
```
\u000B - Line Tabulation (\v) - <VT>
\u000C - Form Feed (\f) - <FF>
\u00A0 - No-Break Space - <NBSP>
\u0085 - Next Line
\u1680 - Ogham Space Mark
\u180E - Mongolian Vowel Separator - <MVS>
\ufeff - Zero Width No-Break Space - <BOM>
\u2000 - En Quad
\u2001 - Em Quad
\u2002 - En Space - <ENSP>
\u2003 - Em Space - <EMSP>
\u2004 - Tree-Per-Em
\u2005 - Four-Per-Em
\u2006 - Six-Per-Em
\u2007 - Figure Space
\u2008 - Punctuation Space - <PUNCSP>
\u2009 - Thin Space
\u200A - Hair Space
\u200B - Zero Width Space - <ZWSP>
\u2028 - Line Separator
\u2029 - Paragraph Separator
\u202F - Narrow No-Break Space
\u205f - Medium Mathematical Space
\u3000 - Ideographic Space
```

To fix this linting issue, replace instances of the above with regular spaces,
tabs or new lines.  If it's not obvious where the offending character(s) are
try retyping the line from scratch.
"#
  }
}

struct NoIrregularWhitespaceVisitor {
  ranges: Vec<Span>,
}

impl NoIrregularWhitespaceVisitor {
  fn default() -> Self {
    Self { ranges: vec![] }
  }
}

impl Visit for NoIrregularWhitespaceVisitor {
  fn visit_str(&mut self, string_literal: &Str, _parent: &dyn Node) {
    self.ranges.push(string_literal.span);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_irregular_whitespace_valid() {
    assert_lint_ok! {
      NoIrregularWhitespace,
      "'\\u{000B}';",
      "'\\u{000C}';",
      "'\\u{0085}';",
      "'\\u{00A0}';",
      "'\\u{180E}';",
      "'\\u{feff}';",
      "'\\u{2000}';",
      "'\\u{2001}';",
      "'\\u{2002}';",
      "'\\u{2003}';",
      "'\\u{2004}';",
      "'\\u{2005}';",
      "'\\u{2006}';",
      "'\\u{2007}';",
      "'\\u{2008}';",
      "'\\u{2009}';",
      "'\\u{200A}';",
      "'\\u{200B}';",
      "'\\u{2028}';",
      "'\\u{2029}';",
      "'\\u{202F}';",
      "'\\u{205f}';",
      "'\\u{3000}';",
      "'\u{000B}';",
      "'\u{000C}';",
      "'\u{0085}';",
      "'\u{00A0}';",
      "'\u{180E}';",
      "'\u{feff}';",
      "'\u{2000}';",
      "'\u{2001}';",
      "'\u{2002}';",
      "'\u{2003}';",
      "'\u{2004}';",
      "'\u{2005}';",
      "'\u{2006}';",
      "'\u{2007}';",
      "'\u{2008}';",
      "'\u{2009}';",
      "'\u{200A}';",
      "'\u{200B}';",
      "'\\\u{2028}';",
      "'\\\u{2029}';",
      "'\u{202F}';",
      "'\u{205f}';",
      "'\u{3000}';",
    };
  }

  #[test]
  fn no_irregular_whitespace_invalid() {
    assert_lint_err! {
      NoIrregularWhitespace,
      "var any \u{000B} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{000C} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{00A0} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{feff} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2000} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2001} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2002} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2003} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2004} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2005} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2006} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2007} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2008} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2009} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{200A} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2028} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{2029} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{202F} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{205f} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{3000} = 'thing';": [
        {
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var a = 'b',\u{2028}c = 'd',\ne = 'f'\u{2028}": [
        {
          line: 1,
          col: 12,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        },
        {
          line: 2,
          col: 7,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ],
      "var any \u{3000} = 'thing', other \u{3000} = 'thing';\nvar third \u{3000} = 'thing';": [
        {
          line: 1,
          col: 8,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        },
        {
          line: 1,
          col: 27,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        },
        {
          line: 2,
          col: 10,
          message: NoIrregularWhitespaceMessage::NotAllowed,
          hint: HINT,
        }
      ]
    };
  }
}
