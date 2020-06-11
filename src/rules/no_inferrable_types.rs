// Copyright 2020 the Deno authors. All rights reserved. MIT license.
#![allow(unused)]

use super::Context;
use super::LintRule;
use swc_ecma_ast::{Expr, Lit, TsKeywordType, VarDecl};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;
use swc_atoms::JsWord;
pub struct NoInferrableTypes;

impl LintRule for NoInferrableTypes {
  fn new() -> Box<Self> {
    Box::new(NoInferrableTypes)
  }

  fn code(&self) -> &'static str {
    "no-this-alias"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoInferrableTypesVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoInferrableTypesVisitor {
  context: Context,
}

impl NoInferrableTypesVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoInferrableTypesVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if !var_decl.decls[0].init.is_some() {
      return;
    }
    if let swc_ecma_ast::Pat::Ident(ident) = &var_decl.decls[0].name {
      if let Some(type_ann) = &ident.type_ann {
        if let swc_ecma_ast::TsType::TsKeywordType(ts_type) =
          &*type_ann.type_ann
        {
          if let Some(init) = &var_decl.decls[0].init {
            use swc_ecma_ast::TsKeywordTypeKind::*;
            match ts_type.kind {
              TsBigIntKeyword => match &**init {
                Expr::Lit(Lit::BigInt(value)) => self.context.add_diagnostic(
                  var_decl.span,
                  "no-inferrable-types",
                  "inferrable types are not allowed",
                ),
                Expr::Call(swc_ecma_ast::CallExpr { callee, .. }) => {
                    if let swc_ecma_ast::ExprOrSuper::Expr(unboxed) = &*callee {
                      if let Expr::Ident(value) = &**unboxed {
                        if value.sym == JsWord::from("BigInt") {
                          self.context.add_diagnostic(
                            var_decl.span,
                            "no-inferrable-types",
                            "inferrable types are not allowed",
                          )
                        }
                      }
                    }
                  },
                Expr::Unary(swc_ecma_ast::UnaryExpr { arg, .. }) => {
                  if let Expr::Lit(Lit::BigInt(_)) = &**arg {
                    self.context.add_diagnostic(
                      var_decl.span,
                      "no-inferrable-types",
                      "inferrable types are not allowed",
                    );
                  }
                  if let Expr::Call(swc_ecma_ast::CallExpr { callee, .. }) =
                    &**arg
                  {
                    if let swc_ecma_ast::ExprOrSuper::Expr(unboxed) = &*callee {
                      if let Expr::Ident(value) = &**unboxed {
                        if value.sym == JsWord::from("BigInt") {
                          self.context.add_diagnostic(
                            var_decl.span,
                            "no-inferrable-types",
                            "inferrable types are not allowed",
                          )
                        }
                      }
                    }
                  }
                }
                x @ _ => println!("{:?}", x),
              }
              _ => {}
            }
          }
        }
      } else {
        //TODO(@disizali) hadnle function expressions.
        return;
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_this_alias_valid() {
    assert_lint_ok::<NoInferrableTypes>("const self = foo(this);");
    assert_lint_ok::<NoInferrableTypes>("const self = 'this';");
  }

  #[test]
  fn no_this_alias_invalid() {
    assert_lint_err::<NoInferrableTypes>("const self = this;", 0);
    assert_lint_err::<NoInferrableTypes>("const { props, state } = this;", 0);
    assert_lint_err_on_line_n::<NoInferrableTypes>(
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
    assert_lint_err_on_line_n::<NoInferrableTypes>(
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
