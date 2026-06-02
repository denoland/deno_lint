// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use derive_more::Display;

#[derive(Debug)]
pub struct NoConstantCondition;

const CODE: &str = "no-constant-condition";

#[derive(Display)]
enum NoConstantConditionMessage {
  #[display(
    fmt = "Use of a constant expressions as conditions is not allowed."
  )]
  Unexpected,
}

#[derive(Display)]
enum NoConstantConditionHint {
  #[display(fmt = "Remove the constant expression")]
  Remove,
}

impl LintRule for NoConstantCondition {
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
    let mut handler = NoConstantConditionHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoConstantConditionHandler;

/// Represents a logical operator for short-circuit checks.
#[derive(Clone, Copy, PartialEq, Eq)]
enum LogicalOp {
  Or,
  And,
}

impl NoConstantConditionHandler {
  fn is_constant(
    node: &Expression,
    parent_node: Option<&Expression>,
    in_boolean_position: bool,
  ) -> bool {
    match node {
      Expression::StringLiteral(_)
      | Expression::NumericLiteral(_)
      | Expression::BooleanLiteral(_)
      | Expression::NullLiteral(_)
      | Expression::BigIntLiteral(_)
      | Expression::RegExpLiteral(_)
      | Expression::ArrowFunctionExpression(_)
      | Expression::FunctionExpression(_)
      | Expression::ObjectExpression(_) => true,
      Expression::TemplateLiteral(tpl) => {
        (in_boolean_position
          && tpl.quasis.iter().any(|quasi| match &quasi.value.cooked {
            Some(str) => !str.is_empty(),
            None => false,
          }))
          || tpl.expressions.iter().all(|expr| {
            Self::is_constant(expr, parent_node, in_boolean_position)
          })
      }
      Expression::ParenthesizedExpression(paren) => {
        Self::is_constant(&paren.expression, Some(node), false)
      }
      Expression::ArrayExpression(arr) => match parent_node {
        Some(Expression::BinaryExpression(bin)) => {
          if bin.operator == BinaryOperator::Addition {
            arr.elements.iter().all(|element| match element {
              ArrayExpressionElement::Elision(_) => true,
              ArrayExpressionElement::SpreadElement(spread) => {
                // A spread element is only constant if its argument is constant
                Self::is_constant(&spread.argument, parent_node, false)
              }
              _ => {
                let expr = element.to_expression();
                Self::is_constant(expr, parent_node, false)
              }
            })
          } else {
            true
          }
        }
        _ => true,
      },
      Expression::UnaryExpression(unary) => {
        if unary.operator == UnaryOperator::Void {
          true
        } else {
          (unary.operator == UnaryOperator::Typeof && in_boolean_position)
            || Self::is_constant(&unary.argument, Some(node), true)
        }
      }
      Expression::BinaryExpression(bin) => {
        if bin.operator != BinaryOperator::In {
          Self::is_constant(&bin.left, Some(node), false)
            && Self::is_constant(&bin.right, Some(node), false)
        } else {
          false
        }
      }
      Expression::LogicalExpression(log) => {
        let is_left_constant =
          Self::is_constant(&log.left, Some(node), in_boolean_position);
        let is_right_constant =
          Self::is_constant(&log.right, Some(node), in_boolean_position);
        let log_op = match log.operator {
          LogicalOperator::Or => LogicalOp::Or,
          LogicalOperator::And => LogicalOp::And,
          LogicalOperator::Coalesce => {
            return is_left_constant && is_right_constant;
          }
        };
        let is_left_short_circuit =
          is_left_constant && check_short_circuit(&log.left, log_op);
        let is_right_short_circuit =
          is_right_constant && check_short_circuit(&log.right, log_op);
        (is_left_constant && is_right_constant)
          || is_left_short_circuit
          || is_right_short_circuit
      }
      Expression::AssignmentExpression(assign) => {
        assign.operator == AssignmentOperator::Assign
          && Self::is_constant(&assign.right, Some(node), in_boolean_position)
      }
      Expression::SequenceExpression(seq) => {
        if let Some(last) = seq.expressions.last() {
          Self::is_constant(last, Some(node), in_boolean_position)
        } else {
          false
        }
      }
      _ => false,
    }
  }

