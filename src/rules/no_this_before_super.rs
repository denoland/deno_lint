// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  CallExpression, Class, Expression, FunctionBody, MethodDefinitionKind,
  Program, Statement,
};
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct NoThisBeforeSuper;

const CODE: &str = "no-this-before-super";
const MESSAGE: &str = "In the constructor of derived classes, `this` / `super` are not allowed before calling to `super()`.";
const HINT: &str = "Call `super()` before using `this` or `super` keyword.";

impl LintRule for NoThisBeforeSuper {
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
    let mut handler = NoThisBeforeSuperHandler {
      is_derived_class: Vec::new(),
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoThisBeforeSuperHandler {
  is_derived_class: Vec<bool>,
}

impl NoThisBeforeSuperHandler {
  fn inside_derived_class(&self) -> bool {
    self.is_derived_class.last().copied().unwrap_or(false)
  }
}

enum FirstAppeared {
  SuperCalled,
  SuperAccessed(Span),
  ThisAccessed(Span),
}

fn check_stmt_for_this_before_super(stmt: &Statement) -> Option<FirstAppeared> {
  let mut checker = StmtChecker {
    result: None,
    fn_depth: 0,
  };
  check_statement(&mut checker, stmt);
  checker.result
}

struct StmtChecker {
  result: Option<FirstAppeared>,
  fn_depth: u32,
}

impl StmtChecker {
  fn record_this(&mut self, span: Span) {
    if self.fn_depth == 0 && self.result.is_none() {
      self.result = Some(FirstAppeared::ThisAccessed(span));
    }
  }

  fn record_super_access(&mut self, span: Span) {
    if self.fn_depth == 0 && self.result.is_none() {
      self.result = Some(FirstAppeared::SuperAccessed(span));
    }
  }

  fn record_super_call(&mut self) {
    if self.fn_depth == 0 && self.result.is_none() {
      self.result = Some(FirstAppeared::SuperCalled);
    }
  }
}

fn check_expression(checker: &mut StmtChecker, expr: &Expression) {
  if checker.result.is_some() {
    return;
  }
  match expr {
    Expression::ThisExpression(this) => {
      checker.record_this(this.span);
    }
    Expression::Super(s) => {
      checker.record_super_access(s.span);
    }
    Expression::CallExpression(call) => {
      check_call_expression(checker, call);
    }
    Expression::FunctionExpression(_)
    | Expression::ArrowFunctionExpression(_) => {
      // Don't descend into nested functions
    }
    Expression::ClassExpression(class) => {
      // Don't descend into nested class bodies, but do check super_class
      if let Some(super_class) = &class.super_class {
        check_expression(checker, super_class);
      }
    }
    Expression::AssignmentExpression(a) => {
      check_expression(checker, &a.right);
      // Also check left side for member expressions containing this/super
      check_assignment_target(checker, &a.left);
    }
    Expression::BinaryExpression(b) => {
      check_expression(checker, &b.left);
      check_expression(checker, &b.right);
    }
    Expression::LogicalExpression(l) => {
      check_expression(checker, &l.left);
      check_expression(checker, &l.right);
    }
    Expression::ConditionalExpression(c) => {
      check_expression(checker, &c.test);
      check_expression(checker, &c.consequent);
      check_expression(checker, &c.alternate);
    }
    Expression::SequenceExpression(s) => {
      for e in &s.expressions {
        check_expression(checker, e);
      }
    }
    Expression::UnaryExpression(u) => {
      check_expression(checker, &u.argument);
    }
    Expression::UpdateExpression(u) => {
      check_simple_assign_target_expr(checker, &u.argument);
    }
    Expression::AwaitExpression(a) => {
      check_expression(checker, &a.argument);
    }
    Expression::YieldExpression(y) => {
      if let Some(arg) = &y.argument {
        check_expression(checker, arg);
      }
    }
    Expression::NewExpression(n) => {
      check_expression(checker, &n.callee);
      for arg in &n.arguments {
        check_argument(checker, arg);
      }
    }
    Expression::ArrayExpression(a) => {
      for elem in &a.elements {
        match elem {
          deno_ast::oxc::ast::ast::ArrayExpressionElement::SpreadElement(s) => {
            check_expression(checker, &s.argument);
          }
          deno_ast::oxc::ast::ast::ArrayExpressionElement::Elision(_) => {}
          _ => {
            if let Some(expr) = elem.as_expression() {
              check_expression(checker, expr);
            }
          }
        }
      }
    }
    Expression::ObjectExpression(o) => {
      for prop in &o.properties {
        match prop {
          deno_ast::oxc::ast::ast::ObjectPropertyKind::ObjectProperty(p) => {
            check_expression(checker, &p.value);
          }
          deno_ast::oxc::ast::ast::ObjectPropertyKind::SpreadProperty(s) => {
            check_expression(checker, &s.argument);
          }
        }
      }
    }
    Expression::TemplateLiteral(t) => {
      for expr in &t.expressions {
        check_expression(checker, expr);
      }
    }
    Expression::TaggedTemplateExpression(t) => {
      check_expression(checker, &t.tag);
      for expr in &t.quasi.expressions {
        check_expression(checker, expr);
      }
    }
    Expression::StaticMemberExpression(m) => {
      check_expression(checker, &m.object);
    }
    Expression::ComputedMemberExpression(m) => {
      check_expression(checker, &m.object);
      check_expression(checker, &m.expression);
    }
    Expression::PrivateFieldExpression(m) => {
      check_expression(checker, &m.object);
    }
    Expression::ParenthesizedExpression(p) => {
      check_expression(checker, &p.expression);
    }
    Expression::TSAsExpression(t) => {
      check_expression(checker, &t.expression);
    }
    Expression::TSTypeAssertion(t) => {
      check_expression(checker, &t.expression);
    }
    Expression::TSNonNullExpression(t) => {
      check_expression(checker, &t.expression);
    }
    Expression::TSSatisfiesExpression(t) => {
      check_expression(checker, &t.expression);
    }
    Expression::TSInstantiationExpression(t) => {
      check_expression(checker, &t.expression);
    }
    _ => {}
  }
}

fn check_argument(
  checker: &mut StmtChecker,
  arg: &deno_ast::oxc::ast::ast::Argument,
) {
  if let Some(expr) = arg.as_expression() {
    check_expression(checker, expr);
  } else if let deno_ast::oxc::ast::ast::Argument::SpreadElement(s) = arg {
    check_expression(checker, &s.argument);
  }
}

fn check_simple_assign_target_expr(
  checker: &mut StmtChecker,
  expr: &deno_ast::oxc::ast::ast::SimpleAssignmentTarget,
) {
  use deno_ast::oxc::ast::ast::SimpleAssignmentTarget::*;
  match expr {
    AssignmentTargetIdentifier(_) => {}
    TSAsExpression(t) => {
      check_expression(checker, &t.expression);
    }
    TSNonNullExpression(t) => {
      check_expression(checker, &t.expression);
    }
    TSSatisfiesExpression(t) => {
      check_expression(checker, &t.expression);
    }
    TSTypeAssertion(t) => {
      check_expression(checker, &t.expression);
    }
    _ => {
      if let Some(member) = expr.as_member_expression() {
        check_member_expr(checker, member);
      }
    }
  }
}

fn check_member_expr(
  checker: &mut StmtChecker,
  member: &deno_ast::oxc::ast::ast::MemberExpression,
) {
  match member {
    deno_ast::oxc::ast::ast::MemberExpression::StaticMemberExpression(m) => {
      check_expression(checker, &m.object);
    }
    deno_ast::oxc::ast::ast::MemberExpression::ComputedMemberExpression(m) => {
      check_expression(checker, &m.object);
      check_expression(checker, &m.expression);
    }
    deno_ast::oxc::ast::ast::MemberExpression::PrivateFieldExpression(m) => {
      check_expression(checker, &m.object);
    }
  }
}

fn check_assignment_target(
  checker: &mut StmtChecker,
  target: &deno_ast::oxc::ast::ast::AssignmentTarget,
) {
  use deno_ast::oxc::ast::ast::AssignmentTarget;
  match target {
    AssignmentTarget::AssignmentTargetIdentifier(_) => {}
    _ => {
      if let Some(simple) = target.as_simple_assignment_target() {
        check_simple_assign_target_expr(checker, simple);
      }
    }
  }
}

fn check_call_expression(checker: &mut StmtChecker, call: &CallExpression) {
  // arguments are evaluated before the callee
  for arg in &call.arguments {
    check_argument(checker, arg);
  }

  if checker.result.is_none() && matches!(&call.callee, Expression::Super(_)) {
    checker.record_super_call();
    return;
  }

  check_expression(checker, &call.callee);
}

fn check_statement(checker: &mut StmtChecker, stmt: &Statement) {
  if checker.result.is_some() {
    return;
  }
  match stmt {
    Statement::ExpressionStatement(expr_stmt) => {
      check_expression(checker, &expr_stmt.expression);
    }
    Statement::VariableDeclaration(var_decl) => {
      for decl in &var_decl.declarations {
        if let Some(init) = &decl.init {
          check_expression(checker, init);
        }
      }
    }
    Statement::ReturnStatement(ret) => {
      if let Some(arg) = &ret.argument {
        check_expression(checker, arg);
      }
    }
    Statement::ThrowStatement(t) => {
      check_expression(checker, &t.argument);
    }
    Statement::IfStatement(if_stmt) => {
      check_expression(checker, &if_stmt.test);
      check_statement(checker, &if_stmt.consequent);
      if let Some(alt) = &if_stmt.alternate {
        check_statement(checker, alt);
      }
    }
    Statement::BlockStatement(block) => {
      for s in &block.body {
        check_statement(checker, s);
      }
    }
    Statement::ForStatement(f) => {
      if let Some(init) = &f.init {
        if let deno_ast::oxc::ast::ast::ForStatementInit::VariableDeclaration(
          v,
        ) = init
        {
          for decl in &v.declarations {
            if let Some(init_expr) = &decl.init {
              check_expression(checker, init_expr);
            }
          }
        } else if let Some(expr) = init.as_expression() {
          check_expression(checker, expr);
        }
      }
    }
    Statement::WhileStatement(w) => {
      check_expression(checker, &w.test);
    }
    Statement::DoWhileStatement(d) => {
      check_expression(checker, &d.test);
    }
    Statement::SwitchStatement(s) => {
      check_expression(checker, &s.discriminant);
      for case in &s.cases {
        if let Some(test) = &case.test {
          check_expression(checker, test);
        }
        for s in &case.consequent {
          check_statement(checker, s);
        }
      }
    }
    Statement::TryStatement(t) => {
      for s in &t.block.body {
        check_statement(checker, s);
      }
    }
    _ => {}
  }
}

fn check_constructor_body(body: &FunctionBody, ctx: &mut Context) {
  for stmt in &body.statements {
    match check_stmt_for_this_before_super(stmt) {
      None => continue,
      Some(FirstAppeared::SuperCalled) => break,
      Some(FirstAppeared::ThisAccessed(span))
      | Some(FirstAppeared::SuperAccessed(span)) => {
        ctx.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
      }
    }
  }
}

impl Handler<'_> for NoThisBeforeSuperHandler {
  fn class(&mut self, n: &Class, _ctx: &mut Context) {
    let is_derived = n.super_class.is_some();
    self.is_derived_class.push(is_derived);
  }

  fn class_exit(&mut self, _n: &Class, _ctx: &mut Context) {
    self.is_derived_class.pop();
  }

  fn method_definition(
    &mut self,
    n: &deno_ast::oxc::ast::ast::MethodDefinition,
    ctx: &mut Context,
  ) {
    if n.kind != MethodDefinitionKind::Constructor {
      return;
    }

    if !self.inside_derived_class() {
      return;
    }

    if let Some(body) = &n.value.body {
      check_constructor_body(body, ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_this_before_super_valid() {
    assert_lint_ok! {
      NoThisBeforeSuper,
      r#"
class A {
  constructor() {
    this.a = 0;
  }
}
      "#,
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
  }
}
      "#,
      r#"
class A extends B {
  foo() {
    this.a = 0;
  }
}
      "#,
      r#"
class A extends B {
  constructor() {
    function foo() {
      this.bar();
    }
  }
}
      "#,
      r#"
class A extends B {
  constructor() {
    const foo = () => {
      this.bar();
    };
  }
}
      "#,
      r#"
class A extends B {
  constructor() {
    super({
      foo() {
        this.bar();
      }
    });
  }
}
      "#,

      // inline super class
      r#"
class A extends class extends B {
  constructor() {
    super();
    this.a = 0;
  }
} {
    constructor() {
      super();
      this.a = 0;
    }
}
      "#,

      // nested class
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
  }
  foo() {
    class C extends D {
      constructor() {
        super();
        this.c = 1;
      }
    }
  }
}
      "#,
    };
  }

