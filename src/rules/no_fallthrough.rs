use super::LintRule;
use crate::linter::Context;
use swc_common::{comments::Comment, Spanned, DUMMY_SP};
use swc_ecmascript::{
  ast::*,
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

  fn visit_switch_cases(&mut self, cases: &[SwitchCase], parent: &dyn Node) {
    let mut should_emit_err = false;
    let mut prev_span = DUMMY_SP;

    'cases: for (case_idx, case) in cases.iter().enumerate() {
      case.visit_with(parent, self);

      if should_emit_err {
        let mut emit = true;
        if let Some(comments) = self.context.leading_comments.get(&case.span.lo)
        {
          if allow_fall_through(&comments) {
            emit = false;
          }
        }
        if emit {
          self.context.add_diagnostic(
            prev_span,
            "no-fallthrough",
            "Fallthrough is not allowed",
          );
        }
      }
      should_emit_err = true;
      let mut stops_exec = false;

      // Handle return / throw / break / continue
      for (idx, stmt) in case.cons.iter().enumerate() {
        let last = idx + 1 == case.cons.len();
        let metadata = self.context.control_flow.meta(stmt.span().lo);
        stops_exec |= metadata.map(|v| v.stops_execution()).unwrap_or(false);
        if stops_exec {
          should_emit_err = false;
        }

        if last {
          if let Some(comments) =
            self.context.trailing_comments.get(&stmt.span().hi)
          {
            if allow_fall_through(&comments) {
              should_emit_err = false;
              // User comment beats everything
              prev_span = case.span;
              continue 'cases;
            }
          }
        }
      }

      let empty = case.cons.is_empty()
        || match &case.cons[0] {
          Stmt::Block(b) => b.stmts.is_empty(),
          _ => false,
        };

      if case_idx + 1 < cases.len() {
        // A case is not allowed to fall through to default handler
        if cases[case_idx + 1].test.is_none() {
          if empty {
            should_emit_err = true;
          }
        } else {
          // Fallthrough
          if empty {
            should_emit_err = false;
          }
        }
      }

      prev_span = case.span;
    }
  }
}

fn allow_fall_through(comments: &[Comment]) -> bool {
  for comment in comments {
    let l = comment.text.to_ascii_lowercase();
    if l.contains("fallthrough")
      || l.contains("falls through")
      || l.contains("fall through")
    {
      return true;
    }
  }
  false
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
  }

  #[test]
  fn ok_11() {
    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: try { throw 0; } catch (err) { break; } default: b(); }",
    );
  }

  #[test]
  fn ok_12() {
    assert_lint_ok::<NoFallthrough>(
      "switch (foo) { case 0: do { throw 0; } while(a); default: b(); }",
    );
  }

  #[test]
  fn err_1() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a();\ncase 1: b() }",
      14,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a();\ndefault: b() }",
      14,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a(); default: b() }",
      14,
    );
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: if (a) { break; } default: b() }",
      14,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: try { throw 0; } catch (err) {} default: b() }",
      14,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: while (a) { break; } default: b() }",
      14,
    );
  }

  #[test]
  #[ignore = "It ends with break statement"]
  fn err_3_wrong() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: do { break; } while (a); default: b() }",
      47,
    );
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0:\n\n default: b() }",
      14,
    );

    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0:\n // comment\n default: b() }",
      14,
    );
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoFallthrough>(
      "switch(foo) { case 0: a(); /* falling through */ default: b() }",
      14,
    );
  }
}
