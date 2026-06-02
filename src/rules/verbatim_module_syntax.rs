// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::span::{GetSpan, Span};
use derive_more::Display;

const CODE: &str = "verbatim-module-syntax";
const FIX_ADD_TYPE_KEYWORD_DESC: &str = "Add a type keyword";
const FIX_USE_IMPORT_DECL_DESC: &str = "Use an import declaration";

#[allow(clippy::enum_variant_names)]
#[derive(Display)]
enum Message {
  #[display(fmt = "All import identifiers are used in types")]
  AllImportIdentsUsedInTypes,
  #[display(fmt = "Import identifier only used in types")]
  ImportIdentUsedInTypes,
  #[display(fmt = "All export identifiers are used in types")]
  AllExportIdentsUsedInTypes,
  #[display(fmt = "Export identifier only used in types")]
  ExportIdentUsedInTypes,
  #[display(
    fmt = "Empty export declaration elided without verbatim-module-syntax"
  )]
  ExportDeclarationElided,
}

#[derive(Display)]
enum Hint {
  #[display(
    fmt = "Change `import` to `import type` and optionally add an explicit side effect import"
  )]
  ChangeImportToImportType,
  #[display(fmt = "Change `export` to `export type`")]
  ChangeExportToExportType,
  #[display(fmt = "Add a `type` keyword before the identifier")]
  AddTypeKeyword,
  #[display(fmt = "Change to side effect import for consistent behavior")]
  ChangeSideEffectImport,
}

#[derive(Debug)]
pub struct VerbatimModuleSyntax;

impl VerbatimModuleSyntax {
  fn analyze_import(
    &self,
    import: &ImportDeclaration,
    ids: &IdCollector,
    context: &mut Context,
    source_text: &str,
  ) {
    if import.import_kind == ImportOrExportKind::Type
      || import.specifiers.as_ref().map_or(true, |s| s.is_empty())
    {
      return;
    }
    let specifiers = import.specifiers.as_ref().unwrap();
    let mut type_only_usage: Vec<Span> = Vec::with_capacity(specifiers.len());
    let mut type_only_named_import = Vec::with_capacity(specifiers.len());
    for specifier in specifiers {
      match specifier {
        ImportDeclarationSpecifier::ImportSpecifier(named) => {
          if named.import_kind == ImportOrExportKind::Type {
            type_only_named_import.push(named.span);
          } else if !ids.has_import_ident(named.local.name.as_str()) {
            type_only_usage.push(specifier.span());
          }
        }
        ImportDeclarationSpecifier::ImportDefaultSpecifier(default) => {
          if !ids.has_import_ident(default.local.name.as_str()) {
            type_only_usage.push(specifier.span());
          }
        }
        ImportDeclarationSpecifier::ImportNamespaceSpecifier(namespace) => {
          if !ids.has_import_ident(namespace.local.name.as_str()) {
            type_only_usage.push(specifier.span());
          }
        }
      }
    }
    if specifiers.len() == type_only_usage.len() + type_only_named_import.len()
    {
      // Find the "import" keyword span - it starts at import.span.start
      let import_keyword_end = import.span.start + 6; // "import" is 6 chars
      let import_keyword_span =
        Span::new(import.span.start, import_keyword_end);
      let mut changes = Vec::with_capacity(1 + type_only_named_import.len());
      changes.push(LintFixChange {
        new_text: " type".into(),
        range: Span::new(import_keyword_end, import_keyword_end),
      });
      for named_span in type_only_named_import {
        // The specifier span starts at `type` itself (e.g. `type Type`).
        // Remove the leading `type ` (5 chars) from the specifier.
        let spec_text =
          &source_text[named_span.start as usize..named_span.end as usize];
        if spec_text.starts_with("type ") {
          changes.push(LintFixChange {
            new_text: "".into(),
            range: Span::new(named_span.start, named_span.start + 5),
          });
        }
      }
      context.add_diagnostic_with_fixes(
        import_keyword_span,
        CODE,
        Message::AllImportIdentsUsedInTypes,
        Some(Hint::ChangeImportToImportType.to_string()),
        vec![LintFix {
          description: FIX_ADD_TYPE_KEYWORD_DESC.into(),
          changes,
        }],
      );
    } else {
      for specifier_span in type_only_usage {
        context.add_diagnostic_with_fixes(
          specifier_span,
          CODE,
          Message::ImportIdentUsedInTypes,
          Some(Hint::AddTypeKeyword.to_string()),
          vec![LintFix {
            description: FIX_ADD_TYPE_KEYWORD_DESC.into(),
            changes: vec![LintFixChange {
              new_text: "type ".into(),
              range: Span::new(specifier_span.start, specifier_span.start),
            }],
          }],
        );
      }
    }
  }

