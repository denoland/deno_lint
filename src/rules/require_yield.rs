// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Function, Program, YieldExpression};

#[derive(Debug)]
pub struct RequireYield;

const CODE: &str = "require-yield";
const MESSAGE: &str = "Generator function has no `yield`";

impl LintRule for RequireYield {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = RequireYieldHandler {
      yield_stack: vec![],
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct RequireYieldHandler {
  yield_stack: Vec<u32>,
}

impl Handler<'_> for RequireYieldHandler {
  fn function(&mut self, function: &Function, _ctx: &mut Context) {
    if function.generator {
      self.yield_stack.push(0);
    }
  }

  fn function_exit(&mut self, function: &Function, ctx: &mut Context) {
    if function.generator {
      let yield_count = self.yield_stack.pop().unwrap();

      // Verify that `yield` was called only if function body is non-empty
      if let Some(body) = &function.body {
        if !body.statements.is_empty() && yield_count == 0 {
          ctx.add_diagnostic(function.span, CODE, MESSAGE);
        }
      }
    }
  }

  fn yield_expression(
    &mut self,
    _yield_expr: &YieldExpression,
    _ctx: &mut Context,
  ) {
    if let Some(last) = self.yield_stack.last_mut() {
      *last += 1;
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn require_yield_valid() {
    assert_lint_ok! {
      RequireYield,
      r#"
function foo() {}
function* bar() {
  yield "bar";
}
function* emptyBar() {}

class Fizz {
  *fizz() {
    yield "fizz";
  }

  *#buzz() {
    yield "buzz";
  }
}

const obj = {
  *foo() {
    yield "foo";
  }
};
      "#,
    };
  }

  #[test]
  fn require_yield_invalid() {
    assert_lint_err! {
      RequireYield,
      r#"function* bar() { return "bar"; }"#: [{ col: 0, message: MESSAGE }],
      r#"(function* foo() { return "foo"; })();"#: [{ col: 1, message: MESSAGE }],
      r#"function* nested() { function* gen() { yield "gen"; } }"#: [{ col: 0, message: MESSAGE }],
      r#"const obj = { *foo() { return "foo"; } };"#: [{ col: 18, message: MESSAGE }],
      r#"
class Fizz {
  *fizz() {
    return "fizz";
  }

  *#buzz() {
    return "buzz";
  }
}
    "#: [{ line: 3, col: 7, message: MESSAGE }, { line: 7, col: 8, message: MESSAGE }],
    }
  }
}
