// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::span::Span;
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::{Matches, Regex};

#[derive(Debug)]
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

fn test_for_whitespace(value: &str) -> Vec<Matches<'_, '_>> {
  let mut matches_vector: Vec<Matches> = vec![];
  if IRREGULAR_WHITESPACE.is_match(value) {
    let matches = IRREGULAR_WHITESPACE.find_iter(value);
    matches_vector.push(matches);
  }
  if IRREGULAR_LINE_TERMINATORS.is_match(value) {
    let matches = IRREGULAR_LINE_TERMINATORS.find_iter(value);
    matches_vector.push(matches);
  }
  matches_vector
}

/// Collects spans of string literals, template literals, and regex literals
/// that should be excluded from irregular whitespace checking.
struct LiteralSpanCollector {
  literal_spans: Vec<Span>,
}

impl LiteralSpanCollector {
  fn new() -> Self {
    Self {
      literal_spans: Vec::new(),
    }
  }
}

impl<'a> Visit<'a> for LiteralSpanCollector {
  fn visit_string_literal(&mut self, lit: &StringLiteral<'a>) {
    self.literal_spans.push(lit.span);
  }

  fn visit_template_literal(&mut self, lit: &TemplateLiteral<'a>) {
    self.literal_spans.push(lit.span);
    walk::walk_template_literal(self, lit);
  }

  fn visit_reg_exp_literal(&mut self, lit: &RegExpLiteral<'a>) {
    self.literal_spans.push(lit.span);
  }
}

fn is_inside_literal(pos: u32, literal_spans: &[Span]) -> bool {
  literal_spans
    .iter()
    .any(|span| pos >= span.start && pos < span.end)
}

impl LintRule for NoIrregularWhitespace {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    // Collect literal spans to exclude
    let mut collector = LiteralSpanCollector::new();
    collector.visit_program(program);

    let source_text = context.source_text();

    // Check entire source text for irregular whitespace
    for whitespace_matches in test_for_whitespace(source_text).into_iter() {
      for whitespace_match in whitespace_matches {
        let start = whitespace_match.start() as u32;
        let end = whitespace_match.end() as u32;

        // Skip if inside a literal
        if is_inside_literal(start, &collector.literal_spans) {
          continue;
        }

        let span = Span::new(start, end);
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
