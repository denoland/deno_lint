// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintQuickFix, LintQuickFixChange};
use crate::Program;
use deno_ast::swc::ast::{
  BindingIdent, ExportNamedSpecifier, Id, Ident, ImportDecl, JSXElementName,
  NamedExport, TsEntityName, TsExprWithTypeArgs, TsImportEqualsDecl,
  TsModuleRef,
};
use deno_ast::swc::common::collections::AHashSet;
use deno_ast::swc::visit::{noop_visit_type, Visit, VisitWith};
use deno_ast::view::NodeTrait;
use deno_ast::{
  view as ast_view, SourceRange, SourceRanged, SourceRangedForSpanned,
};
use derive_more::Display;

#[derive(Debug)]
pub struct VerbatimModuleSyntax;

const CODE: &str = "verbatim-module-syntax";

#[derive(Display)]
enum ConsistentTypeImportsMessage {
  #[display(fmt = "All import identifiers are used in type positions")]
  AllIdentsUsedInTypePositions,
  #[display(fmt = "Import identifier only used in type positions")]
  IdentUsedInTypePositions,
}

#[derive(Display)]
enum ConsistentTypeImportsHint {
  #[display(
    fmt = "Change `import` to `import type` and optionally add an explicit side effect import"
  )]
  ChangeImportToImportType,
  #[display(fmt = "Add a `type` keyword before the identifier")]
  AddTypeKeyword,
}

impl LintRule for VerbatimModuleSyntax {
  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    let module = match program.program() {
      Program::Module(module) => module,
      Program::Script(_) => return,
    };
    let mut usages = UsageCollect::default();
    module.inner.visit_with(&mut usages);

    for child in module.body {
      match child {
        ast_view::ModuleItem::ModuleDecl(module_decl) => match module_decl {
          ast_view::ModuleDecl::Import(import) => {
            if !import.type_only() && !import.specifiers.is_empty() {
              let mut type_only_usage =
                Vec::with_capacity(import.specifiers.len());
              let mut type_only_named_import =
                Vec::with_capacity(import.specifiers.len());
              for specifier in import.specifiers {
                match specifier {
                  ast_view::ImportSpecifier::Named(named) => {
                    if named.is_type_only() {
                      type_only_named_import.push(named);
                    } else if !usages.id_usage.contains(&named.local.to_id()) {
                      type_only_usage.push(specifier);
                    }
                  }
                  ast_view::ImportSpecifier::Default(default) => {
                    if !usages.id_usage.contains(&default.local.to_id()) {
                      type_only_usage.push(specifier);
                    }
                  }
                  ast_view::ImportSpecifier::Namespace(namespace) => {
                    if !usages.id_usage.contains(&namespace.local.to_id()) {
                      type_only_usage.push(specifier);
                    }
                  }
                }
              }
              if import.specifiers.len()
                == type_only_usage.len() + type_only_named_import.len()
              {
                let import_token_range = import.tokens_fast(program)[0].range();
                let mut changes =
                  Vec::with_capacity(1 + type_only_named_import.len());
                changes.push(LintQuickFixChange {
                  new_text: " type".to_string(),
                  range: import_token_range.end().range(),
                });
                for named_import in type_only_named_import {
                  // remove `type` from all these
                  let tokens = named_import.tokens_fast(program);
                  let range =
                    SourceRange::new(tokens[0].start(), tokens[1].start());
                  changes.push(LintQuickFixChange {
                    new_text: "".to_string(),
                    range,
                  });
                }
                context.add_diagnostic_with_quick_fixes(
                  import_token_range,
                  CODE,
                  ConsistentTypeImportsMessage::AllIdentsUsedInTypePositions,
                  Some(
                    ConsistentTypeImportsHint::ChangeImportToImportType
                      .to_string(),
                  ),
                  vec![LintQuickFix {
                    description: "Add a type keyword".to_string(),
                    changes,
                  }],
                );
              } else {
                for specifier in type_only_usage {
                  context.add_diagnostic_with_quick_fixes(
                    specifier.range(),
                    CODE,
                    ConsistentTypeImportsMessage::IdentUsedInTypePositions,
                    Some(ConsistentTypeImportsHint::AddTypeKeyword.to_string()),
                    vec![LintQuickFix {
                      description: "Add a type keyword".to_string(),
                      changes: vec![LintQuickFixChange {
                        new_text: "type ".to_string(),
                        range: specifier.start().range(),
                      }],
                    }],
                  );
                }
              }
            }
          }
          ast_view::ModuleDecl::ExportNamed(_)
          | ast_view::ModuleDecl::ExportDefaultDecl(_)
          | ast_view::ModuleDecl::ExportDefaultExpr(_)
          | ast_view::ModuleDecl::ExportAll(_)
          | ast_view::ModuleDecl::TsImportEquals(_)
          | ast_view::ModuleDecl::TsExportAssignment(_)
          | ast_view::ModuleDecl::TsNamespaceExport(_)
          | ast_view::ModuleDecl::ExportDecl(_) => {}
        },
        ast_view::ModuleItem::Stmt(_) => {}
      }
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/verbatim_module_syntax.md")
  }
}