  fn report(condition: &Expression, ctx: &mut Context) {
    if Self::is_constant(condition, None, true) {
      use deno_ast::oxc::span::GetSpan;
      ctx.add_diagnostic_with_hint(
        condition.span(),
        CODE,
        NoConstantConditionMessage::Unexpected,
        NoConstantConditionHint::Remove,
      );
    }
  }
}

fn check_short_circuit(expr: &Expression, operator: LogicalOp) -> bool {
  match expr {
    Expression::BooleanLiteral(boolean) => {
      (operator == LogicalOp::Or && boolean.value)
        || (operator == LogicalOp::And && !boolean.value)
    }
    Expression::StringLiteral(_)
    | Expression::NumericLiteral(_)
    | Expression::NullLiteral(_)
    | Expression::BigIntLiteral(_)
    | Expression::RegExpLiteral(_) => false,
    Expression::UnaryExpression(unary) => {
      operator == LogicalOp::And && unary.operator == UnaryOperator::Void
    }
    Expression::LogicalExpression(log) => {
      let log_op = match log.operator {
        LogicalOperator::Or => LogicalOp::Or,
        LogicalOperator::And => LogicalOp::And,
        LogicalOperator::Coalesce => return false,
      };
      check_short_circuit(&log.left, log_op)
        || check_short_circuit(&log.right, log_op)
    }
    _ => false,
  }
}

impl Handler<'_> for NoConstantConditionHandler {
  fn conditional_expression(
    &mut self,
    cond_expr: &ConditionalExpression,
    ctx: &mut Context,
  ) {
    Self::report(&cond_expr.test, ctx);
  }

  fn if_statement(&mut self, if_stmt: &IfStatement, ctx: &mut Context) {
    Self::report(&if_stmt.test, ctx);
  }

