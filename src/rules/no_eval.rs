// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::StringRepr;
use swc_common::Span;
use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::VarDeclarator;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::swc_ecma_ast::ParenExpr;
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

  fn maybe_add_diagnostic(&mut self, source: &dyn StringRepr, span: Span) {
    if source.string_repr().unwrap() == "eval" {
      self.add_diagnostic(span);
    }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      "no-eval",
      "`eval` call is not allowed",
      "Remove the use of `eval`",
    );
  }

  fn handle_paren_callee(&mut self, p: &ParenExpr) {
    match p.expr.as_ref() {
      // Nested paren callee ((eval))('var foo = 0;')
      Expr::Paren(paren) => self.handle_paren_callee(paren),
      // Single argument callee: (eval)('var foo = 0;')
      Expr::Ident(ident) => self.maybe_add_diagnostic(ident, ident.span),
      // Multiple arguments callee: (0, eval)('var foo = 0;')
      Expr::Seq(seq) => {
        for expr in &seq.exprs {
          if let Expr::Ident(ident) = expr.as_ref() {
            self.maybe_add_diagnostic(ident, ident.span)
          }
        }
      }
      _ => {}
    }
  }
}

impl<'c> Visit for NoEvalVisitor<'c> {
  noop_visit_type!();

  fn visit_var_declarator(&mut self, v: &VarDeclarator, _: &dyn Node) {
    if let Some(expr) = &v.init {
      if let Expr::Ident(ident) = expr.as_ref() {
        self.maybe_add_diagnostic(ident, v.span);
      }
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      match expr.as_ref() {
        Expr::Ident(ident) => self.maybe_add_diagnostic(ident, call_expr.span),
        Expr::Member(member_expr) => {
          self.maybe_add_diagnostic(member_expr, call_expr.span)
        }
        Expr::Paren(paren) => self.handle_paren_callee(paren),
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
    assert_lint_err::<NoEval>(r#"(0, eval)("var a = 0");"#, 4);
    assert_lint_err::<NoEval>(r#"((eval))("var a = 0");"#, 2);
    assert_lint_err::<NoEval>(r#"var foo = eval;"#, 4);
    assert_lint_err::<NoEval>(r#"this.eval("123");"#, 0);
  }
}
