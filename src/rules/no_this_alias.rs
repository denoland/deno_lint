// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{ArrowExpr, Expr, Function, Pat, VarDecl};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoThisAlias;

impl LintRule for NoThisAlias {
  fn new() -> Box<Self> {
    Box::new(NoThisAlias)
  }

  fn code(&self) -> &'static str {
    "no-this-alias"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecmascript::ast::Module) {
    let mut visitor = NoThisAliasVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoThisAliasVisitor {
  context: Arc<Context>,
}

impl NoThisAliasVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for NoThisAliasVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    for decl in &var_decl.decls {
      if let Some(init) = &decl.init {
        if let Expr::Arrow(arrow) = &**init {
          self.visit_arrow_expr(&arrow, _parent);
        } else if let Expr::This(_) = &**init {
          if let Pat::Ident(_ident) = &decl.name {
            self.context.add_diagnostic(
              var_decl.span,
              "no-this-alias",
              "assign `this` to declare a value is not allowed",
            );
          }
        }
      }
    }
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.visit_block_stmt_or_expr(&arrow_expr.body, _parent);
  }

  fn visit_expr_stmt(
    &mut self,
    expr: &swc_ecmascript::ast::ExprStmt,
    _parent: &dyn Node,
  ) {
    if let Expr::Arrow(arrow) = &*expr.expr {
      self.visit_arrow_expr(arrow, _parent);
    }
  }

  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    if let Some(stmt) = &function.body {
      self.visit_block_stmt(stmt, _parent);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_this_alias_valid() {
    assert_lint_ok::<NoThisAlias>("const self = foo(this);");
    assert_lint_ok::<NoThisAlias>("const self = 'this';");
    assert_lint_ok::<NoThisAlias>("const { props, state } = this;");
    assert_lint_ok::<NoThisAlias>("const [foo] = this;");
  }

  #[test]
  fn no_this_alias_invalid() {
    assert_lint_err::<NoThisAlias>("const self = this;", 0);
    assert_lint_err_on_line_n::<NoThisAlias>(
      "
var unscoped = this;

function testFunction() {
  let inFunction = this;
}

const testLambda = () => {
  const inLambda = this;
};",
      vec![(2, 0), (5, 2), (9, 2)],
    );
    assert_lint_err_on_line_n::<NoThisAlias>(
      "
class TestClass {
  constructor() {
    const inConstructor = this;
    const asThis: this = this;
      
    const asString = 'this';
    const asArray = [this];
    const asArrayString = ['this'];
  }
      
  public act(scope: this = this) {
    const inMemberFunction = this;
  }
}",
      vec![(4, 4), (5, 4), (13, 4)],
    );
  }
}