  fn analyze_export(
    &self,
    named_export: &ExportNamedDeclaration,
    ids: &IdCollector,
    context: &mut Context,
    source_text: &str,
  ) {
    if named_export.export_kind == ImportOrExportKind::Type {
      return;
    }

    if named_export.specifiers.is_empty() {
      if let Some(src) = &named_export.source {
        let src_text =
          &source_text[src.span.start as usize..src.span.end as usize];
        let quote_kind = if src_text.starts_with('\'') {
          '\''
        } else {
          '\"'
        };
        let full_text = &source_text
          [named_export.span.start as usize..named_export.span.end as usize];
        let semicolon = if full_text.ends_with(';') { ";" } else { "" };
        let changes = Vec::from([LintFixChange {
          new_text: format!(
            "import {0}{1}{0}{2}",
            quote_kind, src.value, semicolon
          )
          .into(),
          range: named_export.span,
        }]);
        context.add_diagnostic_with_fixes(
          named_export.span,
          CODE,
          Message::ExportDeclarationElided,
          Some(Hint::ChangeSideEffectImport.to_string()),
          vec![LintFix {
            description: FIX_USE_IMPORT_DECL_DESC.into(),
            changes,
          }],
        );

        return;
      }
    }

    if named_export.specifiers.is_empty() || named_export.source.is_some() {
      return;
    }

    let mut type_only_usage: Vec<Span> =
      Vec::with_capacity(named_export.specifiers.len());
    let mut type_only_named_export =
      Vec::with_capacity(named_export.specifiers.len());
    for specifier in &named_export.specifiers {
      if specifier.export_kind == ImportOrExportKind::Type {
        type_only_named_export.push(specifier.span);
      } else {
        let name = match &specifier.local {
          ModuleExportName::IdentifierReference(ident) => ident.name.as_str(),
          ModuleExportName::IdentifierName(ident) => ident.name.as_str(),
          ModuleExportName::StringLiteral(_) => continue,
        };
        if !ids.has_export_ident(name) {
          type_only_usage.push(specifier.span);
        }
      }
    }
    if named_export.specifiers.len()
      == type_only_usage.len() + type_only_named_export.len()
    {
      let export_keyword_end = named_export.span.start + 6; // "export" is 6 chars
      let export_keyword_span =
        Span::new(named_export.span.start, export_keyword_end);
      let mut changes = Vec::with_capacity(1 + type_only_named_export.len());
      changes.push(LintFixChange {
        new_text: " type".into(),
        range: Span::new(export_keyword_end, export_keyword_end),
      });
      for named_span in type_only_named_export {
        // The specifier span starts at `type` itself (e.g. `type value`).
        // Remove the leading `type ` (5 chars) from the specifier.
        let spec_text =
          &source_text[named_span.start as usize..named_span.end as usize];
        if spec_text.starts_with("type ") {
          changes.push(LintFixChange {
            new_text: "".into(),
            range: Span::new(named_span.start, named_span.start + 5),
          });
        }
      }
      context.add_diagnostic_with_fixes(
        export_keyword_span,
        CODE,
        Message::AllExportIdentsUsedInTypes,
        Some(Hint::ChangeExportToExportType.to_string()),
        vec![LintFix {
          description: FIX_ADD_TYPE_KEYWORD_DESC.into(),
          changes,
        }],
      );
    } else {
      for specifier_span in type_only_usage {
        context.add_diagnostic_with_fixes(
          specifier_span,
          CODE,
          Message::ExportIdentUsedInTypes,
          Some(Hint::AddTypeKeyword.to_string()),
          vec![LintFix {
            description: FIX_ADD_TYPE_KEYWORD_DESC.into(),
            changes: vec![LintFixChange {
              new_text: "type ".into(),
              range: Span::new(specifier_span.start, specifier_span.start),
            }],
          }],
        );
      }
    }
  }
}

impl LintRule for VerbatimModuleSyntax {
  fn tags(&self) -> Tags {
    &[tags::JSR]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    if program.source_type.is_script() {
      return;
    }
    let ids = IdCollector::build(program);
    let source_text = context.source_text();

    for stmt in &program.body {
      match stmt {
        Statement::ImportDeclaration(import) => {
          self.analyze_import(import, &ids, context, source_text);
        }
        Statement::ExportNamedDeclaration(named_export) => {
          self.analyze_export(named_export, &ids, context, source_text);
        }
        _ => {}
      }
    }
  }
}

