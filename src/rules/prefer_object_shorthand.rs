// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::Program;

use deno_ast::view::{Expr, FnExpr, KeyValueProp, PropName};
use deno_ast::SourceRanged;
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::Regex;

const CODE: &str = "prefer-object-shorthand";
const PROPERTY_FIX_DESC: &str = "Use property shorthand syntax";
const METHOD_FIX_DESC: &str = "Use method shorthand syntax";

static FUNCTION_EXPR_REGEX: Lazy<Regex> = Lazy::new(|| {
  Regex::new(r"^(?:async\s+)?function(?:\s*\*)?(?P<tail>[\s\S]*)$").unwrap()
});

#[derive(Display)]
enum PreferObjectShorthandMessage {
  #[display(fmt = "Expected property shorthand syntax")]
  ExpectedPropertyShorthand,
  #[display(fmt = "Expected method shorthand syntax")]
  ExpectedMethodShorthand,
}

#[derive(Display)]
enum PreferObjectShorthandHint {
  #[display(fmt = "Use property shorthand syntax")]
  UsePropertyShorthand,
  #[display(fmt = "Use method shorthand syntax")]
  UseMethodShorthand,
}

#[derive(Debug)]
pub struct PreferObjectShorthand;

impl LintRule for PreferObjectShorthand {
  fn tags(&self) -> Tags {
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
    PreferObjectShorthandHandler.traverse(program, context);
  }
}

struct PreferObjectShorthandHandler;

impl PreferObjectShorthandHandler {
  fn add_diagnostic(
    &self,
    prop: &KeyValueProp,
    message: impl ToString,
    hint: impl ToString,
    fix_desc: &'static str,
    new_text: String,
    ctx: &mut Context,
  ) {
    ctx.add_diagnostic_with_fixes(
      prop.range(),
      CODE,
      message,
      Some(hint.to_string()),
      vec![LintFix {
        description: fix_desc.into(),
        changes: vec![LintFixChange {
          new_text: new_text.into(),
          range: prop.range(),
        }],
      }],
    );
  }

  fn check_property_shorthand(
    &self,
    prop: &KeyValueProp,
    ctx: &mut Context,
  ) -> bool {
    let PropName::Ident(key) = prop.key else {
      return false;
    };

    let Expr::Ident(value) = prop.value else {
      return false;
    };

    if key.sym() != value.sym() {
      return false;
    }

    let key_text = key.range().text_fast(ctx.text_info()).to_string();
    self.add_diagnostic(
      prop,
      PreferObjectShorthandMessage::ExpectedPropertyShorthand,
      PreferObjectShorthandHint::UsePropertyShorthand,
      PROPERTY_FIX_DESC,
      key_text,
      ctx,
    );
    true
  }

  fn check_method_shorthand(
    &self,
    prop: &KeyValueProp,
    fn_expr: &FnExpr,
    ctx: &mut Context,
  ) {
    if fn_expr.ident.is_some() {
      return;
    }

    let key_text = prop.key.range().text_fast(ctx.text_info());
    let value_text = prop.value.range().text_fast(ctx.text_info());
    let Some(captures) = FUNCTION_EXPR_REGEX.captures(value_text) else {
      return;
    };
    let Some(tail) = captures.name("tail") else {
      return;
    };

    let mut method_text = String::new();
    if fn_expr.function.is_async() {
      method_text.push_str("async ");
    }
    if fn_expr.function.is_generator() {
      method_text.push('*');
    }
    method_text.push_str(key_text);
    method_text.push_str(tail.as_str());

    self.add_diagnostic(
      prop,
      PreferObjectShorthandMessage::ExpectedMethodShorthand,
      PreferObjectShorthandHint::UseMethodShorthand,
      METHOD_FIX_DESC,
      method_text,
      ctx,
    );
  }
}

impl Handler for PreferObjectShorthandHandler {
  fn key_value_prop(&mut self, prop: &KeyValueProp, ctx: &mut Context) {
    if self.check_property_shorthand(prop, ctx) {
      return;
    }

    let Expr::Fn(fn_expr) = prop.value else {
      return;
    };

    self.check_method_shorthand(prop, &fn_expr, ctx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn prefer_object_shorthand_valid() {
    assert_lint_ok! {
      PreferObjectShorthand,
      "({ foo });",
      "({ foo: bar });",
      "({ 'foo': foo });",
      "({ [foo]: foo });",
      "({ foo() {} });",
      "({ [foo]() {} });",
      "({ foo: function foo() {} });",
      "({ foo: () => {} });",
    };
  }

  #[test]
  fn prefer_object_shorthand_invalid() {
    assert_lint_err! {
      PreferObjectShorthand,
      "({ foo: foo });": [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedPropertyShorthand,
          hint: PreferObjectShorthandHint::UsePropertyShorthand,
          fix: (PROPERTY_FIX_DESC, "({ foo });"),
        }
      ],
      "({ foo: function() {} });": [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedMethodShorthand,
          hint: PreferObjectShorthandHint::UseMethodShorthand,
          fix: (METHOD_FIX_DESC, "({ foo() {} });"),
        }
      ],
      "({ foo: async function() {} });": [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedMethodShorthand,
          hint: PreferObjectShorthandHint::UseMethodShorthand,
          fix: (METHOD_FIX_DESC, "({ async foo() {} });"),
        }
      ],
      "({ foo: async function*() {} });": [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedMethodShorthand,
          hint: PreferObjectShorthandHint::UseMethodShorthand,
          fix: (METHOD_FIX_DESC, "({ async *foo() {} });"),
        }
      ],
      "({ foo: function*() {} });": [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedMethodShorthand,
          hint: PreferObjectShorthandHint::UseMethodShorthand,
          fix: (METHOD_FIX_DESC, "({ *foo() {} });"),
        }
      ],
      r#"({ "foo": function() {} });"#: [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedMethodShorthand,
          hint: PreferObjectShorthandHint::UseMethodShorthand,
          fix: (METHOD_FIX_DESC, r#"({ "foo"() {} });"#),
        }
      ],
      "({ [foo]: function() {} });": [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedMethodShorthand,
          hint: PreferObjectShorthandHint::UseMethodShorthand,
          fix: (METHOD_FIX_DESC, "({ [foo]() {} });"),
        }
      ],
      "({ foo: foo, bar: function() {} });": [
        {
          col: 3,
          message: PreferObjectShorthandMessage::ExpectedPropertyShorthand,
          hint: PreferObjectShorthandHint::UsePropertyShorthand,
          fix: (PROPERTY_FIX_DESC, "({ foo, bar: function() {} });"),
        },
        {
          col: 13,
          message: PreferObjectShorthandMessage::ExpectedMethodShorthand,
          hint: PreferObjectShorthandHint::UseMethodShorthand,
          fix: (METHOD_FIX_DESC, "({ foo: foo, bar() {} });"),
        }
      ],
    };
  }
}
