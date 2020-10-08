// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use swc_ecmascript::ast::{
  ArrowExpr, Class, ClassMember, Decl, DefaultDecl, Expr, Function, Module,
  ModuleDecl, Pat, TsKeywordTypeKind, TsType, TsTypeAnn, VarDecl,
};

pub struct ExplicitModuleBoundaryTypes;

impl LintRule for ExplicitModuleBoundaryTypes {
  fn new() -> Box<Self> {
    Box::new(ExplicitModuleBoundaryTypes)
  }

  fn code(&self) -> &'static str {
    "explicit-module-boundary-types"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = ExplicitModuleBoundaryTypesVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct ExplicitModuleBoundaryTypesVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> ExplicitModuleBoundaryTypesVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
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
      self.context.add_diagnostic(
        function.span,
        "explicit-module-boundary-types",
        "Missing return type on function",
      );
    }
    for param in &function.params {
      self.check_pat(&param.pat);
    }
  }

  fn check_arrow(&mut self, arrow: &ArrowExpr) {
    if arrow.return_type.is_none() {
      self.context.add_diagnostic(
        arrow.span,
        "explicit-module-boundary-types",
        "Missing return type on function",
      );
    }
    for pat in &arrow.params {
      self.check_pat(&pat);
    }
  }

  fn check_ann(&mut self, ann: &Option<TsTypeAnn>, span: Span) {
    if let Some(ann) = ann {
      let ts_type = ann.type_ann.as_ref();
      if let TsType::TsKeywordType(keyword_type) = ts_type {
        if TsKeywordTypeKind::TsAnyKeyword == keyword_type.kind {
          self.context.add_diagnostic(
            span,
            "explicit-module-boundary-types",
            "All arguments should be typed",
          );
        }
      }
    } else {
      self.context.add_diagnostic(
        span,
        "explicit-module-boundary-types",
        "All arguments should be typed",
      );
    }
  }

  fn check_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(ident) => self.check_ann(&ident.type_ann, ident.span),
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

impl<'c> Visit for ExplicitModuleBoundaryTypesVisitor<'c> {
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
  use crate::test_util::*;

  #[test]
  fn explicit_module_boundary_types_valid() {
    assert_lint_ok_n::<ExplicitModuleBoundaryTypes>(vec![
      "function test() { return }",
      "export var fn = function (): number { return 1; }",
      "export var arrowFn = (arg: string): string => `test ${arg}`",
      "export var arrowFn = (arg: unknown): string => `test ${arg}`",
      "class Test { method() { return; } }",
    ]);
  }

  #[test]
  fn explicit_module_boundary_types_invalid() {
    assert_lint_err::<ExplicitModuleBoundaryTypes>(
      "export function test() { return; }",
      7,
    );
    assert_lint_err::<ExplicitModuleBoundaryTypes>(
      "export default function () { return 1; }",
      15,
    );
    assert_lint_err::<ExplicitModuleBoundaryTypes>(
      "export var arrowFn = () => 'test';",
      21,
    );
    assert_lint_err::<ExplicitModuleBoundaryTypes>(
      "export var arrowFn = (arg): string => `test ${arg}`;",
      22,
    );
    assert_lint_err::<ExplicitModuleBoundaryTypes>(
      "export var arrowFn = (arg: any): string => `test ${arg}`;",
      22,
    );
    assert_lint_err::<ExplicitModuleBoundaryTypes>(
      "export class Test { method() { return; } }",
      20,
    );
  }
}
