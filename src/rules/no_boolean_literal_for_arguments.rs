use std::any::Any;
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::find_lhs_ids;
use crate::Program;
use deno_ast::view::{AssignExpr, CallExpr, NodeTrait};
use deno_ast::{BindingKind, SourceRanged};

#[derive(Debug)]
pub struct NoBooleanLiteralForArguments;

const CODE: &str = "no-boolean-literal-for-arguments";
const MESSAGE: &str = "Please create a self-documenting constant instead of \
passing plain boolean values as parameters";
const HINT: &str = "const ARG_ONE = true, ARG_TWO = false; yourFunction(ARG_ONE, ARG_TWO)";

impl LintRule for NoBooleanLiteralForArguments {
  fn lint_program_with_ast_view<'view>(&self, context: &mut Context<'view>, program: Program<'view>) {
    NoBooleanLiteralForArgumentsVisitor.traverse(program, context);
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    todo!()
  }
}

struct NoBooleanLiteralForArgumentsVisitor;

impl Handler for NoBooleanLiteralForArgumentsVisitor {
  fn call_expr(&mut self, call_expression: &CallExpr, ctx: &mut Context) {
    let args = call_expression.args;
    if args.len() < 2 {
      return;
    }
    let mut prev_argument_is_bool = false;
    let is_boolean = |text: &str| -> bool {
      match text {
        "true" | "false" => true,
        _ => false
      }
    };
    for arg in args {
      if prev_argument_is_bool.clone() && is_boolean(arg.text()) {
        ctx.add_diagnostic_with_hint(
         call_expression.range(),
          CODE,
          MESSAGE,
          HINT,
        );
      }
      if is_boolean(arg.text()) {
        prev_argument_is_bool = true;
      }
    }
  }
}

#[cfg(test)]
mod test {
  use super::*;

  #[test]
  fn no_boolean_literal_for_arguments_valid() {

  }

  #[test]
  fn no_boolean_literal_for_arguments_invalid() {
    assert_lint_err! {
      NoBooleanLiteralForArguments,
      r#"test(true,true)"#:[{line: 1, col: 0, message: MESSAGE, hint: HINT}],
      r#"test(false,true)"#:[{line: 1, col: 0, message: MESSAGE, hint: HINT}],
      r#"test(false,false)"#:[{line: 1, col: 0, message: MESSAGE, hint: HINT}],
    };
  }
}
