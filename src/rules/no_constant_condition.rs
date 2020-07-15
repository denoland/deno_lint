// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_common::Spanned;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::Lit;
use crate::swc_ecma_ast::Module;
use swc_ecma_visit::{Node, Visit};

pub struct NoConstantCondition;

impl LintRule for NoConstantCondition {
  fn new() -> Box<Self> {
    Box::new(NoConstantCondition)
  }

  fn code(&self) -> &'static str {
    "no-constant-condition"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoConstantConditionVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoConstantConditionVisitor {
  context: Context,
}

impl NoConstantConditionVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "no-constant-condition",
      "Use of a constant expressions as conditions is not allowed.",
    );
  }

  fn check_short_circuit(
    &self,
    expr: &Expr,
    operator: swc_ecma_ast::BinaryOp,
  ) -> bool {
    match expr {
      Expr::Lit(lit) => match lit {
        Lit::Bool(boolean) => {
          // println!("inside short circuit lit");
          (operator == swc_ecma_ast::BinaryOp::LogicalOr && boolean.value)
            || (operator == swc_ecma_ast::BinaryOp::LogicalAnd
              && !boolean.value)
        }
        _ => {
          // println!("inside short circuit lit false");
          false
        }
      },
      Expr::Unary(unary) => {
        operator == swc_ecma_ast::BinaryOp::LogicalAnd
          && unary.op == swc_ecma_ast::UnaryOp::Void
      }
      Expr::Bin(bin)
        if bin.op == swc_ecma_ast::BinaryOp::LogicalAnd
          || bin.op == swc_ecma_ast::BinaryOp::LogicalOr =>
      {
        // println!("inside short circuit binary logical expression");
        self.check_short_circuit(&bin.left, bin.op)
          || self.check_short_circuit(&bin.right, bin.op)
      }
      _ => {
        // println!("inside short circuit default false _ ");
        false
      }
    }
  }

  fn is_constant(&self, node: &Expr, parent_node: Option<&Expr>) -> bool {
    match node {
      Expr::Lit(_) | Expr::Arrow(_) | Expr::Fn(_) | Expr::Object(_) => {
        // println!("inside is_constant Lit, Arrow, Fn, Object");
        true
      }
      Expr::Paren(paren) => self.is_constant(&paren.expr, Some(node)),
      // TODO(humancalico) add support for Template Literals
      Expr::Array(arr) => match parent_node {
        Some(Expr::Bin(bin)) => {
          if bin.op == swc_ecma_ast::BinaryOp::Add {
            // println!("inside array add");
            arr.elems.iter().all(|element| {
              // println!("checking all elements");
              self.is_constant(&element.as_ref().unwrap().expr, parent_node)
            })
          } else {
            false
          }
        }
        _ => {
          // println!("inside Expr::Array _");
          true
        }
      },
      Expr::Unary(unary) => {
        if unary.op == swc_ecma_ast::UnaryOp::Void {
          true
        } else {
          // println!("inside Expr::Unary else");
          // TODO(humancalico) add typeof condition here https://github.com/eslint/eslint/blob/f4d7b9e1a599346b2f21ff9de003b311b51411e6/lib/rules/no-constant-condition.js#L120
          self.is_constant(&unary.arg, Some(node))
        }
      }
      Expr::Bin(bin) => {
        // This is for LogicalExpression
        if bin.op == swc_ecma_ast::BinaryOp::LogicalOr
          || bin.op == swc_ecma_ast::BinaryOp::LogicalAnd
        {
          // println!("inside is_constant binary logical expression");
          let is_left_constant = self.is_constant(&bin.left, Some(node));
          // println!("is_left_constant: {}", &is_left_constant);
          let is_right_constant = self.is_constant(&bin.right, Some(node));
          // println!("tag7: {}", &self.check_short_circuit(&bin.left, bin.op));
          let is_left_short_circuit =
            is_left_constant && self.check_short_circuit(&bin.left, bin.op);
          // println!("tag8: {}", &is_left_short_circuit);
          let is_right_short_circuit =
            is_right_constant && self.check_short_circuit(&bin.right, bin.op);
          // println!("tag10: {}", &is_right_short_circuit);
          (is_left_constant && is_right_constant)
          // TODO(humancalico) add more condiitons here from https://github.com/eslint/eslint/blob/f4d7b9e1a599346b2f21ff9de003b311b51411e6/lib/rules/no-constant-condition.js#L135-L146
            || is_left_short_circuit
            || is_right_short_circuit
        // (is_left_constant && is_right_constant) || ();
        } else if bin.op != swc_ecma_ast::BinaryOp::In {
          // println!("inside binary expression");
          self.is_constant(&bin.left, Some(node))
            && self.is_constant(&bin.right, Some(node))
        } else {
          false
        }
      }
      Expr::Assign(assign) => self.is_constant(&assign.right, Some(node)),
      Expr::Seq(seq) => {
        self.is_constant(&seq.exprs[seq.exprs.len() - 1], Some(node))
      }
      _ => false,
    }
  }

  fn report(&self, condition: &Expr) {
    if self.is_constant(condition, None) {
      let span = condition.span();
      self.add_diagnostic(span);
    } else {
      ()
    }
  }
}

