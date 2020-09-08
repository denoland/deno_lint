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
      "switch(foo) { case 0: a(); /* falls through */ case 1: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: a()\n /* falls through */ case 1: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: a(); /* fall through */ case 1: b(); }",
    );
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: a(); /* fallthrough */ case 1: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: a(); /* FALLS THROUGH */ case 1: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "function foo() { switch(foo) { case 0: a(); return; case 1: b(); }; }",
    );
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: a(); throw 'foo'; case 1: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "while (a) { switch(foo) { case 0: a(); continue; case 1: b(); } }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: a(); break; case 1: b(); }",
    );
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: case 1: a(); break; case 2: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: case 1: break; case 2: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: case 1: break; default: b(); }",
    );
  }

  #[test]
  fn ok_5() {
    assert_lint_ok::<NoFallthrough>("switch(foo) { case 0: case 1: a(); }");

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: case 1: a(); break; }",
    );

    assert_lint_ok::<NoFallthrough>("switch(foo) { case 0: case 1: break; }");
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoFallthrough>("switch(foo) { case 0:\n case 1: break; }");

    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: // comment\n case 1: break; }",
    );

    assert_lint_ok::<NoFallthrough>(
      "function foo() { switch(foo) { case 0: case 1: return; } }",
    );
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoFallthrough>(
      "function foo() { switch(foo) { case 0: {return;}\n case 1: {return;} } }",
    );

    assert_lint_ok::<NoFallthrough>("switch(foo) { case 0: case 1: {break;} }");

    assert_lint_ok::<NoFallthrough>("switch(foo) { }");
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoFallthrough>(
      "switch(foo) { case 0: switch(bar) { case 2: break; } /* falls through */ case 1: break; }",
    );

    assert_lint_ok::<NoFallthrough>(
      "function foo() { switch(foo) { case 1: return a; a++; }}",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: a(); /* falls through */ default:  b(); /* comment */ }",
    );
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: a(); /* falls through */ default: /* comment */ b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: if (a) { break; } else { throw 0; } default: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: try { break; } finally {} default: b(); }",
    );
  }

  #[test]
  fn ok_10() {
    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: try {} finally { break; } default: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: try { throw 0; } catch (err) { break; } default: b(); }",
    );

    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: do { throw 0; } while(a); default: b(); }",
    );
  }

  #[test]
  fn err_1() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a();\ncase 1: b() }",
      0,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a();\ndefault: b() }",
      0,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a(); default: b() }",
      0,
    );
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: if (a) { break; } default: b() }",
      0,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: try { throw 0; } catch (err) {} default: b() }",
      0,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: while (a) { break; } default: b() }",
      0,
    );
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: do { break; } while (a); default: b() }",
      0,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0:\n\n default: b() }",
      0,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0:\n // comment\n default: b() }",
      0,
    );
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a(); /* falling through */ default: b() }",
      0,
    );
  }
}
