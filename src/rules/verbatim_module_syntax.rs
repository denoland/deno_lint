// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::Program;
use deno_ast::swc::ast::{
  BindingIdent, ExportNamedSpecifier, Id, Ident, ImportDecl, ImportSpecifier,
  JSXElementName, ModuleExportName, NamedExport, TsEntityName,
  TsImportEqualsDecl, TsModuleRef,
};
use deno_ast::swc::common::collections::AHashSet;
use deno_ast::swc::visit::{noop_visit_type, Visit, VisitWith};
use deno_ast::view::NodeTrait;
use deno_ast::{
  view as ast_view, SourceRange, SourceRanged, SourceRangedForSpanned,
};
use derive_more::Display;

const CODE: &str = "verbatim-module-syntax";
const FIX_DESC: &str = "Add a type keyword";

#[allow(clippy::enum_variant_names)]
#[derive(Display)]
enum Message {
  #[display("All import identifiers are used in types")]
  AllImportIdentsUsedInTypes,
  #[display("Import identifier only used in types")]
  ImportIdentUsedInTypes,
  #[display("All export identifiers are used in types")]
  AllExportIdentsUsedInTypes,
  #[display("Export identifier only used in types")]
  ExportIdentUsedInTypes,
}

#[derive(Display)]
enum Hint {
  #[display(
    "Change `import` to `import type` and optionally add an explicit side effect import"
  )]
  ChangeImportToImportType,
  #[display("Change `export` to `export type`")]
  ChangeExportToExportType,
  #[display("Add a `type` keyword before the identifier")]
  AddTypeKeyword,
}

#[derive(Debug)]
pub struct VerbatimModuleSyntax;

impl VerbatimModuleSyntax {
  fn analyze_import(
    &self,
    import: &ast_view::ImportDecl,
    ids: &IdCollector,
    context: &mut Context,
    program: Program,
  ) {
    if import.type_only() || import.specifiers.is_empty() {
      return;
    }
    let mut type_only_usage = Vec::with_capacity(import.specifiers.len());
    let mut type_only_named_import =
      Vec::with_capacity(import.specifiers.len());
    for specifier in import.specifiers {
      match specifier {
        ast_view::ImportSpecifier::Named(named) => {
          if named.is_type_only() {
            type_only_named_import.push(named);
          } else if !ids.has_import_ident(&named.local.to_id()) {
            type_only_usage.push(specifier);
          }
        }
        ast_view::ImportSpecifier::Default(default) => {
          if !ids.has_import_ident(&default.local.to_id()) {
            type_only_usage.push(specifier);
          }
        }
        ast_view::ImportSpecifier::Namespace(namespace) => {
          if !ids.has_import_ident(&namespace.local.to_id()) {
            type_only_usage.push(specifier);
          }
        }
      }
    }
    if import.specifiers.len()
      == type_only_usage.len() + type_only_named_import.len()
    {
      let import_token_range = import.tokens_fast(program)[0].range();
      let mut changes = Vec::with_capacity(1 + type_only_named_import.len());
      changes.push(LintFixChange {
        new_text: " type".into(),
        range: import_token_range.end().range(),
      });
      for named_import in type_only_named_import {
        // remove `type` from all these
        let tokens = named_import.tokens_fast(program);
        let range = SourceRange::new(tokens[0].start(), tokens[1].start());
        changes.push(LintFixChange {
          new_text: "".into(),
          range,
        });
      }
      context.add_diagnostic_with_fixes(
        import_token_range,
        CODE,
        Message::AllImportIdentsUsedInTypes,
        Some(Hint::ChangeImportToImportType.to_string()),
        vec![LintFix {
          description: FIX_DESC.into(),
          changes,
        }],
      );
    } else {
      for specifier in type_only_usage {
        context.add_diagnostic_with_fixes(
          specifier.range(),
          CODE,
          Message::ImportIdentUsedInTypes,
          Some(Hint::AddTypeKeyword.to_string()),
          vec![LintFix {
            description: FIX_DESC.into(),
            changes: vec![LintFixChange {
              new_text: "type ".into(),
              range: specifier.start().range(),
            }],
          }],
        );
      }
    }
  }

