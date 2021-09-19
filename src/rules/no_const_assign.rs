// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::scopes::BindingKind;
use crate::ProgramRef;
use deno_ast::swc::ast::AssignExpr;
use deno_ast::swc::ast::Expr;
use deno_ast::swc::ast::ObjectPatProp;
use deno_ast::swc::ast::Pat;
use deno_ast::swc::ast::PatOrExpr;
use deno_ast::swc::ast::{Ident, UpdateExpr};
use deno_ast::swc::common::Span;
use deno_ast::swc::visit::Node;
use deno_ast::swc::{utils::ident::IdentLike, visit::Visit};
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoConstAssign;

const CODE: &str = "no-const-assign";

#[derive(Display)]
enum NoConstantAssignMessage {
  #[display(fmt = "Reassigning constant variable is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoConstantAssignHint {
  #[display(
    fmt = "Change `const` declaration to `let` or double check the correct variable is used"
  )]
  Remove,
}
impl LintRule for NoConstAssign {
  fn new() -> Arc<Self> {
    Arc::new(NoConstAssign)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoConstAssignVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_const_assign.md")
  }
}

struct NoConstAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoConstAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check_pat(&mut self, pat: &Pat, span: Span) {
    match pat {
      Pat::Ident(ident) => {
        self.check_scope_for_const(span, &ident.id);
      }
      Pat::Assign(assign) => {
        self.check_pat(&assign.left, span);
      }
      Pat::Array(array) => {
        self.check_array_pat(array, span);
      }
      Pat::Object(object) => {
        self.check_obj_pat(object, span);
      }
      _ => {}
    }
  }

  fn check_obj_pat(
    &mut self,
    object: &deno_ast::swc::ast::ObjectPat,
    span: Span,
  ) {
    if !object.props.is_empty() {
      for prop in object.props.iter() {
        if let ObjectPatProp::Assign(assign_prop) = prop {
          self.check_scope_for_const(assign_prop.key.span, &assign_prop.key);
        } else if let ObjectPatProp::KeyValue(kv_prop) = prop {
          self.check_pat(&kv_prop.value, span);
        }
      }
    }
  }

  fn check_array_pat(
    &mut self,
    array: &deno_ast::swc::ast::ArrayPat,
    span: Span,
  ) {
    if !array.elems.is_empty() {
      for elem in array.elems.iter().flatten() {
        self.check_pat(elem, span);
      }
    }
  }

  fn check_scope_for_const(&mut self, span: Span, name: &Ident) {
    let id = name.to_id();
    if let Some(v) = self.context.scope().var(&id) {
      if let BindingKind::Const = v.kind() {
        self.context.add_diagnostic_with_hint(
          span,
          CODE,
          NoConstantAssignMessage::Unexpected,
          NoConstantAssignHint::Remove,
        );
      }
    }
  }
}

impl<'c, 'view> Visit for NoConstAssignVisitor<'c, 'view> {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    match &assign_expr.left {
      PatOrExpr::Expr(pat_expr) => {
        if let Expr::Ident(ident) = &**pat_expr {
          self.check_scope_for_const(assign_expr.span, ident);
        }
      }
      PatOrExpr::Pat(boxed_pat) => self.check_pat(boxed_pat, assign_expr.span),
    };
  }

  fn visit_update_expr(&mut self, update_expr: &UpdateExpr, _node: &dyn Node) {
    if let Expr::Ident(ident) = &*update_expr.arg {
      self.check_scope_for_const(update_expr.span, ident);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_const_assign_valid() {
    assert_lint_ok! {
      NoConstAssign,
      r#"
      const x = 0; { let x; x = 1; }
      const x = 0; function a(x) { x = 1; }
      const x = 0; foo(x);
      for (const x in [1,2,3]) { foo(x); }
      for (const x of [1,2,3]) { foo(x); }
      const x = {key: 0}; x.key = 1;
      if (true) {const a = 1} else { a = 2};
      // ignores non constant.
      var x = 0; x = 1;
      let x = 0; x = 1;
      function x() {} x = 1;
      function foo(x) { x = 1; }
      class X {} X = 1;
      try {} catch (x) { x = 1; }
      Deno.test("test function", function(){
        const a = 1;
      });
      Deno.test("test another function", function(){
        a=2;
      });

      Deno.test({
        name : "test object",
        fn() : Promise<void> {
          const a = 1;
        }
      });

      Deno.test({
        name : "test another object",
        fn() : Promise<void> {
         a = 2;
        }
      });

      let obj = {
        get getter(){
          const a = 1;
          return a;
        }
        ,
        set setter(x){
          a = 2;
        }
      }
      "#,
    };
  }

  #[test]
  fn no_const_assign_invalid() {
    assert_lint_err! {
      NoConstAssign,
      r#"const x = 0; x = 1;"#: [
      {
        col: 13,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const {a: x} = {a: 0}; x = 1;"#: [
      {
        col: 23,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; ({x} = {x: 1});"#: [
      {
        col: 15,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; ({a: x = 1} = {});"#: [
      {
        col: 14,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; x += 1;"#: [
      {
        col: 13,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; ++x;"#: [
      {
        col: 13,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; function foo() { x = x + 1; }"#: [
      {
        col: 30,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; function foo(a) { x = a; }"#: [
      {
        col: 31,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"for (const i = 0; i < 10; ++i) {}"#: [
      {
        col: 26,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"const x = 0; while (true) { x = x + 1; }"#: [
      {
        col: 28,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
switch (char) {
  case "a":
    const a = true;
  break;
  case "b":
    a = false;
  break;
}"#: [
      {
        line: 7,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
try {
  const a = 1;
  a = 2;
} catch (e) {}"#:[
      {
        line: 4,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
if (true) {
  const a = 1;
  if (false) {
    a = 2;
  } else {
    a = 2;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      },
      {
        line: 7,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
for (const a of [1, 2, 3]) {
  a = 0;
}"#:[
      {
        line: 3,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
for (const a in [1, 2, 3]) {
  a = 0;
}"#:[
      {
        line: 3,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
while (true) {
  const a = 1;
  while (a == 1) {
    a = 2;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
const lambda = () => {
  const a = 1;
  {
    a = 1;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
class URL {
  get port(){
    const port = 80;
    port = 3000;
    return port;
  }
}"#:[
      {
        line: 5,
        col: 4,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      r#"
declare module "foo" {
  const a = 1;
  a=2;
}"#:[
      {
        line: 4,
        col: 2,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
      "const x = 0  ; x = 1; x = 2;": [
      {
        line: 1,
        col: 15,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      },
      {
        line: 1,
        col: 22,
        message: NoConstantAssignMessage::Unexpected,
        hint: NoConstantAssignHint::Remove,
      }],
    }
  }
}
