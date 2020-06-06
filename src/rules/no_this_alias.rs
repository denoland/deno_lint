// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::{Expr, VarDecl, ArrowExpr, Function };
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
      if let Expr::This(_) = **init {
        self.context.add_diagnostic(
          var_decl.span,
          "no-this-alias",
          "assign `this` to declare a value is not allowed",
        );
      }
    }
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node){
    println!("arrow : {:?}",arrow_expr);
  }
  
  fn visit_function(&mut self, fnn: &Function, _parent: &dyn Node){
    println!("fn : {:?}",fnn);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn test_literal_octal() {
    assert_lint_err::<NoThisAlias>("07", 0);
  }

  #[test]
  fn test_operand_octal() {
    assert_lint_err::<NoThisAlias>("let x = 7 + 07", 12);
  }

  #[test]
  fn test_octals_valid() {
    assert_lint_ok_n::<NoThisAlias>(vec!["7", "\"07\"", "0x08", "-0.01"]);
  }
}