/// Collects identifiers used in value positions, import positions, and export positions.
#[derive(Debug, Default)]
struct IdCollector {
  id_usage: HashSet<String>,
  export_value_id_usage: HashSet<String>,
  import_value_id_usage: HashSet<String>,
  /// Names that are declared exclusively as types (e.g. `type Foo = ...`, `interface Foo {}`).
  type_only_decls: HashSet<String>,
}

impl IdCollector {
  pub fn build(program: &Program) -> Self {
    let mut ids = Self::default();
    ids.visit_program(program);
    ids
  }

  pub fn has_import_ident(&self, name: &str) -> bool {
    self.id_usage.contains(name) || self.export_value_id_usage.contains(name)
  }

  pub fn has_export_ident(&self, name: &str) -> bool {
    // If the identifier is only declared as a type, it is not a value usage.
    if self.type_only_decls.contains(name)
      && !self.id_usage.contains(name)
      && !self.import_value_id_usage.contains(name)
    {
      return false;
    }
    self.id_usage.contains(name) || self.import_value_id_usage.contains(name)
  }
}

impl<'a> Visit<'a> for IdCollector {
  fn visit_identifier_reference(&mut self, n: &IdentifierReference<'a>) {
    self.id_usage.insert(n.name.to_string());
  }

  fn visit_binding_identifier(&mut self, id: &BindingIdentifier<'a>) {
    // mark declarations as usages for export declarations
    self.id_usage.insert(id.name.to_string());
  }

  fn visit_import_declaration(&mut self, n: &ImportDeclaration<'a>) {
    if n.import_kind == ImportOrExportKind::Type {
      return;
    }
    walk::walk_import_declaration(self, n);
  }

  fn visit_import_specifier(&mut self, n: &ImportSpecifier<'a>) {
    if n.import_kind != ImportOrExportKind::Type {
      self.import_value_id_usage.insert(n.local.name.to_string());
    }
  }

  fn visit_import_default_specifier(&mut self, n: &ImportDefaultSpecifier<'a>) {
    self.import_value_id_usage.insert(n.local.name.to_string());
  }

  fn visit_import_namespace_specifier(
    &mut self,
    n: &ImportNamespaceSpecifier<'a>,
  ) {
    self.import_value_id_usage.insert(n.local.name.to_string());
  }

  fn visit_ts_import_equals_declaration(
    &mut self,
    n: &TSImportEqualsDeclaration<'a>,
  ) {
    if n.import_kind == ImportOrExportKind::Type {
      return;
    }

    // skip id visit
    match &n.module_reference {
      TSModuleReference::IdentifierReference(ident) => {
        self.visit_identifier_reference(ident);
      }
      TSModuleReference::QualifiedName(name) => {
        self.visit_ts_qualified_name(name);
      }
      TSModuleReference::ExternalModuleReference(_) => {}
    }
  }

  fn visit_export_specifier(&mut self, n: &ExportSpecifier<'a>) {
    if n.export_kind == ImportOrExportKind::Type {
      return;
    }

    match &n.local {
      ModuleExportName::IdentifierReference(ident) => {
        self.export_value_id_usage.insert(ident.name.to_string());
      }
      ModuleExportName::IdentifierName(ident) => {
        self.export_value_id_usage.insert(ident.name.to_string());
      }
      ModuleExportName::StringLiteral(_) => {}
    }
  }

  fn visit_export_named_declaration(&mut self, n: &ExportNamedDeclaration<'a>) {
    if n.export_kind == ImportOrExportKind::Type || n.source.is_some() {
      return;
    }

    walk::walk_export_named_declaration(self, n);
  }

  fn visit_ts_type(&mut self, _n: &TSType<'a>) {
    // Skip visiting type positions — identifiers referenced in type-only
    // contexts (e.g. `type Test = Type`) are not value usages.
  }

  fn visit_ts_type_annotation(&mut self, _n: &TSTypeAnnotation<'a>) {
    // Skip visiting type annotations — identifiers in type position
    // are not value usages.
  }

  fn visit_ts_type_alias_declaration(
    &mut self,
    n: &TSTypeAliasDeclaration<'a>,
  ) {
    // Record the name as a type-only declaration; do NOT add it to id_usage.
    self.type_only_decls.insert(n.id.name.to_string());
  }

  fn visit_ts_interface_declaration(&mut self, n: &TSInterfaceDeclaration<'a>) {
    // Record the name as a type-only declaration; do NOT add it to id_usage.
    self.type_only_decls.insert(n.id.name.to_string());
  }

