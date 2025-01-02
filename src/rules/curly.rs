// Copyright 2020-2025 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::tags::Tags;
use crate::Program;

use deno_ast::view::DoWhileStmt;
use deno_ast::view::ForInStmt;
use deno_ast::view::ForOfStmt;
use deno_ast::view::ForStmt;
use deno_ast::view::IfStmt;
use deno_ast::view::NodeTrait;
use deno_ast::view::Stmt;
use deno_ast::view::WhileStmt;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct Curly;

const CODE: &str = "curly";
const MESSAGE: &str =
  "Enforce consistent brace style for all control statements";
const HINT: &str = "Add curly braces around this statement";

impl LintRule for Curly {
  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    CurlyHandler.traverse(program, context);
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> Tags {
    &[]
  }
}

struct CurlyHandler;

impl CurlyHandler {
  fn add_diagnostic(
    &mut self,
    ctx: &mut Context,
    range: SourceRange,
    text: &str,
  ) {
    ctx.add_diagnostic_with_fixes(
      range,
      CODE,
      MESSAGE,
      Some(HINT.to_string()),
      vec![LintFix {
        description: "Add curly braces".into(),
        changes: vec![LintFixChange {
          new_text: format!("{}\n  {}\n{}", "{", text, "}").into(),
          range,
        }],
      }],
    );
  }

  fn report_no_block(&mut self, ctx: &mut Context, stmt: Stmt) {
    match stmt {
      Stmt::Block(_) => {}
      _ => {
        self.add_diagnostic(ctx, stmt.range(), stmt.text());
      }
    }
  }
}

impl Handler for CurlyHandler {
  fn if_stmt(&mut self, node: &IfStmt, ctx: &mut Context) {
    self.report_no_block(ctx, node.cons);

    if let Some(alt) = node.alt {
      self.report_no_block(ctx, alt);
    }
  }

  fn while_stmt(&mut self, node: &WhileStmt, ctx: &mut Context) {
    self.report_no_block(ctx, node.body);
  }

  fn do_while_stmt(&mut self, node: &DoWhileStmt, ctx: &mut Context) {
    self.report_no_block(ctx, node.body);
  }

  fn for_stmt(&mut self, node: &ForStmt, ctx: &mut Context) {
    self.report_no_block(ctx, node.body);
  }

  fn for_in_stmt(&mut self, node: &ForInStmt, ctx: &mut Context) {
    self.report_no_block(ctx, node.body);
  }

  fn for_of_stmt(&mut self, node: &ForOfStmt, ctx: &mut Context) {
    self.report_no_block(ctx, node.body);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid() {
    assert_lint_ok! {
      Curly,
      "if (foo) { foo; }",
      "if (foo) { foo; } else { bar; }",
      "while (foo) { foo; }",
      "do { foo; } while (foo)",
      "for (;;) { foo; }",
      "for (const a in b) { foo; }",
      "for (const a of b) { foo; }",
    }
  }

  #[test]
  fn invalid() {
    assert_lint_err! {
      Curly,
      "if (foo) foo;": [
        {
          col: 9,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Add curly braces",
            r#"if (foo) {
  foo;
}"#
          ),
        }
      ],
      "if (foo) { foo; } else bar;": [
        {
          col: 23,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Add curly braces",
            r#"if (foo) { foo; } else {
  bar;
}"#
          ),
        }
      ],
      "while (foo) bar;": [
        {
          col: 12,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Add curly braces",
            r#"while (foo) {
  bar;
}"#
          ),
        }
      ],
      "do bar; while (foo);": [
        {
          col: 3,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Add curly braces",
            r#"do {
  bar;
} while (foo);"#
          ),
        }
      ],
      "for (;;) foo;": [
        {
          col: 9,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Add curly braces",
            r#"for (;;) {
  foo;
}"#
          ),
        }
      ],
      "for (const a in b) foo;": [
        {
          col: 19,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Add curly braces",
            r#"for (const a in b) {
  foo;
}"#
          ),
        }
      ],
      "for (const a of b) foo;": [
        {
          col: 19,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            "Add curly braces",
            "for (const a of b) {
  foo;
}"
          ),
        }
      ]
    };
  }
}