  fn analyze_export(
    &self,
    named_export: &ast_view::NamedExport,
    ids: &IdCollector,
    context: &mut Context,
    program: Program,
  ) {
    if named_export.type_only()
      || named_export.specifiers.is_empty()
      || named_export.src.is_some()
    {
      return;
    }

    let mut type_only_usage = Vec::with_capacity(named_export.specifiers.len());
    let mut type_only_named_export =
      Vec::with_capacity(named_export.specifiers.len());
    for specifier in named_export.specifiers {
      match specifier {
        ast_view::ExportSpecifier::Named(named) => {
          if named.is_type_only() {
            type_only_named_export.push(named);
          } else if let ast_view::ModuleExportName::Ident(ident) = &named.orig {
            if !ids.has_export_ident(&ident.to_id()) {
              type_only_usage.push(specifier);
            }
          }
        }
        ast_view::ExportSpecifier::Default(_)
        | ast_view::ExportSpecifier::Namespace(_) => {
          // nothing to analyze
        }
      }
    }
    if named_export.specifiers.len()
      == type_only_usage.len() + type_only_named_export.len()
    {
      let export_token_range = named_export.tokens_fast(program)[0].range();
      let mut changes = Vec::with_capacity(1 + type_only_named_export.len());
      changes.push(LintFixChange {
        new_text: " type".into(),
        range: export_token_range.end().range(),
      });
      for named_import in type_only_named_export {
        // remove `type` from all these
        let tokens = named_import.tokens_fast(program);
        let range = SourceRange::new(tokens[0].start(), tokens[1].start());
        changes.push(LintFixChange {
          new_text: "".into(),
          range,
        });
      }
      context.add_diagnostic_with_fixes(
        export_token_range,
        CODE,
        Message::AllExportIdentsUsedInTypes,
        Some(Hint::ChangeExportToExportType.to_string()),
        vec![LintFix {
          description: FIX_DESC.into(),
          changes,
        }],
      );
    } else {
      for specifier in type_only_usage {
        context.add_diagnostic_with_fixes(
          specifier.range(),
          CODE,
          Message::ExportIdentUsedInTypes,
          Some(Hint::AddTypeKeyword.to_string()),
          vec![LintFix {
            description: FIX_DESC.into(),
            changes: vec![LintFixChange {
              new_text: "type ".into(),
              range: specifier.start().range(),
            }],
          }],
        );
      }
    }
  }
}

