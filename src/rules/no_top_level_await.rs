// Copyright 2020-2022 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::view::NodeTrait;
use deno_ast::view::{self as ast_view};
use deno_ast::SourceRanged;
use if_chain::if_chain;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoTopLevelAwait;

const CODE: &str = "no-top-level-await";
const MESSAGE: &str = "Top level await is not allowed";

impl LintRule for NoTopLevelAwait {
  fn new() -> Arc<Self> {
    Arc::new(NoTopLevelAwait)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoTopLevelAwaitHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_top_level_await.md")
  }
}

struct NoTopLevelAwaitHandler;

impl Handler for NoTopLevelAwaitHandler {
  fn await_expr(
    &mut self,
    await_expr: &ast_view::AwaitExpr,
    ctx: &mut Context,
  ) {
    if !is_node_inside_function(await_expr) {
      ctx.add_diagnostic(await_expr.range(), CODE, MESSAGE);
    }
  }

  fn for_of_stmt(
    &mut self,
    for_of_stmt: &ast_view::ForOfStmt,
    ctx: &mut Context,
  ) {
    if_chain! {
      if for_of_stmt.await_token().is_some();
      if !is_node_inside_function(for_of_stmt);
      then {
        ctx.add_diagnostic(for_of_stmt.range(), CODE, MESSAGE)
      }
    }
  }
}

fn is_node_inside_function<'a>(node: &impl NodeTrait<'a>) -> bool {
  use deno_ast::view::Node;
  match node.parent() {
    Some(Node::FnDecl(_))
    | Some(Node::FnExpr(_))
    | Some(Node::ArrowExpr(_))
    | Some(Node::ClassMethod(_))
    | Some(Node::PrivateMethod(_)) => true,
    None => false,
    Some(n) => is_node_inside_function(&n),
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_top_level_await_valid() {
    assert_lint_ok! {
      NoTopLevelAwait,
      r#"async function foo() { await bar(); }"#,
      r#"const foo = async function () { await bar()};"#,
      r#"const foo = () => { await bar()};"#,
      r#"async function foo() { for await (item of items){}}"#,
      r#"async function foo() { await bar(); }"#,
      r#"class Foo {
        async foo() { await task(); }
        private async bar(){ await task(); }
      }"#,
      r#"const foo = { bar : async () => { await task()} }"#,
    };
  }

  #[test]
  fn no_top_level_await_invalid() {
    assert_lint_err! {
      NoTopLevelAwait,
      r#"await foo()"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
      r#"for await (item of items) {}"#: [
        {
          col: 0,
          message: MESSAGE,
        },
      ],
    };
  }
}
