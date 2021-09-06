// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::swc_util::StringRepr;
use crate::ProgramRef;
use deno_ast::swc::ast::CallExpr;
use deno_ast::swc::ast::Expr;
use deno_ast::swc::ast::ExprOrSuper;
use deno_ast::swc::ast::ParenExpr;
use deno_ast::swc::ast::VarDeclarator;
use deno_ast::swc::common::Span;
use deno_ast::swc::visit::noop_visit_type;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;

#[derive(Debug)]
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
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_eval.md")
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
