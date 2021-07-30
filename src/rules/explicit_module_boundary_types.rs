// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_common::Span;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use swc_ecmascript::ast::{
  ArrowExpr, Class, ClassMember, Decl, DefaultDecl, Expr, Function, ModuleDecl,
  Pat, TsKeywordTypeKind, TsType, TsTypeAnn, VarDecl,
};

pub struct ExplicitModuleBoundaryTypes;

const CODE: &str = "explicit-module-boundary-types";

#[derive(Display)]
enum ExplicitModuleBoundaryTypesMessage {
  #[display(fmt = "Missing return type on function")]
  MissingRetType,

  #[display(fmt = "All arguments should be typed")]
  MissingArgType,
}

#[derive(Display)]
enum ExplicitModuleBoundaryTypesHint {
  #[display(fmt = "Add a return type to the function signature")]
  AddRetType,

  #[display(fmt = "Add types to all the function arguments")]
  AddArgTypes,
}

impl LintRule for ExplicitModuleBoundaryTypes {
  fn new() -> Box<Self> {
    Box::new(ExplicitModuleBoundaryTypes)
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = ExplicitModuleBoundaryTypesVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Requires all module exports to have fully typed declarations

Having fully typed function arguments and return values clearly defines the
inputs and outputs of a module (known as the module boundary).  This will make
it very clear to any users of the module how to supply inputs and handle
outputs in a type safe manner.

### Invalid:

```typescript
// Missing return type (e.g. void)
export function printDoc(doc: string, doubleSided: boolean) { return; }

// Missing argument type (e.g. `arg` is of type string)
export const arrowFn = (arg): string => `hello ${arg}`;

// Missing return type (e.g. boolean)
export function isValid() {
  return true;
}
```

### Valid:

```typescript
// Typed input parameters and return value
export function printDoc(doc: string, doubleSided: boolean): void { return; }

// Input of type string and a return value of type string
export const arrowFn = (arg: string): string => `hello ${arg}`;

// Though lacking a return type, this is valid as it is not exported
function isValid() {
  return true;
}
```
"#
  }
}

struct ExplicitModuleBoundaryTypesVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> ExplicitModuleBoundaryTypesVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check_class(&mut self, class: &Class) {
    for member in &class.body {
      if let ClassMember::Method(method) = member {
        self.check_fn(&method.function);
      }
    }
  }

  fn check_fn(&mut self, function: &Function) {
    if function.return_type.is_none() {
      self.context.add_diagnostic_with_hint(
        function.span,
        CODE,
        ExplicitModuleBoundaryTypesMessage::MissingRetType,
        ExplicitModuleBoundaryTypesHint::AddRetType,
      );
    }
    for param in &function.params {
      self.check_pat(&param.pat);
    }
  }

  fn check_arrow(&mut self, arrow: &ArrowExpr) {
    if arrow.return_type.is_none() {
      self.context.add_diagnostic_with_hint(
        arrow.span,
        CODE,
        ExplicitModuleBoundaryTypesMessage::MissingRetType,
        ExplicitModuleBoundaryTypesHint::AddRetType,
      );
    }
    for pat in &arrow.params {
      self.check_pat(pat);
    }
  }

  fn check_ann(&mut self, ann: &Option<TsTypeAnn>, span: Span) {
    if let Some(ann) = ann {
      let ts_type = ann.type_ann.as_ref();
      if let TsType::TsKeywordType(keyword_type) = ts_type {
        if TsKeywordTypeKind::TsAnyKeyword == keyword_type.kind {
          self.context.add_diagnostic_with_hint(
            span,
            CODE,
            ExplicitModuleBoundaryTypesMessage::MissingArgType,
            ExplicitModuleBoundaryTypesHint::AddArgTypes,
          );
        }
      }
    } else {
      self.context.add_diagnostic_with_hint(
        span,
        CODE,
        ExplicitModuleBoundaryTypesMessage::MissingArgType,
        ExplicitModuleBoundaryTypesHint::AddArgTypes,
      );
    }
  }

  fn check_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(ident) => self.check_ann(&ident.type_ann, ident.id.span),
      Pat::Array(array) => self.check_ann(&array.type_ann, array.span),
      Pat::Rest(rest) => self.check_ann(&rest.type_ann, rest.span),
      Pat::Object(object) => self.check_ann(&object.type_ann, object.span),
      Pat::Assign(assign) => self.check_ann(&assign.type_ann, assign.span),
      _ => {}
    };
  }

  fn check_var_decl(&mut self, var: &VarDecl) {
    for declarator in &var.decls {
      if let Some(expr) = &declarator.init {
        if let Expr::Arrow(arrow) = expr.as_ref() {
          self.check_arrow(arrow);
        }
      }
    }
  }
}

impl<'c, 'view> Visit for ExplicitModuleBoundaryTypesVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_module_decl(
    &mut self,
    module_decl: &ModuleDecl,
    _parent: &dyn Node,
  ) {
    match module_decl {
      ModuleDecl::ExportDecl(export) => match &export.decl {
        Decl::Class(decl) => self.check_class(&decl.class),
        Decl::Fn(decl) => self.check_fn(&decl.function),
        Decl::Var(var) => self.check_var_decl(var),
        _ => {}
      },
      ModuleDecl::ExportDefaultDecl(export) => match &export.decl {
        DefaultDecl::Class(expr) => self.check_class(&expr.class),
        DefaultDecl::Fn(expr) => self.check_fn(&expr.function),
        _ => {}
      },
      _ => {}
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn explicit_module_boundary_types_valid() {
    assert_lint_ok! {
      ExplicitModuleBoundaryTypes,
      "function test() { return }",
      "export var fn = function (): number { return 1; }",
      "export var arrowFn = (arg: string): string => `test ${arg}`",
      "export var arrowFn = (arg: unknown): string => `test ${arg}`",
      "class Test { method() { return; } }",
    };
  }

  #[test]
  fn explicit_module_boundary_types_invalid() {
    assert_lint_err! {
      ExplicitModuleBoundaryTypes,

      r#"export function test() { return; }"#: [
      {
        col: 7,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export default function () { return 1; }"#: [
      {
        col: 15,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export var arrowFn = () => 'test';"#: [
      {
        col: 21,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
      r#"export var arrowFn = (arg): string => `test ${arg}`;"#: [
      {
        col: 22,
        message: ExplicitModuleBoundaryTypesMessage::MissingArgType,
        hint: ExplicitModuleBoundaryTypesHint::AddArgTypes,
      }],
      r#"export var arrowFn = (arg: any): string => `test ${arg}`;"#: [
      {
        col: 22,
        message: ExplicitModuleBoundaryTypesMessage::MissingArgType,
        hint: ExplicitModuleBoundaryTypesHint::AddArgTypes,
      }],
      r#"export class Test { method() { return; } }"#: [
      {
        col: 20,
        message: ExplicitModuleBoundaryTypesMessage::MissingRetType,
        hint: ExplicitModuleBoundaryTypesHint::AddRetType,
      }],
    }
  }
}