impl Visit for NoConstantConditionVisitor {
  // TODO(humancalico) add tests for conditional expressions
  fn visit_cond_expr(
    &mut self,
    cond_expr: &swc_ecma_ast::CondExpr,
    _parent: &dyn Node,
  ) {
    // dbg!(&cond_expr);
    self.report(&cond_expr.test)
  }
  fn visit_if_stmt(
    &mut self,
    if_stmt: &swc_ecma_ast::IfStmt,
    _parent: &dyn Node,
  ) {
    // dbg!(&if_stmt);
    self.report(&if_stmt.test)
  }
  fn visit_while_stmt(
    &mut self,
    while_stmt: &swc_ecma_ast::WhileStmt,
    __parent: &dyn Node,
  ) {
    // dbg!(&while_stmt);
    self.report(&while_stmt.test)
  }
  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &swc_ecma_ast::DoWhileStmt,
    __parent: &dyn Node,
  ) {
    // dbg!(&while_stmt);
    self.report(&do_while_stmt.test)
  }
  fn visit_for_stmt(
    &mut self,
    for_stmt: &swc_ecma_ast::ForStmt,
    __parent: &dyn Node,
  ) {
    // dbg!(&for_stmt);
    self.report(&for_stmt.test.as_ref().unwrap())
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_constant_condition_all_tests() {
    assert_lint_ok::<NoConstantCondition>(r#"if(a);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if(a == 0);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if(a = f());"#);
    assert_lint_ok::<NoConstantCondition>(r#"if(1, a);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if ('every' in []);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (`\\\n${a}`) {}"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (`${a}`);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (`${foo()}`);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (`${a === 'b' && b==='a'}`);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (`foo${a}` === 'fooa');"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (tag`a`);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (tag`${a}`);"#);
    assert_lint_ok::<NoConstantCondition>(r#"while(~!a);"#);
    assert_lint_ok::<NoConstantCondition>(r#"while(a = b);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"while(`${a}`);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"for(;x < 10;);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"for(;;);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"for(;`${a}`;);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"do{ }while(x)"#);
    // assert_lint_ok::<NoConstantCondition>(r#"q > 0 ? 1 : 2;"#);
    // assert_lint_ok::<NoConstantCondition>(r#"`${a}` === a ? 1 : 2"#);
    // assert_lint_ok::<NoConstantCondition>(r#"`foo${a}` === a ? 1 : 2"#);
    // assert_lint_ok::<NoConstantCondition>(r#"tag`a` === a ? 1 : 2"#);
    // assert_lint_ok::<NoConstantCondition>(r#"tag`${a}` === a ? 1 : 2"#);
    // assert_lint_ok::<NoConstantCondition>(r#"while(x += 3) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"while(tag`a`) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"while(tag`${a}`) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"while(`\\\n${a}`) {}"#);

    // typeof conditions
    assert_lint_ok::<NoConstantCondition>(r#"if(typeof x === 'undefined'){}"#);
    assert_lint_ok::<NoConstantCondition>(
      r#"if(`${typeof x}` === 'undefined'){}"#,
    );
    assert_lint_ok::<NoConstantCondition>(r#"if(a === 'str' && typeof b){}"#);
    // assert_lint_ok::<NoConstantCondition>("typeof a == typeof b");
    // assert_lint_ok::<NoConstantCondition>("typeof 'a' === 'string'|| typeof b === 'string'");
    // assert_lint_ok::<NoConstantCondition>("`${typeof 'a'}` === 'string'|| `${typeof b}` === 'string'");

    // void conditions
    assert_lint_ok::<NoConstantCondition>(r#"if (a || void a);"#);
    assert_lint_ok::<NoConstantCondition>(r#"if (void a || a);"#);

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

    // string literals
    // assert_lint_ok::<NoConstantCondition>(r#"if('str' || a){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if('str1' && a){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if(a && 'str'){}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if('str' || abc==='str'){}"#);

    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 'bar') === 'baz') {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 'bar') !== 'baz') {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 'bar') == 'baz') {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 'bar') != 'baz') {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 233) > 666) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 233) < 666) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 233) >= 666) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || 233) <= 666) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((key || 'k') in obj) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ((foo || {}) instanceof obj) {}"#);

    // assert_lint_ok::<NoConstantCondition>(r#"if ('' + [y] === '' + [ty]) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ('a' === '' + [ty]) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ('' + [y, m, d] === 'a') {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ('' + [y, 'm'] === '' + [ty, 'tm']) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ('' + [y, 'm'] === '' + ['ty']) {}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ([,] in\n\n($2))\n ;\nelse\n ;"#);
    // assert_lint_ok::<NoConstantCondition>(r#"if ([...x]+'' === 'y'){}"#);

    // { checkLoops: false } https://eslint.org/docs/rules/no-constant-condition#checkloops
    // assert_lint_ok::<NoConstantCondition>(r#"while(true);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"for(;true;);"#);
    // assert_lint_ok::<NoConstantCondition>(r#"do{}while(true)"#);

    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){while(true){yield 'foo';}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){for(;true;){yield 'foo';}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){do{yield 'foo';}while(true)}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo(){while (true) { while(true) {yield;}}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() {for (; yield; ) {}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() {for (; ; yield) {}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() {while (true) {function* foo() {yield;}yield;}}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() { for (let x = yield; x < 10; x++) {yield;}yield;}"#);
    // assert_lint_ok::<NoConstantCondition>(r#"function* foo() { for (let x = yield; ; x++) { yield; }}#);

    assert_lint_err::<NoConstantCondition>(r#"for(;true;);"#, 5);
    // assert_lint_err::<NoConstantCondition>(r#"for(;``;);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"for(;`foo`;);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"for(;`foo${bar}`;);"#, );
    assert_lint_err::<NoConstantCondition>(r#"do{}while(true)"#, 10);
    assert_lint_err::<NoConstantCondition>(r#"do{}while(t = -2)"#, 10);
    // assert_lint_err::<NoConstantCondition>(r#"do{}while(``)"#, );
    // assert_lint_err::<NoConstantCondition>(r#"do{}while(`foo`)"#, );
    // assert_lint_err::<NoConstantCondition>(r#"do{}while(`foo${bar}`)"#, );

    assert_lint_err::<NoConstantCondition>(r#"true ? 1 : 2;"#, 0);
    // FIXME(humancalico) Is it supposed to be on column 4
    assert_lint_err::<NoConstantCondition>(r#"q = 0 ? 1 : 2;"#, 4);
    assert_lint_err::<NoConstantCondition>(r#"(q = 0) ? 1 : 2;"#, 0);
    // assert_lint_err::<NoConstantCondition>(r#"`` ? 1 : 2;"#, );
    // assert_lint_err::<NoConstantCondition>(r#"`foo` ? 1 : 2;"#, );
    // assert_lint_err::<NoConstantCondition>(r#"`foo${bar}` ? 1 : 2;"#, );
    assert_lint_err::<NoConstantCondition>(r#"if(-2);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(true);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if({});"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(0 < 1);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(0 || 1);"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(a, 1);"#, 3);
    // assert_lint_err::<NoConstantCondition>(r#"if(`foo`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(``);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`\\\n`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`${'bar'}`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`${'bar' + `foo`}`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`foo${false || true}`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`foo${bar}`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`${bar}foo`);"#, );

    assert_lint_err::<NoConstantCondition>(r#"while([]);"#, 6);
    assert_lint_err::<NoConstantCondition>(r#"while(~!0);"#, 6);
    assert_lint_err::<NoConstantCondition>(r#"while(x = 1);"#, 6);
    assert_lint_err::<NoConstantCondition>(r#"while(function(){});"#, 6);
    assert_lint_err::<NoConstantCondition>(r#"while(true);"#, 6);
    assert_lint_err::<NoConstantCondition>(r#"while(() => {});"#, 6);
    // assert_lint_err::<NoConstantCondition>(r#"while(`foo`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"while(``);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"while(`${'foo'}`);"#, );
    // assert_lint_err::<NoConstantCondition>(r#"while(`${'foo' + 'bar'}`);"#, );

    // typeof conditions
    // assert_lint_err::<NoConstantCondition>(r#"if(typeof x){}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`${typeof x}`){}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(`${''}${typeof x}`){}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(typeof 'abc' === 'string'){}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(a = typeof b){}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(a, typeof b){}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"if(typeof 'a' == 'string' || typeof 'b' == 'string'){}"#, );
    // assert_lint_err::<NoConstantCondition>(r#"while(typeof x){}"#, );

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
    // TODO(humancalico) make these test pass
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

    assert_lint_err::<NoConstantCondition>(r#"if(+1) {}"#, 3);
    // assert_lint_err::<NoConstantCondition>(r#"if ([,] + ''){}"#, );
    assert_lint_err::<NoConstantCondition>(r#"if([a]) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if([]) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(''+['a']) {}"#, 3);
    assert_lint_err::<NoConstantCondition>(r#"if(''+[]) {}"#, 3);
    // assert_lint_err::<NoConstantCondition>(r#"if([a]==[a]) {}"#, 3);
    // assert_lint_err::<NoConstantCondition>(r#"if([a] - '') {}"#, 3);
    // assert_lint_err::<NoConstantCondition>(r#"if(+[a]) {}"#, 3);
  }

  #[test]
  fn temporary_test() {}
}
