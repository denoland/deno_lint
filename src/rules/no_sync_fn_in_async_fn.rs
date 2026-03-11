// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, ComputedMemberExpression, Expression, Function,
  PrivateFieldExpression, Program, StaticMemberExpression,
};

#[derive(Debug)]
pub struct NoSyncFnInAsyncFn;

const CODE: &str = "no-sync-fn-in-async-fn";
const MESSAGE: &str =
  "Sync fn is used inside an async fn, this blocks deno event loop";

impl LintRule for NoSyncFnInAsyncFn {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoSyncFnInAsyncFnHandler {
      async_fn_depth: 0,
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

/// Extracts a property name from a static member expression property.
fn extract_static_prop_name<'a>(
  member: &'a StaticMemberExpression<'a>,
) -> Option<&'a str> {
  Some(member.property.name.as_str())
}

/// Extracts a property name from a computed member expression.
fn extract_computed_prop_name<'a>(
  member: &'a ComputedMemberExpression<'a>,
) -> Option<&'a str> {
  match &member.expression {
    Expression::StringLiteral(s) => Some(s.value.as_str()),
    Expression::Identifier(ident) => Some(ident.name.as_str()),
    Expression::TemplateLiteral(tpl)
      if tpl.expressions.is_empty() && tpl.quasis.len() == 1 =>
    {
      Some(tpl.quasis[0].value.raw.as_str())
    }
    _ => None,
  }
}

fn check_deno_sync_call(
  obj: &Expression,
  prop_symbol: Option<&str>,
  span: deno_ast::oxc::span::Span,
  async_fn_depth: u32,
  ctx: &mut Context,
) {
  if async_fn_depth == 0 {
    return;
  }

  if let Expression::Identifier(ident) = obj {
    if ident.name.as_str() != "Deno" {
      return;
    }

    if let Some(prop) = prop_symbol {
      if let Some(async_name) = prop.strip_suffix("Sync") {
        ctx.add_diagnostic_with_hint(
          span,
          CODE,
          MESSAGE,
          format!(
            "Consider changing this to an async equivalent: `await Deno.{}(..)`",
            async_name
          ),
        );
      }
    }
  }
}

struct NoSyncFnInAsyncFnHandler {
  async_fn_depth: u32,
}

impl Handler<'_> for NoSyncFnInAsyncFnHandler {
  fn function(&mut self, n: &Function, _ctx: &mut Context) {
    if n.r#async {
      self.async_fn_depth += 1;
    }
  }

  fn function_exit(&mut self, n: &Function, _ctx: &mut Context) {
    if n.r#async {
      self.async_fn_depth -= 1;
    }
  }

  fn arrow_function_expression(
    &mut self,
    n: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    if n.r#async {
      self.async_fn_depth += 1;
    }
  }

  fn arrow_function_expression_exit(
    &mut self,
    n: &ArrowFunctionExpression,
    _ctx: &mut Context,
  ) {
    if n.r#async {
      self.async_fn_depth -= 1;
    }
  }

  fn static_member_expression(
    &mut self,
    member: &StaticMemberExpression,
    ctx: &mut Context,
  ) {
    // Skip chained member expressions (e.g. `foo.bar.baz` — only check the outermost)
    if matches!(&member.object, Expression::StaticMemberExpression(_)
      | Expression::ComputedMemberExpression(_)
      | Expression::PrivateFieldExpression(_))
    {
      return;
    }

    let prop = extract_static_prop_name(member);
    check_deno_sync_call(
      &member.object,
      prop,
      member.span,
      self.async_fn_depth,
      ctx,
    );
  }

  fn computed_member_expression(
    &mut self,
    member: &ComputedMemberExpression,
    ctx: &mut Context,
  ) {
    if matches!(&member.object, Expression::StaticMemberExpression(_)
      | Expression::ComputedMemberExpression(_)
      | Expression::PrivateFieldExpression(_))
    {
      return;
    }

    let prop = extract_computed_prop_name(member);
    check_deno_sync_call(
      &member.object,
      prop,
      member.span,
      self.async_fn_depth,
      ctx,
    );
  }

  fn private_field_expression(
    &mut self,
    member: &PrivateFieldExpression,
    ctx: &mut Context,
  ) {
    if matches!(&member.object, Expression::StaticMemberExpression(_)
      | Expression::ComputedMemberExpression(_)
      | Expression::PrivateFieldExpression(_))
    {
      return;
    }

    let prop = Some(member.field.name.as_str());
    check_deno_sync_call(
      &member.object,
      prop,
      member.span,
      self.async_fn_depth,
      ctx,
    );
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
