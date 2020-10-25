// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use swc_common::Span;
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ObjectPatProp;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::PatOrExpr;
use swc_ecmascript::ast::{Ident, UpdateExpr};
use swc_ecmascript::visit::Node;
use swc_ecmascript::{utils::ident::IdentLike, visit::Visit};

pub struct NoConstAssign;

impl LintRule for NoConstAssign {
  fn new() -> Box<Self> {
    Box::new(NoConstAssign)
  }

  fn code(&self) -> &'static str {
    "no-const-assign"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoConstAssignVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows modifying a variable declared as `const`.

Modifying a variable declared as `const` will result in a runtime error.

### Invalid:
```typescript
const a = 0;
a = 1;
a += 1;
a++;
++a;
```

### Valid:
```typescript
const a = 0;
const b = a + 1;

// `c` is out of scope on each loop iteration, allowing a new assignment
for (const c in [1,2,3]) {}
```
"#
  }
}

struct NoConstAssignVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoConstAssignVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_pat(&mut self, pat: &Pat, span: Span) {
    match pat {
      Pat::Ident(ident) => {
        self.check_scope_for_const(span, ident);
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
    object: &swc_ecmascript::ast::ObjectPat,
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
    array: &swc_ecmascript::ast::ArrayPat,
    span: Span,
  ) {
    if !array.elems.is_empty() {
      for elem in array.elems.iter() {
        if let Some(element) = elem {
          self.check_pat(element, span);
        }
      }
    }
  }

  fn check_scope_for_const(&mut self, span: Span, name: &Ident) {
    let id = name.to_id();
    if let Some(v) = self.context.scope.var(&id) {
      if let BindingKind::Const = v.kind() {
        self.context.add_diagnostic_with_hint(
          span,
          "no-const-assign",
          "Reassigning constant variable is not allowed",
          "Change `const` declaration to `let` or double check the correct variable is used"
        );
      }
    }
  }
}

impl<'c> Visit for NoConstAssignVisitor<'c> {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    match &assign_expr.left {
      PatOrExpr::Expr(pat_expr) => {
        if let Expr::Ident(ident) = &**pat_expr {
          self.check_scope_for_const(assign_expr.span, &ident);
        }
      }
      PatOrExpr::Pat(boxed_pat) => self.check_pat(boxed_pat, assign_expr.span),
    };
  }

  fn visit_update_expr(&mut self, update_expr: &UpdateExpr, _node: &dyn Node) {
    if let Expr::Ident(ident) = &*update_expr.arg {
      self.check_scope_for_const(update_expr.span, &ident);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
    assert_lint_err::<NoConstAssign>("const x = 0; x = 1;", 13);
    assert_lint_err::<NoConstAssign>("const {a: x} = {a: 0}; x = 1;", 23);
    assert_lint_err::<NoConstAssign>("const x = 0; ({x} = {x: 1});", 15);
    assert_lint_err::<NoConstAssign>("const x = 0; ({a: x = 1} = {});", 14);
    assert_lint_err::<NoConstAssign>("const x = 0; x += 1;", 13);
    assert_lint_err::<NoConstAssign>("const x = 0; ++x;", 13);
    assert_lint_err::<NoConstAssign>(
      "const x = 0; function foo() { x = x + 1; }",
      30,
    );
    assert_lint_err::<NoConstAssign>(
      "const x = 0; function foo(a) { x = a; }",
      31,
    );
    assert_lint_err::<NoConstAssign>("for (const i = 0; i < 10; ++i) {}", 26);
    assert_lint_err::<NoConstAssign>(
      "const x = 0; while (true) { x = x + 1; }",
      28,
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
switch (char) {
  case "a":
    const a = true;
  break;
  case "b":
    a = false;
  break;
}"#,
      7,
      4,
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
  try {
    const a = 1;
    a = 2;
  } catch (e) {}"#,
      4,
      4,
    );
    assert_lint_err_on_line_n::<NoConstAssign>(
      r#"
if (true) {
  const a = 1;
  if (false) {
    a = 2;
  } else {
    a = 2;
  }
}"#,
      vec![(5, 4), (7, 4)],
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
for (const a of [1, 2, 3]) {
  a = 0;
}"#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
for (const a in [1, 2, 3]) {
  a = 0;
}"#,
      3,
      2,
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
while (true) {
  const a = 1;
  while (a == 1) {
    a = 2;
  }
}"#,
      5,
      4,
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
const lambda = () => {
  const a = 1;
  {
    a = 1;
  }
}"#,
      5,
      4,
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
class URL {
  get port(){
    const port = 80;
    port = 3000;
    return port;
  }
}"#,
      5,
      4,
    );
    assert_lint_err_on_line::<NoConstAssign>(
      r#"
declare module "foo" {
  const a = 1;
  a=2;
}"#,
      4,
      2,
    );
    assert_lint_err_n::<NoConstAssign>(
      "const x = 0  ; x = 1; x = 2;",
      vec![15, 22],
    );
  }
}
