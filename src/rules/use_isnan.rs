// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct UseIsNaN;

impl LintRule for UseIsNaN {
  fn new() -> Box<Self> {
    Box::new(UseIsNaN)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "use-isnan"
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = UseIsNaNVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }
}

struct UseIsNaNVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> UseIsNaNVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

fn is_nan_identifier(ident: &swc_ecmascript::ast::Ident) -> bool {
  ident.sym == *"NaN"
}

impl<'c, 'view> Visit for UseIsNaNVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_bin_expr(
    &mut self,
    bin_expr: &swc_ecmascript::ast::BinExpr,
    _parent: &dyn Node,
  ) {
    if bin_expr.op == swc_ecmascript::ast::BinaryOp::EqEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::NotEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::EqEqEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::NotEqEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::Lt
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::LtEq
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::Gt
      || bin_expr.op == swc_ecmascript::ast::BinaryOp::GtEq
    {
      if let swc_ecmascript::ast::Expr::Ident(ident) = &*bin_expr.left {
        if is_nan_identifier(&ident) {
          self.context.add_diagnostic(
            bin_expr.span,
            "use-isnan",
            "Use the isNaN function to compare with NaN",
          );
        }
      }
      if let swc_ecmascript::ast::Expr::Ident(ident) = &*bin_expr.right {
        if is_nan_identifier(&ident) {
          self.context.add_diagnostic(
            bin_expr.span,
            "use-isnan",
            "Use the isNaN function to compare with NaN",
          );
        }
      }
    }
  }

  fn visit_switch_stmt(
    &mut self,
    switch_stmt: &swc_ecmascript::ast::SwitchStmt,
    _parent: &dyn Node,
  ) {
    if let swc_ecmascript::ast::Expr::Ident(ident) = &*switch_stmt.discriminant
    {
      if is_nan_identifier(&ident) {
        self.context.add_diagnostic(
          switch_stmt.span,
          "use-isnan",
          "'switch(NaN)' can never match a case clause. Use Number.isNaN instead of the switch",
        );
      }
    }

    for case in &switch_stmt.cases {
      if let Some(expr) = &case.test {
        if let swc_ecmascript::ast::Expr::Ident(ident) = &**expr {
          if is_nan_identifier(ident) {
            self.context.add_diagnostic(
              case.span,
              "use-isnan",
              "'case NaN' can never match. Use Number.isNaN before the switch",
            );
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn use_isnan_invalid() {
    assert_lint_err::<UseIsNaN>("42 === NaN", 0);
    assert_lint_err_on_line_n::<UseIsNaN>(
      r#"
switch (NaN) {
  case NaN:
    break;
  default:
    break;
}
      "#,
      vec![(2, 0), (3, 2)],
    );
  }
}