/// This struct is lifted and adapted from:
/// https://github.com/swc-project/swc/blob/d8186fb94efb150b50d96519f0b8c5740d15b92f/crates/swc_ecma_transforms_typescript/src/strip_import_export.rs#L9C1-L100C2
#[derive(Debug, Default)]
struct UsageCollect {
  id_usage: AHashSet<Id>,
}

impl Visit for UsageCollect {
  noop_visit_type!();

  fn visit_ident(&mut self, n: &Ident) {
    self.id_usage.insert(n.to_id());
  }

  fn visit_binding_ident(&mut self, _: &BindingIdent) {
    // skip
  }

  fn visit_ts_expr_with_type_args(&mut self, _: &TsExprWithTypeArgs) {
    // skip, not ignored by noop_visit_type for some reason
    // (see https://github.com/swc-project/swc/discussions/8669)
  }

  fn visit_import_decl(&mut self, _: &ImportDecl) {
    // skip
  }

  fn visit_ts_import_equals_decl(&mut self, n: &TsImportEqualsDecl) {
    if n.is_type_only {
      return;
    }

    // skip id visit

    let TsModuleRef::TsEntityName(ts_entity_name) = &n.module_ref else {
      return;
    };

    get_module_ident(ts_entity_name).visit_with(self);
  }

  fn visit_export_named_specifier(&mut self, n: &ExportNamedSpecifier) {
    if n.is_type_only {
      return;
    }

    n.orig.visit_with(self);
  }

  fn visit_named_export(&mut self, n: &NamedExport) {
    if n.type_only || n.src.is_some() {
      return;
    }

    n.visit_children_with(self);
  }

  fn visit_jsx_element_name(&mut self, n: &JSXElementName) {
    if matches!(n, JSXElementName::Ident(i) if i.sym.starts_with(|c: char| c.is_ascii_lowercase()) )
    {
      return;
    }

    n.visit_children_with(self);
  }
}

fn get_module_ident(ts_entity_name: &TsEntityName) -> &Ident {
  match ts_entity_name {
    TsEntityName::TsQualifiedName(ts_qualified_name) => {
      get_module_ident(&ts_qualified_name.left)
    }
    TsEntityName::Ident(ident) => ident,
  }
}

#[derive(Debug, Default)]
struct AllIdents {
  idents: AHashSet<Id>,
}

impl Visit for AllIdents {
  fn visit_ident(&mut self, n: &Ident) {
    self.idents.insert(n.to_id());
  }
}

// most tests are taken from ESlint, commenting those
// requiring code path support
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn constructor_super_valid() {
    assert_lint_ok! {
      VerbatimModuleSyntax,
      // non derived classes.
      "class A { }",
      "class A { constructor() { } }",

      // inherit from non constructors.
      // those are valid if we don't define the constructor.
      "class A extends null { }",

      // derived classes.
      "class A extends B { }",
      "class A extends B { constructor() { super(); } }",

      // TODO(magurotuna): control flow analysis is required to handle these cases
      // "class A extends B { constructor() { if (true) { super(); } else { super(); } } }",
      // "class A extends B { constructor() { a ? super() : super(); } }",
      // "class A extends B { constructor() { if (a) super(); else super(); } }",
      // "class A extends B { constructor() { switch (a) { case 0: super(); break; default: super(); } } }",
      // "class A extends B { constructor() { try {} finally { super(); } } }",
      // "class A extends B { constructor() { if (a) throw Error(); super(); } }",

      // derived classes.
      "class A extends (class B {}) { constructor() { super(); } }",
      "class A extends (B = C) { constructor() { super(); } }",
      "class A extends (B || C) { constructor() { super(); } }",
      "class A extends (a ? B : C) { constructor() { super(); } }",
      "class A extends (B, C) { constructor() { super(); } }",

      // nested.
      "class A { constructor() { class B extends C { constructor() { super(); } } } }",
      "class A extends B { constructor() { super(); class C extends D { constructor() { super(); } } } }",
      "class A extends B { constructor() { super(); class C { constructor() { } } } }",

      // returning value is a substitute of 'super()'.
      "class A extends B { constructor() { if (true) return a; super(); } }",
      "class A extends null { constructor() { return a; } }",
      "class A { constructor() { return a; } }",

      // https://github.com/eslint/eslint/issues/5261
      "class A extends B { constructor(a) { super(); for (const b of a) { this.a(); } } }",

      // https://github.com/eslint/eslint/issues/5319
      "class Foo extends Object { constructor(method) { super(); this.method = method || function() {}; } }",

      // https://github.com/denoland/deno_lint/issues/464
      "declare class DOMException extends Error {
        constructor(message?: string, name?: string);
      }"
    };
  }

  #[test]
  fn constructor_super_invalid() {
    assert_lint_err! {
      VerbatimModuleSyntax,
      "class A { constructor() { super(); } }": [
        {
          col: 26,
          message: MESSAGE,
          hint: HINT,
        }
      ],
    };
  }
}
