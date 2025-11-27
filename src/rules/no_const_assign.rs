// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  ArrayPat, AssignExpr, AssignTarget, AssignTargetPat, Expr, Ident, ObjectPat,
  ObjectPatProp, Pat, SimpleAssignTarget, UpdateExpr,
};
use deno_ast::{BindingKind, SourceRange, SourceRanged};
use derive_more::Display;

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
  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoConstAssignHandler.traverse(program, context);
  }
}

struct NoConstAssignHandler;

fn check_pat(pat: &Pat, range: SourceRange, ctx: &mut Context) {
  match pat {
    Pat::Ident(ident) => {
      check_scope_for_const(range, ident.id, ctx);
    }
    Pat::Assign(assign) => {
      check_pat(&assign.left, range, ctx);
    }
    Pat::Array(array) => {
      check_array_pat(array, range, ctx);
    }
    Pat::Object(object) => {
      check_obj_pat(object, range, ctx);
    }
    _ => {}
  }
}

fn check_obj_pat(object: &ObjectPat, range: SourceRange, ctx: &mut Context) {
  if !object.props.is_empty() {
    for prop in object.props.iter() {
      if let ObjectPatProp::Assign(assign_prop) = prop {
        check_scope_for_const(assign_prop.key.range(), assign_prop.key.id, ctx);
      } else if let ObjectPatProp::KeyValue(kv_prop) = prop {
        check_pat(&kv_prop.value, range, ctx);
      }
    }
  }
}

fn check_array_pat(array: &ArrayPat, range: SourceRange, ctx: &mut Context) {
  if !array.elems.is_empty() {
    for elem in array.elems.iter().flatten() {
      check_pat(elem, range, ctx);
    }
  }
}

fn check_scope_for_const(range: SourceRange, name: &Ident, ctx: &mut Context) {
  if let Some(v) = ctx.scope().var_by_ident(name) {
    if let BindingKind::Const = v.kind() {
      ctx.add_diagnostic_with_hint(
        range,
        CODE,
        NoConstantAssignMessage::Unexpected,
        NoConstantAssignHint::Remove,
      );
    }
  }
}

impl Handler for NoConstAssignHandler {
  fn assign_expr(&mut self, assign_expr: &AssignExpr, ctx: &mut Context) {
    match &assign_expr.left {
      AssignTarget::Simple(target) => match target {
        SimpleAssignTarget::Ident(ident) => {
          check_scope_for_const(assign_expr.range(), ident.id, ctx)
        }
        SimpleAssignTarget::Member(_)
        | SimpleAssignTarget::SuperProp(_)
        | SimpleAssignTarget::Paren(_)
        | SimpleAssignTarget::OptChain(_)
        | SimpleAssignTarget::TsAs(_)
        | SimpleAssignTarget::TsSatisfies(_)
        | SimpleAssignTarget::TsNonNull(_)
        | SimpleAssignTarget::TsTypeAssertion(_)
        | SimpleAssignTarget::TsInstantiation(_)
        | SimpleAssignTarget::Invalid(_) => {
          // ignore
        }
      },
      AssignTarget::Pat(pat) => match pat {
        AssignTargetPat::Array(array) => {
          check_array_pat(array, assign_expr.range(), ctx);
        }
        AssignTargetPat::Object(object) => {
          check_obj_pat(object, assign_expr.range(), ctx);
        }
        AssignTargetPat::Invalid(_) => {
          // ignore
        }
      },
    }
  }

  fn update_expr(&mut self, update_expr: &UpdateExpr, ctx: &mut Context) {
    if let Expr::Ident(ident) = update_expr.arg {
      check_scope_for_const(update_expr.range(), ident, ctx);
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
