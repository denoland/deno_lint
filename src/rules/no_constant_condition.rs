// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;

use swc_common::Span;
use swc_common::Spanned;
use swc_ecmascript::ast::{
  BinaryOp, CondExpr, Expr, IfStmt, Lit, Module, UnaryOp,
};
use swc_ecmascript::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};

pub struct NoConstantCondition;

impl LintRule for NoConstantCondition {
  fn new() -> Box<Self> {
    Box::new(NoConstantCondition)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-constant-condition"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = NoConstantConditionVisitor::new(context);
    module.visit_all_with(module, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of a constant expression in conditional test

Using a constant expression in a conditional test is often either a mistake or a
temporary situation introduced during development and is not ready for production.

### Invalid:
```typescript
if (true) {}
if (2) {}
do {} while (x = 2);  // infinite loop
```

### Valid:
```typescript
if (x) {}
if (x === 0) {}
do {} while (x === 2);
```
"#
  }
}

struct NoConstantConditionVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoConstantConditionVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      "no-constant-condition",
      "Use of a constant expressions as conditions is not allowed.",
      "Remove the constant expression",
    );
  }

  fn check_short_circuit(&self, expr: &Expr, operator: BinaryOp) -> bool {
    match expr {
      Expr::Lit(lit) => match lit {
        Lit::Bool(boolean) => {
          (operator == BinaryOp::LogicalOr && boolean.value)
            || (operator == BinaryOp::LogicalAnd && !boolean.value)
        }
        _ => false,
      },
      Expr::Unary(unary) => {
        operator == BinaryOp::LogicalAnd && unary.op == UnaryOp::Void
      }
      Expr::Bin(bin)
        if bin.op == BinaryOp::LogicalAnd || bin.op == BinaryOp::LogicalOr =>
      {
        self.check_short_circuit(&bin.left, bin.op)
          || self.check_short_circuit(&bin.right, bin.op)
      }
      _ => false,
    }
  }

  fn is_constant(
    &self,
    node: &Expr,
    parent_node: Option<&Expr>,
    in_boolean_position: bool,
  ) -> bool {
    match node {
      Expr::Lit(_) | Expr::Arrow(_) | Expr::Fn(_) | Expr::Object(_) => true,
      Expr::Tpl(tpl) => {
        (in_boolean_position
          && tpl.quasis.iter().any(|quasi| match &quasi.cooked {
            Some(str) => !str.is_empty(),
            None => false,
          }))
          || tpl.exprs.iter().all(|expr| {
            self.is_constant(expr, parent_node, in_boolean_position)
          })
      }
      // TODO(humancalico) confirm in_boolean_position here
      Expr::Paren(paren) => self.is_constant(&paren.expr, Some(node), false),
      Expr::Array(arr) => match parent_node {
        Some(Expr::Bin(bin)) => {
          if bin.op == BinaryOp::Add {
            arr.elems.iter().all(|element| {
              self.is_constant(
                &element.as_ref().unwrap().expr,
                parent_node,
                false,
              )
            })
          } else {
            true
          }
        }
        _ => true,
      },
      Expr::Unary(unary) => {
        if unary.op == UnaryOp::Void {
          true
        } else {
          (unary.op == UnaryOp::TypeOf && in_boolean_position)
            || self.is_constant(&unary.arg, Some(node), true)
        }
      }
      Expr::Bin(bin) => {
        // This is for LogicalExpression
        if bin.op == BinaryOp::LogicalOr || bin.op == BinaryOp::LogicalAnd {
          let is_left_constant =
            self.is_constant(&bin.left, Some(node), in_boolean_position);
          let is_right_constant =
            self.is_constant(&bin.right, Some(node), in_boolean_position);
          let is_left_short_circuit =
            is_left_constant && self.check_short_circuit(&bin.left, bin.op);
          let is_right_short_circuit =
            is_right_constant && self.check_short_circuit(&bin.right, bin.op);
          (is_left_constant && is_right_constant)
          // TODO(humancalico) add more condiitons here from https://github.com/eslint/eslint/blob/f4d7b9e1a599346b2f21ff9de003b311b51411e6/lib/rules/no-constant-condition.js#L135-L146
            || is_left_short_circuit
            || is_right_short_circuit
        }
        // These are fo regular BinaryExpression
        else if bin.op != BinaryOp::In {
          self.is_constant(&bin.left, Some(node), false)
            && self.is_constant(&bin.right, Some(node), false)
        } else {
          false
        }
      }
      Expr::Assign(assign) => {
        assign.op == swc_ecmascript::ast::AssignOp::Assign
          && self.is_constant(&assign.right, Some(node), in_boolean_position)
      }
      Expr::Seq(seq) => self.is_constant(
        &seq.exprs[seq.exprs.len() - 1],
        Some(node),
        in_boolean_position,
      ),
      _ => false,
    }
  }

  fn report(&mut self, condition: &Expr) {
    if self.is_constant(condition, None, true) {
      let span = condition.span();
      self.add_diagnostic(span);
    }
  }
}

