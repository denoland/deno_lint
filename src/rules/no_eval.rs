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

  fn maybe_add_diagnostic(&mut self, source: &str, span: Span) {
    if source.contains("eval") {
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
}

impl<'c> Visit for NoEvalVisitor<'c> {
  noop_visit_type!();

  fn visit_var_declarator(&mut self, v: &VarDeclarator, _: &dyn Node) {
    if let Some(expr) = &v.init {
      if let Expr::Ident(ident) = expr.as_ref() {
        let ident_name = &ident.string_repr().unwrap();
        self.maybe_add_diagnostic(ident_name, v.span);
      }
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      match expr.as_ref() {
        Expr::Ident(ident) => {
          let ident_name = &ident.string_repr().unwrap();
          self.maybe_add_diagnostic(ident_name, call_expr.span)
        }
        Expr::Member(member_expr) => {
          let member_name = &member_expr.string_repr().unwrap();
          self.maybe_add_diagnostic(member_name, call_expr.span)
        }
        Expr::Paren(paren) => {
          let paren_snippet =
            &self.context.source_map.span_to_snippet(paren.span).unwrap();
          self.maybe_add_diagnostic(paren_snippet, call_expr.span);
        }
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
    assert_lint_err::<NoEval>(r#"(0, eval)("var a = 0");"#, 0);
    assert_lint_err::<NoEval>(r#"var foo = eval;"#, 4);
    assert_lint_err::<NoEval>(r#"this.eval("123");"#, 0);
  }
}
