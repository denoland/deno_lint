// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
//
// Enforces consistent use of semicolons after statements.
// Similar to ESLint's semi rule.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tag;
use crate::Program;
use deno_ast::{view as ast_view, SourceRanged};

#[derive(Debug)]
pub struct Semi;

const CODE: &str = "semi";
const MESSAGE: &str = "Missing semicolon";
const HINT: &str = "Add a semicolon at the end of the statement";

impl LintRule for Semi {
  fn tags(&self) -> &'static [Tag] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    SemiHandler.traverse(program, context);
  }
}

struct SemiHandler;

impl Handler for SemiHandler {
  fn expr_stmt(&mut self, expr_stmt: &ast_view::ExprStmt, ctx: &mut Context) {
    let parent = expr_stmt.parent();

    // Skip if parent is ForInStmt, ForOfStmt, or ForStmt
    if matches!(
      parent,
      ast_view::Node::ForInStmt(_)
        | ast_view::Node::ForOfStmt(_)
        | ast_view::Node::ForStmt(_)
    ) {
      return;
    }

    let text = expr_stmt.range().text_fast(ctx.text_info());
    let has_semi = text.trim_end().ends_with(';');

    if !has_semi {
      ctx.add_diagnostic_with_hint(expr_stmt.range(), CODE, MESSAGE, HINT);
    }
  }

  fn var_decl(&mut self, var_decl: &ast_view::VarDecl, ctx: &mut Context) {
    let parent = var_decl.parent();

    // Skip if parent is ForInStmt, ForOfStmt, or ForStmt
    if matches!(
      parent,
      ast_view::Node::ForInStmt(_)
        | ast_view::Node::ForOfStmt(_)
        | ast_view::Node::ForStmt(_)
    ) {
      return;
    }

    let text = var_decl.range().text_fast(ctx.text_info());
    let has_semi = text.trim_end().ends_with(';');

    if !has_semi {
      ctx.add_diagnostic_with_hint(var_decl.range(), CODE, MESSAGE, HINT);
    }
  }

  fn debugger_stmt(
    &mut self,
    stmt: &ast_view::DebuggerStmt,
    ctx: &mut Context,
  ) {
    let text = stmt.range().text_fast(ctx.text_info());
    let has_semi = text.trim_end().ends_with(';');

    if !has_semi {
      ctx.add_diagnostic_with_hint(stmt.range(), CODE, MESSAGE, HINT);
    }
  }

  fn throw_stmt(&mut self, stmt: &ast_view::ThrowStmt, ctx: &mut Context) {
    let text = stmt.range().text_fast(ctx.text_info());
    let has_semi = text.trim_end().ends_with(';');

    if !has_semi {
      ctx.add_diagnostic_with_hint(stmt.range(), CODE, MESSAGE, HINT);
    }
  }

  fn import_decl(&mut self, decl: &ast_view::ImportDecl, ctx: &mut Context) {
    let text = decl.range().text_fast(ctx.text_info());
    let has_semi = text.trim_end().ends_with(';');

    if !has_semi {
      ctx.add_diagnostic_with_hint(decl.range(), CODE, MESSAGE, HINT);
    }
  }

  fn do_while_stmt(&mut self, stmt: &ast_view::DoWhileStmt, ctx: &mut Context) {
    let text = stmt.range().text_fast(ctx.text_info());
    let has_semi = text.trim_end().ends_with(';');

    if !has_semi {
      ctx.add_diagnostic_with_hint(stmt.range(), CODE, MESSAGE, HINT);
    }
  }

  fn class_prop(&mut self, prop: &ast_view::ClassProp, ctx: &mut Context) {
    let text = prop.range().text_fast(ctx.text_info());
    let has_semi = text.trim_end().ends_with(';');

    if !has_semi {
      ctx.add_diagnostic_with_hint(prop.range(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn semi_valid() {
    assert_lint_ok! {
        Semi,
        r#"var x = 5;"#,
        r#"var x =5, y;"#,
        r#"foo();"#,
        r#"x = foo();"#,
        r#"for (var a in b){}"#,
        r#"for (var i;;){}"#,
        r#"if (true) {}; [1, 2].forEach(function(){});"#,
        r#"throw new Error('foo');"#,
        r#"debugger;"#,
        r#"import * as utils from './utils';"#,
        r#"let x = 5;"#,
        r#"const x = 5;"#,
        r#"function foo() { return 42; }"#,
        r#"while(true) { break; }"#,
        r#"while(true) { continue; }"#,
        r#"do {} while(true);"#,
        r#"export * from 'foo';"#,
        r#"export { foo } from 'foo';"#,
        r#"export var foo;"#,
        r#"export function foo () { }"#,
        r#"export class Foo { }"#,
        r#"export let foo;"#,
        r#"export const FOO = 42;"#,
        r#"export default foo || bar;"#,
        r#"export default (foo) => foo.bar();"#,
        r#"export default foo = 42;"#,
        r#"export default foo += 42;"#,
        r#"class C { foo; }"#,
        r#"class C { static {} }"#
    };
  }

  #[test]
  fn semi_invalid() {
    // Test for missing semicolons on various statements
    assert_lint_err! {
        Semi,
        r#"let x = 5"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"var x = 5"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"var x = 5, y"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"foo()"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"debugger"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"throw new Error('foo')"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"do{}while(true)"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"import * as utils from './utils'"#: [{
            col: 0,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };

    assert_lint_err! {
        Semi,
        r#"class C { foo }"#: [{
            col: 10,
            message: "Missing semicolon",
            hint: "Add a semicolon at the end of the statement",
        }]
    };
  }
}
