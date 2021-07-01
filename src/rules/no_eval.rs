// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use crate::swc_util::StringRepr;
use swc_common::Span;
use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::ParenExpr;
use swc_ecmascript::ast::VarDeclarator;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoEval;

const CODE: &str = "no-eval";
const MESSAGE: &str = "`eval` call is not allowed";
const HINT: &str = "Remove the use of `eval`";

impl LintRule for NoEval {
  fn new() -> Box<Self> {
    Box::new(NoEval)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoEvalVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
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

struct NoEvalVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoEvalVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn maybe_add_diagnostic(&mut self, source: &dyn StringRepr, span: Span) {
    if source.string_repr().as_deref() == Some("eval") {
      self.add_diagnostic(span);
    }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self
      .context
      .add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
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

impl<'c, 'view> Visit for NoEvalVisitor<'c, 'view> {
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
        Expr::Paren(paren) => self.handle_paren_callee(paren),
        _ => {}
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_eval_valid() {
    assert_lint_ok! {
      NoEval,
      "foo.eval('bar');",
    }
  }

  #[test]
  fn no_eval_invalid() {
    assert_lint_err! {
      NoEval,
      "eval('123');": [{col: 0, message: MESSAGE, hint: HINT}],
      "(0, eval)('var a = 0');": [{col: 4, message: MESSAGE, hint: HINT}],
      "((eval))('var a = 0');": [{col: 2, message: MESSAGE, hint: HINT}],
      "var foo = eval;": [{col: 4, message: MESSAGE, hint: HINT}],

      // TODO (see: https://github.com/denoland/deno_lint/pull/490)
      // "this.eval("123");": [{col: 0, message: MESSAGE, hint: HINT}],
      // "var foo = this.eval;": [{col: 0, message: MESSAGE, hint: HINT}],
      // "(function(exe){ exe('foo') })(eval);": [{col: 0, message: MESSAGE, hint: HINT}],
      //
      // "(0, window.eval)('foo');": [{col: 0, message: MESSAGE, hint: HINT}],
      // "(0, window['eval'])('foo');": [{col: 0, message: MESSAGE, hint: HINT}],
      // "var foo = window.eval;": [{col: 0, message: MESSAGE, hint: HINT}],
      // "window.eval('foo');": [{col: 0, message: MESSAGE, hint: HINT}],
      // "window.window.eval('foo');": [{col: 0, message: MESSAGE, hint: HINT}],
      // "window.window['eval']('foo');": [{col: 0, message: MESSAGE, hint: HINT}],
      //
      // "var foo = globalThis.eval;": [{col: 0, message: MESSAGE, hint: HINT}],
      // "globalThis.eval('foo')": [{col: 0, message: MESSAGE, hint: HINT}],
      // "globalThis.globalThis.eval('foo')": [{col: 0, message: MESSAGE, hint: HINT}],
      // "globalThis.globalThis['eval']('foo')": [{col: 0, message: MESSAGE, hint: HINT}],
      // "(0, globalThis.eval)('foo')": [{col: 0, message: MESSAGE, hint: HINT}],
      // "(0, globalThis['eval'])('foo')": [{col: 0, message: MESSAGE, hint: HINT}],
    }
  }
}
