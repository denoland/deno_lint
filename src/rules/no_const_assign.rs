// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::scopes::BindingKind;
use crate::scopes::ScopeManager;
use crate::scopes::ScopeVisitor;
use crate::swc_common::Span;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::AssignExpr;
use crate::swc_ecma_ast::Expr;
use crate::swc_ecma_ast::ObjectPatProp;
use crate::swc_ecma_ast::Pat;
use crate::swc_ecma_ast::PatOrExpr;
use crate::swc_ecma_ast::UpdateExpr;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoConstAssign;

impl LintRule for NoConstAssign {
  fn new() -> Box<Self> {
    Box::new(NoConstAssign)
  }

  fn code(&self) -> &'static str {
    "no-const-assign"
  }

  fn lint_module(&self, context: Context, module: &swc_ecma_ast::Module) {
    let mut scope_visitor = ScopeVisitor::new();
    scope_visitor.visit_module(module, module);
    let scope_manager = scope_visitor.consume();
    let mut visitor = NoConstAssignVisitor::new(context, scope_manager);
    visitor.visit_module(module, module);
  }
}

struct NoConstAssignVisitor {
  context: Context,
  scope_manager: ScopeManager,
}

impl NoConstAssignVisitor {
  pub fn new(context: Context, scope_manager: ScopeManager) -> Self {
    Self {
      context,
      scope_manager,
    }
  }

  fn check_pat(&mut self, pat: &Pat, span: Span) {
    match pat {
      Pat::Ident(ident) => {
        self.check_scope_for_const(span, &ident.sym.to_string());
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

  fn check_obj_pat(&mut self, object: &swc_ecma_ast::ObjectPat, span: Span) {
    if !object.props.is_empty() {
      for prop in object.props.iter() {
        if let ObjectPatProp::Assign(assign_prop) = prop {
          self.check_scope_for_const(
            assign_prop.key.span,
            &assign_prop.key.sym.to_string(),
          );
        } else if let ObjectPatProp::KeyValue(kv_prop) = prop {
          self.check_pat(&kv_prop.value, span);
        }
      }
    }
  }

  fn check_array_pat(&mut self, array: &swc_ecma_ast::ArrayPat, span: Span) {
    if !array.elems.is_empty() {
      for elem in array.elems.iter() {
        if let Some(element) = elem {
          self.check_pat(element, span);
        }
      }
    }
  }

  fn check_scope_for_const(&mut self, span: Span, ident: &str) {
    let scope = self.scope_manager.get_scope_for_span(span);
    if let Some(binding) = self.scope_manager.get_binding(scope, ident) {
      if binding.kind == BindingKind::Const {
        self.context.add_diagnostic(
          span,
          "no-const-assign",
          "Reassigning constant variable is not allowed",
        );
      }
    }
  }
}

impl Visit for NoConstAssignVisitor {
  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _node: &dyn Node) {
    match &assign_expr.left {
      PatOrExpr::Expr(pat_expr) => {
        if let Expr::Ident(ident) = &**pat_expr {
          self.check_scope_for_const(assign_expr.span, &ident.sym.to_string());
        }
      }
      PatOrExpr::Pat(boxed_pat) => self.check_pat(boxed_pat, assign_expr.span),
    };
  }

  fn visit_update_expr(&mut self, update_expr: &UpdateExpr, _node: &dyn Node) {
    if let Expr::Ident(ident) = &*update_expr.arg {
      self.check_scope_for_const(update_expr.span, &ident.sym.to_string());
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_const_assign_valid() {
    assert_lint_ok::<NoConstAssign>(
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
    );
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
