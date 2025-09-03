// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::StringRepr;
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, ParenExpr, VarDeclarator};
use deno_ast::{SourceRange, SourceRanged};

#[derive(Debug)]
pub struct NoEval;

const CODE: &str = "no-eval";
const MESSAGE: &str = "`eval` call is not allowed";
const HINT: &str = "Remove the use of `eval`";

impl LintRule for NoEval {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoEvalHandler.traverse(program, context);
  }
}

struct NoEvalHandler;

impl NoEvalHandler {
  fn maybe_add_diagnostic(
    &mut self,
    source: &dyn StringRepr,
    range: SourceRange,
    ctx: &mut Context,
  ) {
    if source.string_repr().as_deref() == Some("eval") {
      self.add_diagnostic(range, ctx);
    }
  }

  fn add_diagnostic(&mut self, range: SourceRange, ctx: &mut Context) {
    ctx.add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
  }

  fn handle_paren_callee(&mut self, p: &ParenExpr, ctx: &mut Context) {
    match p.expr {
      // Nested paren callee ((eval))('var foo = 0;')
      Expr::Paren(paren) => self.handle_paren_callee(paren, ctx),
      // Single argument callee: (eval)('var foo = 0;')
      Expr::Ident(ident) => {
        self.maybe_add_diagnostic(ident, ident.range(), ctx)
      }
      // Multiple arguments callee: (0, eval)('var foo = 0;')
      Expr::Seq(seq) => {
        for expr in seq.exprs {
          if let Expr::Ident(ident) = expr {
            self.maybe_add_diagnostic(*ident, ident.range(), ctx)
          }
        }
      }
      _ => {}
    }
  }
}

impl Handler for NoEvalHandler {
  fn var_declarator(&mut self, v: &VarDeclarator, ctx: &mut Context) {
    if let Some(Expr::Ident(ident)) = &v.init {
      self.maybe_add_diagnostic(*ident, v.range(), ctx);
    }
  }

  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    if let Callee::Expr(expr) = &call_expr.callee {
      match expr {
        Expr::Ident(ident) => {
          self.maybe_add_diagnostic(*ident, call_expr.range(), ctx)
        }
        Expr::Paren(paren) => self.handle_paren_callee(paren, ctx),
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
