// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{CallExpr, Expr, ExprOrSpread, ExprOrSuper, NewExpr};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

pub struct NoArrayConstructor;

impl LintRule for NoArrayConstructor {
  fn new() -> Box<Self> {
    Box::new(NoArrayConstructor)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-array-constructor"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoArrayConstructorVisitor::new(context);
    module.visit_all_with(module, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Enforce conventional usage of array construction

Array construction is conventionally done via literal notation such as `[]` or
`[1,2,3]`.  Using the `new Array()` is discouraged as is `new Array(1,2,3)`. There
are two reasons for this.  The first is that a single supplied argument defines
the array length, while multiple arguments instead populate the array of no fixed
size.  This confusion is avoided when pre-populated arrays are only created using
literal notation.  The second argument to avoiding the `Array` constructor is that
the `Array` global may be redefined.

The one exception to this rule is when creating a new array of fixed size, e.g.
`new Array(6)`.  This is the conventional way to create arrays of fixed length.
    
### Valid:
```typescript
const a = new Array(100);
const b = [];
const c = [1,2,3];
```

### Invalid:
```typescript
// This is 4 elements, not a size 100 array of 3 elements
const a = new Array(100, 1, 2, 3);

const b = new Array(); // use [] instead
```"#
  }
}

struct NoArrayConstructorVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoArrayConstructorVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn check_args(&mut self, args: Vec<ExprOrSpread>, span: Span) {
    if args.len() != 1 {
      self.context.add_diagnostic_with_hint(
        span,
        "no-array-constructor",
        "Array Constructor is not allowed",
        "Use array literal notation (e.g. []) or single argument specifying array size only (e.g. new Array(5)",
      );
    }
  }
}

impl<'c> VisitAll for NoArrayConstructorVisitor<'c> {
  noop_visit_type!();

  fn visit_new_expr(&mut self, new_expr: &NewExpr, _parent: &dyn Node) {
    if let Expr::Ident(ident) = &*new_expr.callee {
      let name = ident.sym.as_ref();
      if name != "Array" {
        return;
      }
      if new_expr.type_args.is_some() {
        return;
      }
      match &new_expr.args {
        Some(args) => {
          self.check_args(args.to_vec(), new_expr.span);
        }
        None => self.check_args(vec![], new_expr.span),
      };
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _parent: &dyn Node) {
    if let ExprOrSuper::Expr(expr) = &call_expr.callee {
      if let Expr::Ident(ident) = expr.as_ref() {
        let name = ident.sym.as_ref();
        if name != "Array" {
          return;
        }
        if call_expr.type_args.is_some() {
          return;
        }

        self.check_args((&*call_expr.args).to_vec(), call_expr.span);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_array_constructor_valid() {
    assert_lint_ok! {
      NoArrayConstructor,
      "Array(x)",
      "Array(9)",
      "Array.foo()",
      "foo.Array()",
      "new Array(x)",
      "new Array(9)",
      "new foo.Array()",
      "new Array.foo",
      "new Array<Foo>(1, 2, 3);",
      "new Array<Foo>()",
      "Array<Foo>(1, 2, 3);",
      "Array<Foo>();",
    };
  }

  #[test]
  fn no_array_constructor_invalid() {
    assert_lint_err::<NoArrayConstructor>("new Array", 0);
    assert_lint_err::<NoArrayConstructor>("new Array()", 0);
    assert_lint_err::<NoArrayConstructor>("new Array(x, y)", 0);
    assert_lint_err::<NoArrayConstructor>("new Array(0, 1, 2)", 0);
    // nested
    assert_lint_err_on_line::<NoArrayConstructor>(
      r#"
const a = new class {
  foo() {
    let arr = new Array();
  }
}();
"#,
      4,
      14,
    );
    assert_lint_err_on_line::<NoArrayConstructor>(
      r#"
const a = (() => {
  let arr = new Array();
})();
"#,
      3,
      12,
    );
  }
}
