// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::swc_util::StringRepr;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct NoEval;

const CODE: &str = "no-eval";
const MESSAGE: &str = "`eval` call is not allowed";
const HINT: &str = "Remove the use of `eval`";

impl LintRule for NoEval {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoEvalHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoEvalHandler;

impl NoEvalHandler {
  fn maybe_add_diagnostic(
    &mut self,
    source: &dyn StringRepr,
    range: Span,
    ctx: &mut Context,
  ) {
    if source.string_repr().as_deref() == Some("eval") {
      self.add_diagnostic(range, ctx);
    }
  }

  fn add_diagnostic(&mut self, range: Span, ctx: &mut Context) {
    ctx.add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
  }

  fn handle_paren_callee(
    &mut self,
    p: &ParenthesizedExpression,
    ctx: &mut Context,
  ) {
    match &p.expression {
      // Nested paren callee ((eval))('var foo = 0;')
      Expression::ParenthesizedExpression(paren) => {
        self.handle_paren_callee(paren, ctx)
      }
      // Single argument callee: (eval)('var foo = 0;')
      Expression::Identifier(ident) => {
        self.maybe_add_diagnostic(ident.as_ref(), ident.span, ctx)
      }
      // Multiple arguments callee: (0, eval)('var foo = 0;')
      Expression::SequenceExpression(seq) => {
        for expr in &seq.expressions {
          if let Expression::Identifier(ident) = expr {
            self.maybe_add_diagnostic(ident.as_ref(), ident.span, ctx)
          }
        }
      }
      _ => {}
    }
  }
}

impl Handler<'_> for NoEvalHandler {
  fn variable_declarator(&mut self, v: &VariableDeclarator, ctx: &mut Context) {
    if let Some(Expression::Identifier(ident)) = &v.init {
      self.maybe_add_diagnostic(ident.as_ref(), v.span, ctx);
    }
  }

  fn call_expression(&mut self, call_expr: &CallExpression, ctx: &mut Context) {
    match &call_expr.callee {
      Expression::Identifier(ident) => {
        self.maybe_add_diagnostic(ident.as_ref(), call_expr.span, ctx)
      }
      Expression::ParenthesizedExpression(paren) => {
        self.handle_paren_callee(paren, ctx)
      }
      _ => {}
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
