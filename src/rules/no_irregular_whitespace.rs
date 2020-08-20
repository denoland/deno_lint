use super::{Context, LintRule};
use regex::{Matches, Regex};
use std::sync::Arc;
use swc_common::{hygiene::SyntaxContext, BytePos, Span};
use swc_ecmascript::ast::Module;
use swc_ecmascript::ast::Str;
use swc_ecmascript::ast::Tpl;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoIrregularWhitespace;

lazy_static! {
  static ref ALL_IRREGULARS: Regex = Regex::new(r"[\f\v\u0085\ufeff\u00a0\u1680\u180e\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u200b\u202f\u205f\u3000\u2028\u2029]").unwrap();
}

fn test_for_whitespace(value: &str) -> Option<Matches> {
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
    let mut visitor = NoIrregularWhitespaceVisitor::default();
    visitor.visit_module(module, module);

    let excluded_ranges = visitor.ranges.iter();

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
          let span = Span::new(
            BytePos(range.start as u32),
            BytePos(range.end as u32),
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

struct NoIrregularWhitespaceVisitor {
  ranges: Vec<Span>,
}

impl NoIrregularWhitespaceVisitor {
  pub fn default() -> Self {
    Self { ranges: vec![] }
  }
}

impl Visit for NoIrregularWhitespaceVisitor {
  fn visit_str(&mut self, string_literal: &Str, _parent: &dyn Node) {
    self.ranges.push(string_literal.span);
  }

  fn visit_tpl(&mut self, tpl: &Tpl, _parent: &dyn Node) {
    self.ranges.push(tpl.span);
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
    assert_lint_ok::<NoIrregularWhitespace>("function thing() {return '　'};");
  }
}
