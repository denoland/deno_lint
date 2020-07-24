// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::Tpl;
// use crate::swc_ecma_ast::;
use regex::Regex;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

use std::sync::Arc;

pub struct NoIrregularWhitespace;

impl LintRule for NoIrregularWhitespace {
  fn new() -> Box<Self> {
    Box::new(NoIrregularWhitespace)
  }

  fn code(&self) -> &'static str {
    "no-irregular-whitespace"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = NoIrregularWhitespaceVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoIrregularWhitespaceVisitor {
  context: Arc<Context>,
}

impl NoIrregularWhitespaceVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-irregular-whitespace",
      "Irregular whitespace not allowed.",
    );
  }
}

impl Visit for NoIrregularWhitespaceVisitor {
  fn visit_tpl(&mut self, tpl: &Tpl, _parent: &dyn Node) {
    lazy_static! {
     static ref ALL_IRREGULARS: Regex = Regex::new(r"[\f\v\u0085\ufeff\u00a0\u1680\u180e\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u200b\u202f\u205f\u3000\u2028\u2029]").unwrap();
     static ref IRREGULAR_WHITESPACE: Regex = Regex::new(r"[\f\v\u0085\ufeff\u00a0\u1680\u180e\u2000\u2001\u2002\u2003\u2004\u2005\u2006\u2007\u2008\u2009\u200a\u200b\u202f\u205f\u3000\u2028\u2029]+(?mg)").unwrap();
     static ref IRREGULAR_LINE_TERMINATORS: Regex = Regex::new(r"[\u2028\u2029](?gu)").unwrap();
     static ref LINE_BREAK_MATCHER: Regex = Regex::new(r"[^\r\n]+").unwrap();
    }

    for tpl_ltrl in &tpl.quasis {
      if ALL_IRREGULARS.is_match(&tpl_ltrl.raw.value) {
        self.add_diagnostic(tpl_ltrl.span);
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
    assert_lint_err_on_line::<NoIrregularWhitespace>(
      "const name = 'space';
      console.log(`The last ${space} in this literal will make it fail`);",
      2,
      36,
    );
  }
}
