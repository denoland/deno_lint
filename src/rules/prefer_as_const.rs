// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]


use super::Context;
use super::LintRule;
use swc_atoms::JsWord;
use swc_ecma_ast::{Expr, Lit, TsAsExpr, TsLit, TsLitType, VarDecl};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct PreferAsConst;

impl LintRule for PreferAsConst {
  fn new() -> Box<Self> {
    Box::new(PreferAsConst)
  }

  fn code(&self) -> &'static str {
    "prefer-as-const"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = PreferAsConstVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct PreferAsConstVisitor {
  context: Context,
}

impl PreferAsConstVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }

  fn compare_statements(
    &self,
    span: swc_common::Span,
    st1: &JsWord,
    st2: &JsWord,
  ) {
    if st1 == st2 {
      self.context.add_diagnostic(
        span,
        "prefer-as-const",
        "please prefer as const",
      );
    }
  }
}

impl Visit for PreferAsConstVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if let Some(init) = &var_decl.decls[0].init {
      if let swc_ecma_ast::Pat::Ident(ident) = &var_decl.decls[0].name {
        if let Some(swc_ecma_ast::TsTypeAnn { type_ann, .. }) = &ident.type_ann
        {
          if let swc_ecma_ast::TsType::TsLitType(lit_type) = &**type_ann {
            if let swc_ecma_ast::Expr::Lit(lit) = &**init {
              match (lit, &lit_type.lit) {
                (Lit::Str(st1), TsLit::Str(st2)) => {
                  self.compare_statements(var_decl.span, &st1.value, &st2.value)
                }
                _ => return,
              }
            }
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
  fn no_var_test() {
    assert_lint_err::<PreferAsConst>(
      r#"var someVar = "someString"; const c = "c"; let a = "a";"#,
      0,
    );
  }
}
