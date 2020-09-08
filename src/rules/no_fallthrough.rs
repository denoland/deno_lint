use super::LintRule;
use crate::linter::Context;
use swc_common::Spanned;
use swc_ecmascript::{
  ast::Decl,
  ast::Stmt,
  ast::VarDecl,
  ast::VarDeclKind,
  visit::{noop_visit_type, Node, Visit, VisitWith},
};

use std::sync::Arc;

pub struct NoFallthrough;

impl LintRule for NoFallthrough {
  fn new() -> Box<Self> {
    Box::new(NoFallthrough)
  }

  fn code(&self) -> &'static str {
    "no-fallthrough"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoFallthroughVisitor { context };
    visitor.visit_module(module, module);
  }
}

struct NoFallthroughVisitor {
  context: Arc<Context>,
}

impl Visit for NoFallthroughVisitor {
  noop_visit_type!();

  fn visit_stmt(&mut self, stmt: &Stmt, _: &dyn Node) {
    stmt.visit_children_with(self);

    match stmt {
      // Don't print unused error for block statements
      Stmt::Block(_) => return,
      // Hoisted, so reachable.
      Stmt::Decl(Decl::Fn(..)) => return,
      Stmt::Decl(Decl::Var(VarDecl {
        kind: VarDeclKind::Var,
        decls,
        ..
      }))
        if decls.iter().all(|decl| decl.init.is_none()) =>
      {
        return;
      }
      _ => {}
    }

    if let Some(meta) = self.context.control_flow.meta(stmt.span().lo) {
      if meta.fallthrough {
        self.context.add_diagnostic(
          stmt.span(),
          "no-fallthrough",
          "Fallthrough is not allowed",
        )
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_5() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }

  #[test]
  fn ok_10() {
    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );

    assert_lint_ok::<NoFallthrough>(
      "
      ",
    );
  }
}