impl<'c> VisitAll for NoConstantConditionVisitor<'c> {
  noop_visit_type!();

  fn visit_cond_expr(&mut self, cond_expr: &CondExpr, _parent: &dyn Node) {
    self.report(&cond_expr.test);
  }

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, _parent: &dyn Node) {
    self.report(&if_stmt.test);
  }

  /* TODO(bartlomieju): temporarly disabled because
    deno_std uses while (true) {} loops
  fn visit_while_stmt(
    &mut self,
    while_stmt: &WhileStmt,
    _parent: &dyn Node,
  ) {
    self.report(&while_stmt.test)
    while_stmt.visit_children_with(self);
  }

  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &DoWhileStmt,
    _parent: &dyn Node,
  ) {
    self.report(&do_while_stmt.test)
    do_while_stmt.visit_children_with(self);
  }

  fn visit_for_stmt(
    &mut self,
    for_stmt: &ForStmt,
    _parent: &dyn Node,
  ) {
    if let Some(cond) = for_stmt.test.as_ref() {
      self.report(cond)
    }
    for_stmt.visit_children_with(self);
  }
  */
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
    assert_lint_err::<NoConstantCondition>(r#"true ? 1 : 2;"#, 0);
    assert_lint_err::<NoConstantCondition>(r#"q = 0 ? 1 : 2;"#, 4);
    assert_lint_err::<NoConstantCondition>(r#"(q = 0) ? 1 : 2;"#, 0);
    assert_lint_err::<NoConstantCondition>(r#"`` ? 1 : 2;"#, 0);
    assert_lint_err::<NoConstantCondition>(r#"`foo` ? 1 : 2;"#, 0);
    assert_lint_err::<NoConstantCondition>(r#"`foo${bar}` ? 1 : 2;"#, 0);
    assert_lint_err::<NoConstantCondition>(r#"if(-2);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(true);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if({});"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(0 < 1);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(0 || 1);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(a, 1);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`foo`);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(``);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`\\\n`);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`${'bar'}`);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`${'bar' + `foo`}`);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`foo${false || true}`);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`foo${bar}`);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`${bar}foo`);"#, 3);

    // typeof conditions
    assert_lint_err::<NoConstantCondition>(r#"if(typeof x){}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`${typeof x}`){}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(`${''}${typeof x}`){}"#, 3);
    assert_lint_err::<NoConstantCondition>(
      r#"if(typeof 'abc' === 'string'){}"#,
      3,
    );
    assert_lint_err::<NoConstantCondition>(r#"if(a = typeof b){}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(a, typeof b){}"#, 3);
    assert_lint_err::<NoConstantCondition>(
      r#"if(typeof 'a' == 'string' || typeof 'b' == 'string'){}"#,
      3,
    );

    // void conditions
    assert_lint_err::<NoConstantCondition>(r#"if(1 || void x);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(void x);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(y = void x);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(x, void x);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(void x === void y);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(void x && a);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(a && void x);"#, 3);

    assert_lint_err::<NoConstantCondition>(r#"if(false && abc==='str'){}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(true || abc==='str'){}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(abc==='str' || true){}"#, 3);
    assert_lint_err::<NoConstantCondition>(
      r#"if(abc==='str' || true || def ==='str'){}"#,
      3,
    );
    assert_lint_err::<NoConstantCondition>(r#"if(false || true){}"#, 3);
    assert_lint_err::<NoConstantCondition>(
      r#"if(typeof abc==='str' || true){}"#,
      3,
    );

    // string literals
    assert_lint_err::<NoConstantCondition>(r#"if('str1' || 'str2'){}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if('str1' && 'str2'){}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(+1) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if([a]) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if([]) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(''+['a']) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(''+[]) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if([a]==[a]) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if([a] - '') {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(+[a]) {}"#, 3);

    // nested
    assert_lint_err::<NoConstantCondition>(r#"if (foo) { if (true) {} }"#, 15);
    assert_lint_err::<NoConstantCondition>(
      r#"if (foo) {} else if (true) {}"#,
      21,
    );
    assert_lint_err::<NoConstantCondition>(
      r#"if (foo) {} else if (bar) {} else if (true) {}"#,
      38,
    );
    assert_lint_err::<NoConstantCondition>(
      r#"if (foo) {} else { if (true) {} }"#,
      23,
    );
    assert_lint_err::<NoConstantCondition>(r#"foo ? true ? 1 : 2 : 3"#, 6);
  }

  // TODO(humancalico) make these tests pass
  #[test]
  fn failing() {
    // TODO(humancalico) can be uncommented after adding more conditions https://github.com/eslint/eslint/blob/f4d7b9e1a599346b2f21ff9de003b311b51411e6/lib/rules/no-constant-condition.js#L135-L146
    // assert_lint_ok::<NoConstantCondition>(r#"if(xyz === 'str1' && abc==='str2'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(xyz === 'str1' || abc==='str2'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(xyz === 'str1' || abc==='str2' && pqr === 5){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(typeof abc === 'string' && abc==='str2'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(false || abc==='str'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(true && abc==='str'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(typeof 'str' && abc==='str'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(abc==='str' || false || def ==='str'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(true && abc==='str' || def ==='str'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(true && typeof abc==='string'){}"#);

    // TODO(humancalico) add a configuration option for { checkLoops: false } https://eslint.org/docs/rules/no-constant-condition#checkloops
    // assert_lint_ok::<NoConstantCondition>(r#"while(true);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"for(;true;);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"do{}while(true)"#);

    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){while(true){yield 'foo';}}"#,);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){for(;true;){yield 'foo';}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){do{yield 'foo';}while(true)}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){while (true) { while(true) {yield;}}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() {for (; yield; ) {}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() {for (; ; yield) {}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() {while (true) {function* foo() {yield;}yield;}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() { for (let x = yield; x < 10; x++) {yield;}yield;}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() { for (let x = yield; ; x++) { yield; }}"#);

    // TODO(humancalico) can be uncommented after adding more conditions https://github.com/eslint/eslint/blob/f4d7b9e1a599346b2f21ff9de003b311b51411e6/lib/rules/no-constant-condition.js#L135-L146
    // assert_lint_err::<NoConstantCondition>(r#"if(abc==='str' || 'str'){}"#, 3);
    // assert_lint_err::<NoConstantCondition>(r#"if(a || 'str'){}"#, 3);

    // assert_lint_err::<NoConstantCondition>(r#"function* foo(){while(true){} yield 'foo';}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function* foo(){while(true){if (true) {yield 'foo';}}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function* foo(){while(true){yield 'foo';} while(true) {}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"var a = function* foo(){while(true){} yield 'foo';}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"while (true) { function* foo() {yield;}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function* foo(){if (true) {yield 'foo';}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function* foo() {for (let foo = yield; true;) {}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function* foo() {for (foo = yield; true;) {}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function foo() {while (true) {function* bar() {while (true) {yield;}}}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function foo() {while (true) {const bar = function*() {while (true) {yield;}}}}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"function* foo() { for (let foo = 1 + 2 + 3 + (yield); true; baz) {}}"#, );

    // assert_lint_err::<NoConstantCondition>(r#"if ([,] + ''){}"#, );
  }
}
