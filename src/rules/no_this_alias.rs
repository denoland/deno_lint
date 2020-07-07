// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::{ArrowExpr, Expr, Function, VarDecl};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoThisAlias;

impl LintRule for NoThisAlias {
  fn new() -> Box<Self> {
    Box::new(NoThisAlias)
  }

  fn code(&self) -> &'static str {
    "no-this-alias"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoThisAliasVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoThisAliasVisitor {
  context: Context,
}

impl NoThisAliasVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoThisAliasVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if let Some(init) = &var_decl.decls[0].init {
      if let Expr::Arrow(arrow) = &**init {
        self.visit_arrow_expr(&arrow, _parent);
      } else if let Expr::This(_) = &**init {
        self.context.add_diagnostic(
          var_decl.span,
          "no-this-alias",
          "assign `this` to declare a value is not allowed",
        );
      }
    }
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.visit_block_stmt_or_expr(&arrow_expr.body, _parent);
  }

  fn visit_expr_stmt(
    &mut self,
    expr: &swc_ecma_ast::ExprStmt,
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
  }

  #[test]
  fn no_this_alias_invalid() {
    assert_lint_err::<NoThisAlias>("const self = this;", 0);
    assert_lint_err::<NoThisAlias>("const { props, state } = this;", 0);
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
    const { act } = this;
    const { act, constructor } = this;
    const [foo] = this;
    const [foo, bar] = this;
  }
}",
      vec![(4, 4), (5, 4), (13, 4), (14, 4), (15, 4), (16, 4), (17, 4)],
    );
  }
}
