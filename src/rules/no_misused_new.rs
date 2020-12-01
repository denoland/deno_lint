// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{
  ClassDecl, ClassMember, Expr, Ident, Program, PropName, TsEntityName,
  TsInterfaceDecl, TsType, TsTypeAliasDecl, TsTypeAnn,
  TsTypeElement::{TsConstructSignatureDecl, TsMethodSignature},
};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoMisusedNew;

impl LintRule for NoMisusedNew {
  fn new() -> Box<Self> {
    Box::new(NoMisusedNew)
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = NoMisusedNewVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-misused-new"
  }

  fn docs(&self) -> &'static str {
    r#"Disallows defining constructors for interfaces or new for classes

Specifying a constructor for an interface or defining a `new` method for a class
is incorrect and should be avoided.
    
### Invalid:
```typescript
class C {
  new(): C;
}

interface I {
  constructor(): void;
}
```

### Valid:
```typescript
class C {
  constructor() {}
}
interface I {
  new (): C;
}
```
"#
  }
}

struct NoMisusedNewVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoMisusedNewVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
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
    *"constructor" == ident.sym
  }
}

impl<'c> Visit for NoMisusedNewVisitor<'c> {
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
              self.context.add_diagnostic_with_hint(
                ident.span,
                "no-misused-new",
                "Type aliases cannot be constructed, only classes",
                "Consider using a class, not a type",
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
    parent: &dyn Node,
  ) {
    for member in &n.body.body {
      match &member {
        TsMethodSignature(signature) => {
          if let Expr::Ident(ident) = &*signature.key {
            if self.is_constructor_keyword(&ident) {
              // constructor
              self.context.add_diagnostic_with_hint(
                signature.span,
                "no-misused-new",
                "Interfaces cannot be constructed, only classes",
                "Consider using a class, not an interface",
              );
            }
          }
        }
        TsConstructSignatureDecl(signature) => {
          if signature.type_ann.is_some()
            && self
              .match_parent_type(&n.id, &signature.type_ann.as_ref().unwrap())
          {
            self.context.add_diagnostic_with_hint(
              signature.span,
              "no-misused-new",
              "Interfaces cannot be constructed, only classes",
              "Consider using a class, not an interface",
            );
          }
        }
        _ => {}
      }
    }

    swc_ecmascript::visit::visit_ts_interface_decl(self, n, parent);
  }

  fn visit_class_decl(&mut self, expr: &ClassDecl, parent: &dyn Node) {
    for member in &expr.class.body {
      if let ClassMember::Method(method) = member {
        let method_name = match &method.key {
          PropName::Ident(ident) => ident.sym.as_ref(),
          PropName::Str(str_) => str_.value.as_ref(),
          _ => continue,
        };

        if method_name != "new" {
          continue;
        }

        if method.function.return_type.is_some()
          && self.match_parent_type(
            &expr.ident,
            &method.function.return_type.as_ref().unwrap(),
          )
        {
          // new
          self.context.add_diagnostic_with_hint(
            method.span,
            "no-misused-new",
            "Class cannot have method named `new`.",
            "Rename the method",
          );
        }
      }
    }

    swc_ecmascript::visit::visit_class_decl(self, expr, parent);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_misused_new_valid() {
    assert_lint_ok! {
      NoMisusedNew,
      "type T = { new(): T }",
      "interface IC { new(): {} }",
      "class C { new(): {} }",
      "class C { constructor(); }",
      "class C { constructor() {} }",
      r#"
    export class Fnv32a extends Fnv32Base<Fnv32a> {
      write(data: Uint8Array): Fnv32a {
        let hash = this.sum32();
    
        data.forEach((c) => {
          hash ^= c;
          hash = mul32(hash, prime32);
        });
    
        this._updateState(hash);
        return this;
      }
    }
      "#,
    };
  }

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
class B {
    method() {
        interface T {
            new(): T
        }
    }
}
      "#,
      5,
      12,
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
class A {
  foo() {
    class C {
      new(): C;
    }
  }
}
      "#,
      5,
      6,
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
}
