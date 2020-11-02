// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::Key;
use swc_common::Span;
use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::VarDeclarator;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoEval;

impl LintRule for NoEval {
  fn new() -> Box<Self> {
    Box::new(NoEval)
  }

  fn code(&self) -> &'static str {
    "no-eval"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoEvalVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of `eval` 

`eval` is a potentially dangerous function which can open your code to a number
of security vulnerabilities.  In addition to being slow, `eval` is also often
unnecessary with better solutions available.
    
### Invalid:
```typescript
const obj = { x: "foo" };
const key = "x",
const value = eval("obj." + key);
```

### Valid:
```typescript
const obj = { x: "foo" };
const value = obj[x];
```
"#
  }
}

struct NoEvalVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoEvalVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn maybe_diagnose_identifier(&mut self, id: &str, span: &Span) {
    if id == "eval" {
      self.context.add_diagnostic_with_hint(
        *span,
        "no-eval",
        "`eval` call is not allowed",
        "Remove the use of `eval`",
      );
    }
  }
}

impl<'c> Visit for NoEvalVisitor<'c> {
  noop_visit_type!();

  fn visit_var_declarator(&mut self, v: &VarDeclarator, _: &dyn Node) {
    if let Some(expr) = &v.init {
      if let Expr::Ident(ident) = expr.as_ref() {
        self.maybe_diagnose_identifier(&ident.sym.as_ref(), &v.span);
      }
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      match expr.as_ref() {
        Expr::Ident(ident) => {
          self.maybe_diagnose_identifier(&ident.sym.as_ref(), &call_expr.span)
        }
        Expr::Member(member_expr) => self.maybe_diagnose_identifier(
          &member_expr.get_key().unwrap(),
          &call_expr.span,
        ),
        _ => {}
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_eval_test() {
    assert_lint_err::<NoEval>(r#"eval("123");"#, 0);
    // TODO These tests should pass (#466)
    // assert_lint_err::<NoEval>(r#"(0, eval)("var a = 0");"#, 0);
    assert_lint_err::<NoEval>(r#"var foo = eval;"#, 4);
    assert_lint_err::<NoEval>(r#"this.eval("123");"#, 0);
  }
}