  fn visit_jsx_element_name(&mut self, n: &JSXElementName<'a>) {
    if let JSXElementName::IdentifierReference(ident) = n {
      if ident
        .name
        .as_str()
        .starts_with(|c: char| c.is_ascii_lowercase())
      {
        return;
      }
    }

    walk::walk_jsx_element_name(self, n);
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
      "export {};",
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
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type { Type } from 'module'; type Test = Type;"),
        }
      ],
      "import { Type, Other } from 'module'; type Test = Type | Other;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type { Type, Other } from 'module'; type Test = Type | Other;"),
        }
      ],
      "import { type Type, Other } from 'module'; type Test = Type | Other;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type { Type, Other } from 'module'; type Test = Type | Other;"),
        }
      ],
      "import { Type, value } from 'module'; type Test = Type; value();": [
        {
          col: 9,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { type Type, value } from 'module'; type Test = Type; value();"),
        }
      ],
      "import { Type, Other, value } from 'module'; type Test = Type | Other; value();": [
        {
          col: 9,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { type Type, Other, value } from 'module'; type Test = Type | Other; value();"),
        },
        {
          col: 15,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { Type, type Other, value } from 'module'; type Test = Type | Other; value();"),
        }
      ],
      "import * as value from 'module'; type Test = typeof value;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type * as value from 'module'; type Test = typeof value;"),
        }
      ],
      "import value from 'module'; type Test = typeof value;": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type value from 'module'; type Test = typeof value;"),
        }
      ],
      "type Test = string; export { Test };": [
        {
          col: 20,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "type Test = string; export type { Test };"),
        }
      ],
      "type Test = string; type Test2 = string; export { Test, Test2 };": [
        {
          col: 41,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "type Test = string; type Test2 = string; export type { Test, Test2 };"),
        }
      ],
      "import { type value } from 'module'; export { value };": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type { value } from 'module'; export { value };"),
        },
        {
          col: 37,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { type value } from 'module'; export type { value };"),
        }
      ],
      "import { value } from 'module'; export { type value };": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type { value } from 'module'; export { type value };"),
        },
        {
          col: 32,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { value } from 'module'; export type { value };"),
        }
      ],
      "import type { value } from 'module'; export { value };": [
        {
          col: 37,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type { value } from 'module'; export type { value };"),
        }
      ],
      "import { value } from 'module'; export type { value };": [
        {
          col: 0,
          message: Message::AllImportIdentsUsedInTypes,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import type { value } from 'module'; export type { value };"),
        }
      ],
      "import { value, type Type } from 'module'; console.log(value); export { Type };": [
        {
          col: 63,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { value, type Type } from 'module'; console.log(value); export type { Type };"),
        }
      ],
      "import { value, Type } from 'module'; console.log(value); export { type Type };": [
        {
          col: 16,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { value, type Type } from 'module'; console.log(value); export { type Type };"),
        },
        {
          col: 58,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { value, Type } from 'module'; console.log(value); export type { Type };"),
        }
      ],
      "import { value, Type } from 'module'; console.log(value); export type { Type };": [
        {
          col: 16,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { value, type Type } from 'module'; console.log(value); export type { Type };"),
        }
      ],
      "import { value, type Type } from 'module'; export { value, Type };": [
        {
          col: 59,
          message: Message::ExportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { value, type Type } from 'module'; export { value, type Type };"),
        }
      ],
      "import { value, Type } from 'module'; export { value, type Type };": [
        {
          col: 16,
          message: Message::ImportIdentUsedInTypes,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "import { value, type Type } from 'module'; export { value, type Type };"),
        }
      ],
      "export { } from 'module';": [
        {
          col: 0,
          message: Message::ExportDeclarationElided,
          hint: Hint::ChangeSideEffectImport,
          fix: (FIX_USE_IMPORT_DECL_DESC, "import 'module';"),
        }
      ],
      "export { } from \"module\";": [
        {
          col: 0,
          message: Message::ExportDeclarationElided,
          hint: Hint::ChangeSideEffectImport,
          fix: (FIX_USE_IMPORT_DECL_DESC, "import \"module\";"),
        }
      ],
      "export { } from \"module\"": [
        {
          col: 0,
          message: Message::ExportDeclarationElided,
          hint: Hint::ChangeSideEffectImport,
          fix: (FIX_USE_IMPORT_DECL_DESC, "import \"module\""),
        }
      ],
      "interface Test {}\nexport { Test };": [
        {
          line: 2,
          col: 0,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "interface Test {}\nexport type { Test };"),
        }
      ],
      "type Test = 'test';\nexport { Test };": [
        {
          line: 2,
          col: 0,
          message: Message::AllExportIdentsUsedInTypes,
          hint: Hint::ChangeExportToExportType,
          fix: (FIX_ADD_TYPE_KEYWORD_DESC, "type Test = 'test';\nexport type { Test };"),
        }
      ],
    };
  }
}
