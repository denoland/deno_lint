use super::{Context, LintRule};
// use crate::swc_common::SourceMap;
use swc_ecmascript::ast::Module;
// use regex::Regex;
use std::sync::Arc;
// use swc_ecma_visit::Visit;

pub struct NoIrregularWhitespace;

impl LintRule for NoIrregularWhitespace {
  fn new() -> Box<Self> {
    Box::new(NoIrregularWhitespace)
  }

  fn code(&self) -> &'static str {
    "no-irregular-whitespace"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let source_code = context.source_map.span_to_string(module.span);
    context.add_diagnostic(
      module.span,
      "no-extra-semi",
      "Unnecessary semicolon.",
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_irregular_whitespace_valid() {
    assert_lint_ok::<NoIrregularWhitespace>(
      "const name = 'space';
      console.log(`The last ${space} in this literal will make it fail`);",
    );
  }
}
