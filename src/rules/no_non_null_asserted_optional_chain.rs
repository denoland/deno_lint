// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

use std::sync::Arc;

pub struct NoNonNullAssertedOptionalChain;

impl LintRule for NoNonNullAssertedOptionalChain {
  fn new() -> Box<Self> {
    Box::new(NoNonNullAssertedOptionalChain)
  }
  fn code(&self) -> &'static str {
    "no-non-null-asserted-optional-chain"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = NoNonNullAssertedOptionalChainVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoNonNullAssertedOptionalChainVisitor {
  context: Arc<Context>,
}

impl NoNonNullAssertedOptionalChainVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoNonNullAssertedOptionalChainVisitor {
  fn visit_ts_non_null_expr(
    &mut self,
    non_null_expr: &swc_ecma_ast::TsNonNullExpr,
    _parent: &dyn Node,
  ) {
    match non_null_expr {
      swc_ecma_ast::Expr::OptChainExpr(opt_exp) => {
        self.context.add_diagnostic(
          opt_exp.span,
          "no-non-null-asserted-optional-chain",
          "do not use non-null asserted optional chain",
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn should_ok() {
    assert_lint_ok::<NoNonNullAssertedOptionalChain>("instance.doWork();");
  }

  #[test]
  fn should_err() {
    assert_lint_err::<NoNonNullAssertedOptionalChain>("instance!.doWork()", 0);
  }
}