  /* TODO(bartlomieju): temporarly disabled because
    deno_std uses while (true) {} loops
  fn while_statement(
    &mut self,
    while_stmt: &WhileStatement,
    ctx: &mut Context,
  ) {
    Self::report(&while_stmt.test, ctx);
  }

  fn do_while_statement(
    &mut self,
    do_while_stmt: &DoWhileStatement,
    ctx: &mut Context,
  ) {
    Self::report(&do_while_stmt.test, ctx);
  }

  fn for_statement(
    &mut self,
    for_stmt: &ForStatement,
    ctx: &mut Context,
  ) {
    if let Some(cond) = &for_stmt.test {
      Self::report(cond, ctx);
    }
  }
  */
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_constant_condition_valid() {
    assert_lint_ok! {
      NoConstantCondition,
      r#"if(a);"#,
      r#"if(a == 0);"#,
      r#"if(a = f());"#,
      r#"if(1, a);"#,
      r#"if ('every' in []);"#,
      "if (`\\\n${a}`) {}",
      r#"if (`${a}`);"#,
      r#"if (`${foo()}`);"#,
      r#"if (`${a === 'b' && b==='a'}`);"#,
      r#"if (`foo${a}` === 'fooa');"#,
      r#"if (tag`a`);"#,
      r#"if (tag`${a}`);"#,
      r#"while(~!a);"#,
      r#"while(a = b);"#,
      r#"while(`${a}`);"#,
      r#"for(;x < 10;);"#,
      r#"for(;;);"#,
      r#"for(;`${a}`;);"#,
      r#"do{ }while(x)"#,
      r#"q > 0 ? 1 : 2;"#,
      r#"`${a}` === a ? 1 : 2"#,
      r#"`foo${a}` === a ? 1 : 2"#,
      r#"tag`a` === a ? 1 : 2"#,
      r#"tag`${a}` === a ? 1 : 2"#,
      r#"while(x += 3) {}"#,
      r#"while(tag`a`) {}"#,
      r#"while(tag`${a}`) {}"#,
      "while(`\\\n${a}`) {}",

      // typeof conditions
      r#"if(typeof x === 'undefined'){}"#,
      r#"if(`${typeof x}` === 'undefined'){}"#,
      r#"if(a === 'str' && typeof b){}"#,
      "typeof a == typeof b",
      "typeof 'a' === 'string'|| typeof b === 'string'",
      "`${typeof 'a'}` === 'string'|| `${typeof b}` === 'string'",

      // void conditions
      r#"if (a || void a);"#,
      r#"if (void a || a);"#,

      // string literals
      r#"if('str' || a){}"#,
      r#"if('str1' && a){}"#,
      r#"if(a && 'str'){}"#,
      r#"if('str' || abc==='str'){}"#,
      r#"if ((foo || 'bar') === 'baz') {}"#,
      r#"if ((foo || 'bar') !== 'baz') {}"#,
      r#"if ((foo || 'bar') == 'baz') {}"#,
      r#"if ((foo || 'bar') != 'baz') {}"#,
      r#"if ((foo || 233) > 666) {}"#,
      r#"if ((foo || 233) < 666) {}"#,
      r#"if ((foo || 233) >= 666) {}"#,
      r#"if ((foo || 233) <= 666) {}"#,
      r#"if ((key || 'k') in obj) {}"#,
      r#"if ((foo || {}) instanceof obj) {}"#,
      r#"if ('' + [y] === '' + [ty]) {}"#,
      r#"if ('a' === '' + [ty]) {}"#,
      r#"if ('' + [y, m, d] === 'a') {}"#,
      r#"if ('' + [y, 'm'] === '' + [ty, 'tm']) {}"#,
      r#"if ('' + [y, 'm'] === '' + ['ty']) {}"#,
      r#"if ([,] in

        ($2))
         ;
         else
          ;"#,
      r#"if ([...x]+'' === 'y'){}"#,
      r#"for(;true;);"#,
      r#"for(;``;);"#,
      r#"for(;`foo`;);"#,
      r#"for(;`foo${bar}`;);"#,
      r#"do{}while(true)"#,
      r#"do{}while(t = -2)"#,
      r#"do{}while(``)"#,
      r#"do{}while(`foo`)"#,
      r#"do{}while(`foo${bar}`)"#,
      r#"while([]);"#,
      r#"while(~!0);"#,
      r#"while(x = 1);"#,
      r#"while(function(){});"#,
      r#"while(true);"#,
      r#"while(() => {});"#,
      r#"while(`foo`);"#,
      r#"while(``);"#,
      r#"while(`${'foo'}`);"#,
      r#"while(`${'foo' + 'bar'}`);"#,
      r#"while(typeof x){}"#,

      // nested
      r#"if (foo) { if (bar) {} }"#,
      r#"foo ? bar ? 1 : 2 : 3"#,
    };
  }

  #[test]
  fn no_constant_condition_invalid() {
    assert_lint_err! {
      NoConstantCondition,
      r"true ? 1 : 2;": [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"q = 0 ? 1 : 2;": [
        {
          col: 4,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"(q = 0) ? 1 : 2;": [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"`` ? 1 : 2;": [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"`foo` ? 1 : 2;": [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"`foo${bar}` ? 1 : 2;": [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(-2);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(true);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if({});": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(0 < 1);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(0 || 1);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(a, 1);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(`foo`);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(``);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r"if(`\\\n`);": [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(`${'bar'}`);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(`${'bar' + `foo`}`);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(`foo${false || true}`);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(`foo${bar}`);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(`${bar}foo`);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],

      // typeof conditions
      r#"if(typeof x){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(`${typeof x}`){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(`${''}${typeof x}`){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(typeof 'abc' === 'string'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(a = typeof b){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(a, typeof b){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(typeof 'a' == 'string' || typeof 'b' == 'string'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],

      // void conditions
      r#"if(1 || void x);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(void x);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(y = void x);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(x, void x);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(void x === void y);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(void x && a);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(a && void x);"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(false && abc==='str'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(true || abc==='str'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(abc==='str' || true){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(abc==='str' || true || def ==='str'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(false || true){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(typeof abc==='str' || true){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],

      // string literals
      r#"if('str1' || 'str2'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if('str1' && 'str2'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(+1) {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if([a]) {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if([]) {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(''+['a']) {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(''+[]) {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if([a]==[a]) {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if([a] - '') {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(+[a]) {}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],

      // nested
      r#"if (foo) { if (true) {} }"#: [
        {
          col: 15,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if (foo) {} else if (true) {}"#: [
        {
          col: 21,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if (foo) {} else if (bar) {} else if (true) {}"#: [
        {
          col: 38,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if (foo) {} else { if (true) {} }"#: [
        {
          col: 23,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"foo ? true ? 1 : 2 : 3"#: [
        {
          col: 6,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ]
    };
  }

  // TODO(humancalico) make these tests pass
  #[test]
  #[ignore]
  fn should_pass_valid() {
    assert_lint_ok! {
      NoConstantCondition,

      // TODO(humancalico) more conditions should be added to pass these cases
      // https://github.com/eslint/eslint/blob/f4d7b9e1a599346b2f21ff9de003b311b51411e6/lib/rules/no-constant-condition.js#L135-L146
      r#"if(xyz === 'str1' && abc==='str2'){}"#,
      r#"if(xyz === 'str1' || abc==='str2'){}"#,
      r#"if(xyz === 'str1' || abc==='str2' && pqr === 5){}"#,
      r#"if(typeof abc === 'string' && abc==='str2'){}"#,
      r#"if(false || abc==='str'){}"#,
      r#"if(true && abc==='str'){}"#,
      r#"if(typeof 'str' && abc==='str'){}"#,
      r#"if(abc==='str' || false || def ==='str'){}"#,
      r#"if(true && abc==='str' || def ==='str'){}"#,
      r#"if(true && typeof abc==='string'){}"#,

      // TODO(humancalico) add a configuration option for { checkLoops: false }
      // https://eslint.org/docs/rules/no-constant-condition#checkloops
      r#"while(true);"#,
      r#"for(;true;);"#,
      r#"do{}while(true)"#,
      r#"function* foo(){while(true){yield 'foo';}}"#,
      r#"function* foo(){for(;true;){yield 'foo';}}"#,
      r#"function* foo(){do{yield 'foo';}while(true)}"#,
      r#"function* foo(){while (true) { while(true) {yield;}}}"#,
      r#"function* foo() {for (; yield; ) {}}"#,
      r#"function* foo() {for (; ; yield) {}}"#,
      r#"function* foo() {while (true) {function* foo() {yield;}yield;}}"#,
      r#"function* foo() { for (let x = yield; x < 10; x++) {yield;}yield;}"#,
      r#"function* foo() { for (let x = yield; ; x++) { yield; }}"#,
    };
  }

  // TODO(humancalico) make these tests pass
  #[test]
  #[ignore]
  fn should_pass_invalid() {
    assert_lint_err! {
      NoConstantCondition,

      // TODO(humancalico) more conditions should be added to pass these cases
      // https://github.com/eslint/eslint/blob/f4d7b9e1a599346b2f21ff9de003b311b51411e6/lib/rules/no-constant-condition.js#L135-L146
      r#"if(abc==='str' || 'str'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if(a || 'str'){}"#: [
        {
          col: 3,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function* foo(){while(true){} yield 'foo';}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function* foo(){while(true){if (true) {yield 'foo';}}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function* foo(){while(true){yield 'foo';} while(true) {}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"var a = function* foo(){while(true){} yield 'foo';}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"while (true) { function* foo() {yield;}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function* foo(){if (true) {yield 'foo';}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function* foo() {for (let foo = yield; true;) {}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function* foo() {for (foo = yield; true;) {}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function foo() {while (true) {function* bar() {while (true) {yield;}}}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function foo() {while (true) {const bar = function*() {while (true) {yield;}}}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"function* foo() { for (let foo = 1 + 2 + 3 + (yield); true; baz) {}}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ],
      r#"if ([,] + ''){}"#: [
        {
          col: 0,
          message: NoConstantConditionMessage::Unexpected,
          hint: NoConstantConditionHint::Remove,
        }
      ]
    };
  }
}