  #[test]
  fn no_this_before_super_invalid() {
    assert_lint_err! {
      NoThisBeforeSuper,
      r#"
class A extends B {
  constructor() {
    this.a = 0;
    super();
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    this.foo();
    super();
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super.foo();
    super();
  }
}
    "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super(this.foo());
  }
}
    "#: [
        {
          line: 4,
          col: 10,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super();
  }
}
class C extends D {
  constructor() {
    this.c = 42;
    super();
  }
}
    "#: [
        {
          line: 9,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends class extends B {
  constructor() {
    this.a = 0;
    super();
  }
} {
    constructor() {
      super();
      this.a = 0;
    }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends class extends B {
  constructor() {
    super();
    this.a = 0;
  }
} {
    constructor() {
      this.a = 0;
      super();
    }
}
      "#: [
        {
          line: 9,
          col: 6,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
  }
  foo() {
    class C extends D {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#: [
        {
          line: 10,
          col: 8,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    this.a = 0;
    super();
  }
  foo() {
    class C extends D {
      constructor() {
        super();
        this.c = 1;
      }
    }
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A {
  constructor() {
    this.a = 0;
  }
  foo() {
    class C extends D {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#: [
        {
          line: 9,
          col: 8,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    this.a = 0;
  }
  foo() {
    class C {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      r#"
class A extends B {
  constructor() {
    super();
    this.a = 0;
    class C extends D {
      constructor() {
        this.c = 1;
      }
    }
  }
}
      "#: [
        {
          line: 8,
          col: 8,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