impl LintRule for VerbatimModuleSyntax {
  fn tags(&self) -> &'static [&'static str] {
    &["jsr"]
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
    let ids = IdCollector::build(module);

    for child in module.body {
      match child {
        ast_view::ModuleItem::ModuleDecl(module_decl) => match module_decl {
          ast_view::ModuleDecl::Import(import) => {
            self.analyze_import(import, &ids, context, program);
          }
          ast_view::ModuleDecl::ExportNamed(named_export) => {
            self.analyze_export(named_export, &ids, context, program);
          }
          ast_view::ModuleDecl::ExportDefaultDecl(_)
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

/// This struct is partly lifted and adapted from:
/// https://github.com/swc-project/swc/blob/d8186fb94efb150b50d96519f0b8c5740d15b92f/crates/swc_ecma_transforms_typescript/src/strip_import_export.rs#L9C1-L100C2
#[derive(Debug, Default)]
struct IdCollector {
  id_usage: AHashSet<Id>,
  export_value_id_usage: AHashSet<Id>,
  import_value_id_usage: AHashSet<Id>,
}

impl IdCollector {
  pub fn build(module: &ast_view::Module) -> Self {
    let mut ids = Self::default();
    module.inner.visit_with(&mut ids);
    ids
  }

  pub fn has_import_ident(&self, id: &Id) -> bool {
    self.id_usage.contains(id) || self.export_value_id_usage.contains(id)
  }

  pub fn has_export_ident(&self, id: &Id) -> bool {
    self.id_usage.contains(id) || self.import_value_id_usage.contains(id)
  }
}

impl Visit for IdCollector {
  noop_visit_type!();

  fn visit_ident(&mut self, n: &Ident) {
    self.id_usage.insert(n.to_id());
  }

  fn visit_binding_ident(&mut self, id: &BindingIdent) {
    // mark declarations as usages for export declarations
    self.id_usage.insert(id.id.to_id());
  }

  fn visit_import_decl(&mut self, n: &ImportDecl) {
    if n.type_only {
      return;
    }
    n.visit_children_with(self);
  }

  fn visit_import_specifier(&mut self, n: &ImportSpecifier) {
    match n {
      ImportSpecifier::Named(n) => {
        if !n.is_type_only {
          self.import_value_id_usage.insert(n.local.to_id());
        }
      }
      ImportSpecifier::Default(n) => {
        self.import_value_id_usage.insert(n.local.to_id());
      }
      ImportSpecifier::Namespace(n) => {
        self.import_value_id_usage.insert(n.local.to_id());
      }
    }
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

    match &n.orig {
      ModuleExportName::Ident(ident) => {
        self.export_value_id_usage.insert(ident.to_id());
      }
      ModuleExportName::Str(_) => {}
    }
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid() {
    assert_lint_ok! {
      VerbatimModuleSyntax,
      "import type { Type } from 'module'; type Test = Type;",
      "import type { Type, Other } from 'module'; type Test = Type | Other;",
      "import { type Type, value } from 'module'; type Test = Type; value();",
      "import * as value from 'module'; value();",
      "import type * as value from 'module'; type Test = typeof value;",
      "import value from 'module'; value();",
      "import type value from 'module'; type Test = typeof value;",
      "import { value } from 'module'; export { value };",
      "import type { value } from 'module'; export type { value };",
      "import { value, type Type } from 'module'; console.log(value); export type { Type };",
      "import { value, type Type } from 'module'; export { value, type Type };",
      "export { value } from './value.ts';",
      "const logger = { setItems }; export { logger };",
      "class Test {} export { Test };",
    };
  }

  #[test]
  fn invalid() {
    assert_lint_err! {
      VerbatimModuleSyntax,
      "import { Type } from 'module'; type Test = Type;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { Type } from 'module'; type Test = Type;"),
        }
      ],
      "import { Type, Other } from 'module'; type Test = Type | Other;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { Type, Other } from 'module'; type Test = Type | Other;"),
        }
      ],
      "import { type Type, Other } from 'module'; type Test = Type | Other;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { Type, Other } from 'module'; type Test = Type | Other;"),
        }
      ],
      "import { Type, value } from 'module'; type Test = Type; value();": [
        {
          col: 9,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { type Type, value } from 'module'; type Test = Type; value();"),
        }
      ],
      "import { Type, Other, value } from 'module'; type Test = Type | Other; value();": [
        {
          col: 9,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { type Type, Other, value } from 'module'; type Test = Type | Other; value();"),
        },
        {
          col: 15,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { Type, type Other, value } from 'module'; type Test = Type | Other; value();"),
        }
      ],
      "import * as value from 'module'; type Test = typeof value;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type * as value from 'module'; type Test = typeof value;"),
        }
      ],
      "import value from 'module'; type Test = typeof value;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type value from 'module'; type Test = typeof value;"),
        }
      ],
      "type Test = string; export { Test };": [
        {
          col: 20,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "type Test = string; export type { Test };"),
        }
      ],
      "type Test = string; type Test2 = string; export { Test, Test2 };": [
        {
          col: 41,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "type Test = string; type Test2 = string; export type { Test, Test2 };"),
        }
      ],
      "import { type value } from 'module'; export { value };": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { value } from 'module'; export { value };"),
        },
        {
          col: 37,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "import { type value } from 'module'; export type { value };"),
        }
      ],
      "import { value } from 'module'; export { type value };": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { value } from 'module'; export { type value };"),
        },
        {
          col: 32,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "import { value } from 'module'; export type { value };"),
        }
      ],
      "import type { value } from 'module'; export { value };": [
        {
          col: 37,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "import type { value } from 'module'; export type { value };"),
        }
      ],
      "import { value } from 'module'; export type { value };": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { value } from 'module'; export type { value };"),
        }
      ],
      "import { value, type Type } from 'module'; console.log(value); export { Type };": [
        {
          col: 63,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "import { value, type Type } from 'module'; console.log(value); export type { Type };"),
        }
      ],
      "import { value, Type } from 'module'; console.log(value); export { type Type };": [
        {
          col: 16,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { value, type Type } from 'module'; console.log(value); export { type Type };"),
        },
        {
          col: 58,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "import { value, Type } from 'module'; console.log(value); export type { Type };"),
        }
      ],
      "import { value, Type } from 'module'; console.log(value); export type { Type };": [
        {
          col: 16,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { value, type Type } from 'module'; console.log(value); export type { Type };"),
        }
      ],
      "import { value, type Type } from 'module'; export { value, Type };": [
        {
          col: 59,
          message: Message::ExportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { value, type Type } from 'module'; export { value, type Type };"),
        }
      ],
      "import { value, Type } from 'module'; export { value, type Type };": [
        {
          col: 16,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { value, type Type } from 'module'; export { value, type Type };"),
        }
      ],
      "interface Test {}\nexport { Test };": [
        {
          line: 2,
          col: 0,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "interface Test {}\nexport type { Test };"),
        }
      ],
      "type Test = 'test';\nexport { Test };": [
        {
          line: 2,
          col: 0,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_DESC, "type Test = 'test';\nexport type { Test };"),
        }
      ],
    };
  }
}
