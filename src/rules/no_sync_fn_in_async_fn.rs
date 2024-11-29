// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::StringRepr;
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::view::{CallExpr, Callee, Expr, ParenExpr, VarDeclarator};
use deno_ast::SourceRange;
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
    let mut rule = NoSyncFnInAsyncFnHandler::default();
    rule.traverse(program, context);
    rule.traverse(program, context);
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
    MemberProp::PrivateName(ident) => Some(ident.name()),
    MemberProp::Computed(prop) => match &prop.expr {
      Expr::Lit(Lit::Str(s)) => Some(s.value()),
      Expr::Ident(ident) => Some(ident.sym()),
      Expr::Tpl(Tpl { exprs, quasis, .. })
        if exprs.is_empty() && quasis.len() == 1 =>
      {
        Some(quasis[0].raw())
      }
      _ => None,
    },
  }
}

#[derive(Default)]
struct NoSyncFnInAsyncFnHandler {
  blocking_fns: Vec<String>,
}
impl NoSyncFnInAsyncFnHandler {
  fn maybe_add_diagnostic(
    &mut self,
    node: deno_ast::view::Node,
    ctx: &mut Context,
  ) {
    // if we detect one of the blocking functions inside an async context, add lint
    let node_name = node.text().to_string();
    if_chain! {
      if self.blocking_fns.contains(&node_name);
      if inside_async_fn(node);
      then {
        self.add_diagnostic(&node_name, node.range(), ctx)
      }
    }
  }

  fn add_diagnostic(
    &mut self,
    node_name: &str,
    range: SourceRange,
    ctx: &mut Context,
  ) {
    ctx.add_diagnostic_with_hint(
      range,
      CODE,
      MESSAGE,
      format!("consier changing {node_name} to an async function"),
    );
  }

  fn handle_paren_callee(&mut self, p: &ParenExpr, ctx: &mut Context) {
    match p.expr {
      Expr::Paren(paren) => self.handle_paren_callee(paren, ctx),
      Expr::Ident(ident) => {
        self.maybe_add_diagnostic(p.expr.as_node(), ctx);
      }
      Expr::Seq(seq) => {
        for expr in &seq.exprs {
          if let Expr::Ident(ident) = expr {
            self.maybe_add_diagnostic(expr.as_node(), ctx);
          }
        }
      }
      _ => {}
    }
  }
}

impl Handler for NoSyncFnInAsyncFnHandler {
  fn member_expr(
    &mut self,
    member_expr: &ast_view::MemberExpr,
    ctx: &mut Context,
  ) {
    // do not check chained member expressions (e.g. `foo.bar.baz`)
    if member_expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use deno_ast::view::Expr;

    // if we're calling a deno Sync api inside a function
    // add that function to blocking functions list
    if_chain! {
      if let Expr::Ident(obj) = &member_expr.obj;
      if ctx.scope().is_global(&obj.inner.to_id());
      let obj_symbol: &str = obj.sym();
      if obj_symbol == "Deno";
      if let Some(prop_symbol) = extract_symbol(&member_expr.prop);
      if prop_symbol.strip_suffix("Sync").is_some();
      if let Some(sync_fn) = inside_sync_fn(member_expr.as_node());
      then {
        self.blocking_fns.push(sync_fn);
      }
    }

    // if we detect deno sync api in an async context add lint
    if_chain! {
      if let Expr::Ident(obj) = &member_expr.obj;
      if ctx.scope().is_global(&obj.inner.to_id());
      let obj_symbol: &str = obj.sym();
      if obj_symbol == "Deno";
      if let Some(prop_symbol) = extract_symbol(&member_expr.prop);
      if let Some(async_name) = prop_symbol.strip_suffix("Sync");
      if inside_async_fn(member_expr.as_node());
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.range(),
          CODE,
          MESSAGE,
          format!("Consider changing this to an async equivalent: `await Deno.{}(..)`",
            async_name),
        );
      }
    }
  }

  fn var_declarator(&mut self, v: &VarDeclarator, ctx: &mut Context) {
    if let Some(expr) = &v.init {
      if let Expr::Ident(ident) = expr {
        self.maybe_add_diagnostic(expr.as_node(), ctx);
      }
    }
  }

  fn call_expr(&mut self, call_expr: &CallExpr, ctx: &mut Context) {
    if let Callee::Expr(expr) = &call_expr.callee {
      match expr {
        Expr::Ident(ident) => self.maybe_add_diagnostic(expr.as_node(), ctx),
        Expr::Paren(paren) => self.handle_paren_callee(paren, ctx),
        _ => {}
      }
    }
  }
}

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

fn inside_sync_fn(node: ast_view::Node) -> Option<String> {
  use deno_ast::view::Node::*;
  match node {
    FnDecl(decl) if !decl.function.is_async() => Some(decl.ident.text().into()),
    FnExpr(decl) if !decl.function.is_async() => {
      decl.ident.map(|id| id.text().into())
    }
    _ => {
      let parent = match node.parent() {
        Some(p) => p,
        None => return None,
      };
      inside_sync_fn(parent)
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_sync_fn_in_async_fn_fails_nested() {
    // both of these should panic
    // TODO switch to assert_err
    assert_lint_ok! {
     NoSyncFnInAsyncFn,
           r#"
      async function foo2() {
        foo()
      }
      function foo() {
        Deno.readTextFileSync("");
      }"#
    }

    assert_lint_ok! {
    NoSyncFnInAsyncFn,
          r#"
      function foo() {
        Deno.readTextFileSync("");
      }"
      async function foo2() {
        foo()
      }"#
    }
  }

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
