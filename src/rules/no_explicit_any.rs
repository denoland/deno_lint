// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::TsKeywordType;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoExplicitAny;

impl LintRule for NoExplicitAny {
  fn new() -> Box<Self> {
    Box::new(NoExplicitAny)
  }

  fn code(&self) -> &'static str {
    "noExplicitAny"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoExplicitAnyVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoExplicitAnyVisitor {
  context: Context,
}

impl NoExplicitAnyVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoExplicitAnyVisitor {
  fn visit_ts_keyword_type(
    &mut self,
    ts_keyword_type: &TsKeywordType,
    _parent: &dyn Node,
  ) {
    use swc_ecma_ast::TsKeywordTypeKind::*;

    if ts_keyword_type.kind == TsAnyKeyword {
      self.context.add_diagnostic(
        ts_keyword_type.span,
        "noExplicitAny",
        "`any` type is not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_explicit_any_test() {
    assert_lint_err::<NoExplicitAny>(
      "function foo(): any { return undefined; }",
      16,
    );
    assert_lint_err::<NoExplicitAny>(
      "function bar(): Promise<any> { return undefined; }",
      24,
    );
    assert_lint_err::<NoExplicitAny>("const a: any = {};", 9);
  }
}
