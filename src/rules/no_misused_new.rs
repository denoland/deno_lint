// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_atoms::JsWord;
use swc_ecma_ast::{
  ClassDecl, ClassMember, Expr, Ident, Module, TsEntityName, TsInterfaceDecl,
  TsType, TsTypeAliasDecl, TsTypeAnn,
  TsTypeElement::{TsConstructSignatureDecl, TsMethodSignature},
};
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoMisusedNew;

impl LintRule for NoMisusedNew {
  fn new() -> Box<Self> {
    Box::new(NoMisusedNew)
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoMisusedNewVisitor::new(context);
    visitor.visit_module(&module, &module);
  }

  fn code(&self) -> &'static str {
    "no-misused-new"
  }
}

pub struct NoMisusedNewVisitor {
  context: Context,
}

impl NoMisusedNewVisitor {
  fn new(context: Context) -> Self {
    Self { context }
  }

  fn match_parent_type(&self, parent: &Ident, return_type: &TsTypeAnn) -> bool {
    if let TsType::TsTypeRef(type_ref) = &*return_type.type_ann {
      if let TsEntityName::Ident(ident) = &type_ref.type_name {
        return ident.sym == parent.sym;
      }
    }

    false
  }

  fn is_constructor_keyword(&self, ident: &Ident) -> bool {
    JsWord::from("constructor") == ident.sym
  }
}

impl Visit for NoMisusedNewVisitor {
  fn visit_ts_type_alias_decl(
    &mut self,
    t: &TsTypeAliasDecl,
    _parent: &dyn Node,
  ) {
    if let TsType::TsTypeLit(lit) = &*t.type_ann {
      for member in &lit.members {
        if let TsMethodSignature(signature) = &member {
          if let Expr::Ident(ident) = &*signature.key {
            if self.is_constructor_keyword(&ident) {
              self.context.add_diagnostic(
                ident.span,
                "no-misused-new",
                "Type aliases cannot be constructed, only classes",
              );
            }
          }
        }
      }
    }
  }

  fn visit_ts_interface_decl(
    &mut self,
    n: &TsInterfaceDecl,
    _parent: &dyn Node,
  ) {
    for member in &n.body.body {
      match &member {
        TsMethodSignature(signature) => {
          if let Expr::Ident(ident) = &*signature.key {
            if self.is_constructor_keyword(&ident) {
              // constructor
              self.context.add_diagnostic(
                signature.span,
                "no-misused-new",
                "Interfaces cannot be constructed, only classes",
              );
            }
          }
        }
        TsConstructSignatureDecl(signature) => {
          if signature.type_ann.is_some()
            && self
              .match_parent_type(&n.id, &signature.type_ann.as_ref().unwrap())
          {
            self.context.add_diagnostic(
              signature.span,
              "no-misused-new",
              "Interfaces cannot be constructed, only classes",
            );
          }
        }
        _ => {}
      }
    }
  }

  fn visit_class_decl(&mut self, expr: &ClassDecl, _parent: &dyn Node) {
    for member in &expr.class.body {
      if let ClassMember::Method(method) = member {
        if method.function.return_type.is_some()
          && self.match_parent_type(
            &expr.ident,
            &method.function.return_type.as_ref().unwrap(),
          )
        {
          // new
          self.context.add_diagnostic(
            method.span,
            "no-misused-new",
            "Class cannot have method named `new`.",
          );
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
  fn no_misused_new_invalid() {
    assert_lint_err_on_line_n::<NoMisusedNew>(
      r#"
interface I {
    new(): I;
    constructor(): void;
}
      "#,
      vec![(3, 4), (4, 4)],
    );

    assert_lint_err_on_line::<NoMisusedNew>(
      r#"
interface G {
    new<T>(): G<T>;
}
      "#,
      3,
      4,
    );

    assert_lint_err_on_line::<NoMisusedNew>(
      r#"
type T = {
    constructor(): void;
}
      "#,
      3,
      4,
    );

    assert_lint_err_on_line::<NoMisusedNew>(
      r#"
class C {
    new(): C;
}
      "#,
      3,
      4,
    );

    assert_lint_err_on_line::<NoMisusedNew>(
      r#"
declare abstract class C {
    new(): C;
}
      "#,
      3,
      4,
    )
  }

  #[test]
  fn no_misused_new_valid() {
    assert_lint_ok::<NoMisusedNew>("type T = { new(): T }");
    assert_lint_ok::<NoMisusedNew>("interface IC { new(): {} }");

    assert_lint_ok::<NoMisusedNew>("class C { new(): {} }");
    assert_lint_ok::<NoMisusedNew>("class C { constructor(); }");
    assert_lint_ok::<NoMisusedNew>("class C { constructor() {} }");
    assert_lint_ok::<NoMisusedNew>(
      r#"
    declare class DC {
        foo() {

        }
        get new()
        bar();
    }
      "#,
    )
  }
}
