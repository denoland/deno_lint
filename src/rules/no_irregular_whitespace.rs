use super::{Context, LintRule};
use regex::{Matches, Regex};
use std::sync::Arc;
use swc_common::{hygiene::SyntaxContext, BytePos, Span};
use swc_ecmascript::ast::Module;
use swc_ecmascript::ast::Str;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoIrregularWhitespace;

lazy_static! {
  static ref IRREGULAR_WHITESPACE: Regex = Regex::new(r"[\f\v\u0085\ufeff\u00a0\u1680\u180e\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u200b\u202f\u205f\u3000]+").unwrap();
  static ref IRREGULAR_LINE_TERMINATORS: Regex = Regex::new(r"[\u2028\u2029]").unwrap();
}

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

  fn code(&self) -> &'static str {
    "no-irregular-whitespace"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut visitor = NoIrregularWhitespaceVisitor::default();
    visitor.visit_module(module, module);

    let excluded_ranges = visitor.ranges.iter();

    let file_and_lines = context.source_map.span_to_lines(module.span).unwrap();
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
              context.add_diagnostic(
                span,
                "no-irregular-whitespace",
                "Irregular whitespace not allowed.",
              );
            }
          }
        }
      }
    }
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
  use crate::test_util::*;

  #[test]
  fn no_irregular_whitespace_valid() {
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{000B}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{000C}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{0085}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{00A0}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{180E}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{feff}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2000}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2001}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2002}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2003}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2004}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2005}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2006}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2007}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2008}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2009}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{200A}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{200B}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2028}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{2029}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{202F}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{205f}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\u{3000}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{000B}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{000C}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{0085}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{00A0}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{180E}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{feff}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2000}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2001}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2002}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2003}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2004}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2005}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2006}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2007}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2008}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{2009}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{200A}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{200B}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\\u{2028}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\\\u{2029}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{202F}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{205f}';");
    assert_lint_ok::<NoIrregularWhitespace>("'\u{3000}';");
  }

  #[test]
  fn no_irregular_whitespace_invalid() {
    assert_lint_err::<NoIrregularWhitespace>("var any \u{000B} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{000C} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{00A0} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{feff} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2000} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2001} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2002} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2003} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2004} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2005} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2006} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2007} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2008} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2009} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{200A} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2028} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{2029} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{202F} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{205f} = 'thing';", 8);
    assert_lint_err::<NoIrregularWhitespace>("var any \u{3000} = 'thing';", 8);
    assert_lint_err_on_line_n::<NoIrregularWhitespace>(
      "var a = 'b',\u{2028}c = 'd',\ne = 'f'\u{2028}",
      vec![(1, 12), (2, 7)],
    );
    assert_lint_err_on_line_n::<NoIrregularWhitespace>(
      "var any \u{3000} = 'thing', other \u{3000} = 'thing';\nvar third \u{3000} = 'thing';",
      vec![(1, 8), (1, 27), (2, 10)],
    );
  }
}
