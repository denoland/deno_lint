// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::{view as ast_view, SourceRanged};
use if_chain::if_chain;

#[derive(Debug)]
pub struct NoSyncFnInAsyncFn;

const CODE: &str = "no-sync-fn-in-async-fn";
const MESSAGE: &str =
  "Sync fn is used inside an async fn, this blocks deno event loop";

impl LintRule for NoSyncFnInAsyncFn {
  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoSyncFnInAsyncFnHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_sync_fn_in_async_fn.md")
  }
}

/// Extracts a symbol from the given member prop if the symbol is statically determined (otherwise,
/// return `None`).
fn extract_symbol<'a>(
  member_prop: &'a ast_view::MemberProp,
) -> Option<&'a str> {
  use deno_ast::view::{Expr, Lit, MemberProp, Tpl};
  match member_prop {
    MemberProp::Ident(ident) => Some(ident.sym()),
    MemberProp::PrivateName(ident) => Some(ident.id.sym()),
    MemberProp::Computed(prop) => match &prop.expr {
      Expr::Lit(Lit::Str(s)) => Some(s.value()),
      Expr::Ident(ident) => Some(ident.sym()),
      Expr::Tpl(Tpl {
        ref exprs,
        ref quasis,
        ..
      }) if exprs.is_empty() && quasis.len() == 1 => Some(quasis[0].raw()),
      _ => None,
    },
  }
}

struct NoSyncFnInAsyncFnHandler;

impl Handler for NoSyncFnInAsyncFnHandler {
  fn member_expr(
    &mut self,
    member_expr: &ast_view::MemberExpr,
    ctx: &mut Context,
  ) {
    fn inside_async_fn(node: ast_view::Node) -> bool {
      use deno_ast::view::Node::*;
      match node {
        FnDecl(decl) => decl.function.is_async(),
        FnExpr(decl) => decl.function.is_async(),
        ArrowExpr(decl) => decl.is_async(),
        _ => {
          let parent = match node.parent() {
            Some(p) => p,
            None => return false,
          };
          inside_async_fn(parent)
        }
      }
    }

    // Not check chained member expressions (e.g. `foo.bar.baz`)
    if member_expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use deno_ast::view::Expr;
    if_chain! {
      if let Expr::Ident(obj) = &member_expr.obj;
      if ctx.scope().is_global(&obj.inner.to_id());
      let obj_symbol: &str = obj.sym();
      if let Some(prop_symbol) = extract_symbol(&member_expr.prop);
      if obj_symbol == "Deno";
      if prop_symbol.contains("Sync");
      if inside_async_fn(member_expr.as_node());
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.range(),
          CODE,
          MESSAGE,
          format!("Consider changing this to an async equivalent: `await Deno.{}(..)`",
            prop_symbol.strip_suffix("Sync").expect("exists")),
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_sync_fn_in_async_fn_is_valid() {
    assert_lint_ok! {
    NoSyncFnInAsyncFn,
          r#"
      function foo(things) {
        Deno.readTextFileSync("");
      }
      "#,
          r#"
      const foo = (things) => {
        Deno.readTextFileSync("");
      }
      "#,
          r#"
      const foo = function(things) {
        Deno.readTextFileSync("");
      }
      "#,
          r#"
      class Foo {
        foo(things) {
          Deno.readTextFileSync("");
        }
      }
      "#,
        }
  }

  #[test]
  fn no_sync_fn_in_async_fn_is_invalid() {
    assert_lint_err! {
      NoSyncFnInAsyncFn,
      MESSAGE,
      "Consider changing this to an async equivalent: `await Deno.readTextFile(..)`",
      r#"
      async function foo(things) {
        Deno.readTextFileSync("");
      }
      "#: [{ line: 3, col: 8 }],
      r#"
      const foo = async (things) => {
        Deno.readTextFileSync("");
      }
      "#: [{ line: 3, col: 8 }],
      r#"
      const foo = async function (things) {
        Deno.readTextFileSync("");
      }
      "#: [{ line: 3, col: 8 }],
    }
  }
}
