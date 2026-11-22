// Copyright 2020-2026 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::Program;
use deno_ast::view::{Expr, Ident, Lit, MemberExpr, MemberProp};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoCommonjsExports;

const CODE: &str = "no-commonjs-exports";
const MESSAGE: &str = "CommonJS exports are not allowed";
const HINT: &str = "Use ES module exports instead: `export const name = value;` or `export default value;`";

impl LintRule for NoCommonjsExports {
  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> Tags {
    &[]
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoCommonjsExportsHandler.traverse(program, context);
  }
}

struct NoCommonjsExportsHandler;

impl Handler for NoCommonjsExportsHandler {
  fn ident(&mut self, ident: &Ident, ctx: &mut Context) {
    if ident.sym() == "exports" && ident.ctxt() == ctx.unresolved_ctxt() {
      ctx.add_diagnostic_with_hint(ident.range(), CODE, MESSAGE, HINT);
    }
  }

  fn member_expr(&mut self, expr: &MemberExpr, ctx: &mut Context) {
    let Expr::Ident(ident) = expr.obj else {
      return;
    };
    if ident.sym() != "module" || ident.ctxt() != ctx.unresolved_ctxt() {
      return;
    }
    let is_exports = match expr.prop {
      MemberProp::Ident(prop) => prop.sym() == "exports",
      MemberProp::Computed(prop) => match prop.expr {
        Expr::Lit(Lit::Str(value)) => value.value() == "exports",
        Expr::Tpl(tpl) if tpl.exprs.is_empty() && tpl.quasis.len() == 1 => {
          tpl.quasis[0].raw() == "exports"
        }
        _ => false,
      },
      _ => false,
    };
    if is_exports {
      ctx.add_diagnostic_with_hint(expr.range(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid() {
    assert_lint_ok! {
      NoCommonjsExports,
      "export const foo = 1; export default foo;",
      "const exports = {}; exports.foo = 1;",
      "const module = { exports: {} }; module.exports.foo = 1;",
      "function f(exports, module) { exports.foo = 1; module.exports = {}; }",
      "exports.foo = 1; var exports;",
      "module.exports = 1; var module;",
      "import exports from './exports.js'; exports.foo = 1;",
      "import module from './module.js'; module.exports = 1;",
      "const obj = { exports: 1 }; obj.exports;",
      "class Foo { exports() {} }",
      "module.filename; module[foo]; module[`ex${foo}ports`];",
      "const exports = 'other'; module[exports] = 1;",
      "const obj: { exports: number } = { exports: 1 };",
      "// deno-lint-ignore no-commonjs-exports\nmodule.exports = 1;",
    }
  }

  #[test]
  fn invalid() {
    assert_lint_err! {
      NoCommonjsExports,
      "exports.foo = 1;": [{ col: 0, message: MESSAGE, hint: HINT }],
      "exports['foo'] = 1;": [{ col: 0, message: MESSAGE, hint: HINT }],
      "module.exports = 1;": [{ col: 0, message: MESSAGE, hint: HINT }],
      "module.exports.foo = 1;": [{ col: 0, message: MESSAGE, hint: HINT }],
      "module['exports'] = 1;": [{ col: 0, message: MESSAGE, hint: HINT }],
      "module[`exports`] = 1;": [{ col: 0, message: MESSAGE, hint: HINT }],
      "const value = module.exports;": [{ col: 14, message: MESSAGE, hint: HINT }],
      "const value = exports;": [{ col: 14, message: MESSAGE, hint: HINT }],
      "const value = { exports };": [{ col: 16, message: MESSAGE, hint: HINT }],
      "Object.assign(exports, { foo: 1 });": [{ col: 14, message: MESSAGE, hint: HINT }],
      "function f() { module.exports = 1; }": [{ col: 15, message: MESSAGE, hint: HINT }],
    }
  }
}
