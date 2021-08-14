// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_common::Span;
use swc_ecmascript::ast::{CallExpr, Expr, ExprOrSpread, ExprOrSuper, NewExpr};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::VisitAll;
use swc_ecmascript::visit::VisitAllWith;

pub struct NoArrayConstructor;

const CODE: &str = "no-array-constructor";
const MESSAGE: &str = "Array Constructor is not allowed";
const HINT: &str = "Use array literal notation (e.g. []) or single argument specifying array size only (e.g. new Array(5)";

impl LintRule for NoArrayConstructor {
  fn new() -> Box<Self> {
    Box::new(NoArrayConstructor)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoArrayConstructorVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_array_constructor.md")
  }
}

struct NoArrayConstructorVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoArrayConstructorVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check_args(&mut self, args: Vec<ExprOrSpread>, span: Span) {
    if args.len() != 1 {
      self
        .context
        .add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
    }
  }
}

impl<'c, 'view> VisitAll for NoArrayConstructorVisitor<'c, 'view> {
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
    assert_lint_err! {
      NoArrayConstructor,
      "new Array": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Array()": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Array(x, y)": [{ col: 0, message: MESSAGE, hint: HINT }],
      "new Array(0, 1, 2)": [{ col: 0, message: MESSAGE, hint: HINT }],
      // nested
      r#"
const a = new class {
  foo() {
    let arr = new Array();
  }
}();
      "#: [{ line: 4, col: 14, message: MESSAGE, hint: HINT }],
      r#"
const a = (() => {
  let arr = new Array();
})();
      "#: [{ line: 3, col: 12, message: MESSAGE, hint: HINT }],
    }
  }
}
