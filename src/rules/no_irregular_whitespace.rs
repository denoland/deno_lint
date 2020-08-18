use super::{Context, LintRule};
use regex::{Matches, Regex};
use std::sync::Arc;
use swc_common::{hygiene::SyntaxContext, BytePos, Span};
use swc_ecmascript::ast::Module;

pub struct NoIrregularWhitespace;

// TODO(Akin)
// Allow irregular whitespace in strings

fn test_for_whitespace(value: &str) -> Option<Matches> {
  lazy_static! {
    static ref ALL_IRREGULARS: Regex = Regex::new(r"[\f\v\u0085\ufeff\u00a0\u1680\u180e\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u200b\u202f\u205f\u3000\u2028\u2029]").unwrap();
  }
  if ALL_IRREGULARS.is_match(value) {
    let matches = ALL_IRREGULARS.find_iter(value);
    Some(matches)
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
    let lines = context.source_map.span_to_lines(module.span).unwrap().lines;
    for line_info in lines.into_iter() {
      let source_file_and_index = context
        .source_map
        .lookup_line(BytePos(line_info.line_index as u32))
        .unwrap();
      let source_code = source_file_and_index
        .sf
        .get_line(line_info.line_index)
        .unwrap();
      if let Some(whitespace_matches) = test_for_whitespace(&*source_code) {
        for whitespace_match in whitespace_matches {
          let range = whitespace_match.range();
          context.add_diagnostic(
            Span::new(
              BytePos(range.start as u32),
              BytePos(range.end as u32),
              SyntaxContext::empty(),
            ),
            "no-irregular-whitespace",
            "Irregular whitespace not allowed.",
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_irregular_whitespace_valid() {
    assert_lint_err::<NoIrregularWhitespace>("function thing()　{};", 16);
    assert_lint_err::<NoIrregularWhitespace>("const foo = () => { };", 19);
  }
}
