use super::Context;
use super::LintRule;
use swc_atoms::JsWord;
use swc_ecma_ast::{
  ClassDecl, ClassMember, Expr, Ident, Module, TsEntityName, TsInterfaceDecl,
  TsType, TsTypeAnn,
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
    "noMisusedNew"
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
    match &*return_type.type_ann {
      TsType::TsTypeRef(type_ref) => {
        if let TsEntityName::Ident(ident) = &type_ref.type_name {
          return ident.sym == parent.sym;
        }
      }
      _ => {}
    }

    return false;
  }
}

impl Visit for NoMisusedNewVisitor {
  fn visit_ts_interface_decl(
    &mut self,
    n: &TsInterfaceDecl,
    _parent: &dyn Node,
  ) {
    for member in &n.body.body {
      match &member {
        TsMethodSignature(signature) => {
          if let Expr::Ident(ident) = &*signature.key {
            if JsWord::from("constructor") == ident.sym
              && signature.type_ann.is_some()
              && self
                .match_parent_type(&n.id, &signature.type_ann.as_ref().unwrap())
            {
              // constructor
              self.context.add_diagnostic(
                signature.span,
                "noMisusedNew",
                "Interfaces cannot be constructed, only classes",
              );
            }
          }
        }
        TsConstructSignatureDecl(signature) => {
          self.context.add_diagnostic(
            signature.span,
            "noMisusedNew",
            "Interfaces cannot be constructed, only classes",
          );
        }
        _ => {}
      }
    }
  }

  fn visit_class_decl(&mut self, expr: &ClassDecl, _parent: &dyn Node) {
    for member in &expr.class.body {
      match member {
        ClassMember::Method(method) => {
          if method.function.return_type.is_some()
            && self.match_parent_type(
              &expr.ident,
              &method.function.return_type.as_ref().unwrap(),
            )
          {
            // new
            self.context.add_diagnostic(
              method.span,
              "noMisusedNew",
              "Class cannot have method named `new`.",
            );
          }
        }
        _ => {}
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_misused_new_test() {
    assert_lint_err_on_line::<NoMisusedNew>(
      r#"
interface I {
    constructor(): I
}
           "#,
      3,
      4,
    );
    assert_lint_err_on_line::<NoMisusedNew>(
      r#"
class C {
    new(): C
}
           "#,
      3,
      4,
    );
    assert_lint_ok::<NoMisusedNew>("class C { new(): void }");
    assert_lint_ok::<NoMisusedNew>("interface IC { constructor(): void }");
  }
}
