// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::swc_util::StringRepr;
use crate::tags::Tags;

use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct Camelcase;

const CODE: &str = "camelcase";

impl LintRule for Camelcase {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = CamelcaseHandler::default();
    crate::handler::traverse_program(&mut handler, program, context);
    handler.report_errors(context);
  }
}

/// Check if it contains underscores, except for leading and trailing ones
fn is_underscored(ident_name: &str) -> bool {
  let trimmed_ident = ident_name.trim_matches('_');
  trimmed_ident.contains('_')
    && trimmed_ident != trimmed_ident.to_ascii_uppercase()
}

/// Convert the name of identifier into camel case. If the name is originally in camel case, return
/// the name as it is. For more detail, see the test cases below.
fn to_camelcase(ident_name: &str) -> String {
  if !is_underscored(ident_name) {
    return ident_name.to_string();
  }

  static UNDERSCORE_CHAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([^_])_([a-z])").unwrap());

  let result = UNDERSCORE_CHAR_RE.replace_all(ident_name, |caps: &Captures| {
    format!("{}{}", &caps[1], caps[2].to_ascii_uppercase())
  });

  if result != ident_name {
    return result.into_owned();
  }

  ident_name.to_ascii_uppercase()
}

enum IdentToCheck {
  /// Normal variable name e.g. `foo` in `const foo = 42;`
  Variable(String),
  /// Object key name, for example:
  ///
  /// ```typescript
  /// const obj = { foo: 42 }; // key_name: foo, is_shorthand: false
  ///
  /// const obj2 = { someVariable }; // key_name: someVariable, is_shorthand: true
  /// ```
  ObjectKey {
    key_name: String,
    is_shorthand: bool,
  },
  /// Function name e.g. `foo` in `function foo() {}`
  Function(String),
  /// Class name e.g. `Foo` in `class Foo {}`
  Class(String),
  /// Type alias e.g. `Foo` in `type Foo = string;`
  TypeAlias(String),
  /// Interface name e.g. `Foo` in `interface Foo {}`
  Interface(String),
  /// Enum name e.g. `Foo` in `enum Foo {}`
  EnumName(String),
  /// Enum variant e.g. `Bar` in `enum Foo { Bar }`
  EnumVariant(String),
  /// Namespace e.g. `Foo` in `namespace Foo {}`
  Namespace(String),
  /// Module e.g. `Foo` in `module Foo {}`
  Module(String),
  /// Key and value name in object pattern, for example:
  ///
  /// ```typescript
  /// // key_name: foo, value_name: None, has_default: false
  /// const { foo } = obj1;
  ///
  /// // key_name: foo, value_name: Some(bar), has_default: false
  /// const { foo: bar } = obj2;
  ///
  /// // key_name: foo, value_name: Some(bar), has_default: true
  /// const { foo: bar = baz } = obj3; // key_name: foo, value_name: Some(bar),
  ///
  /// // key_name: foo, value_name: None, has_default: false
  /// function f({ foo }) {}
  /// ```
  ObjectPat {
    key_name: String,
    value_name: Option<String>,
    has_default: bool,
    is_destructuring: bool,
  },
  /// Local name and imported name in named import, for example:
  ///
  /// ```typescript
  /// import { foo } from 'mod.ts'; // local: foo, imported: None
  ///
  /// import { foo as bar } from 'mod.ts'; // local: bar, imported: Some(foo)
  /// ```
  NamedImport {
    local: String,
    imported: Option<String>,
  },
}

impl IdentToCheck {
  fn variable(name: impl AsRef<str>) -> Self {
    Self::Variable(name.as_ref().to_string())
  }

  fn object_key(key_name: impl AsRef<str>, is_shorthand: bool) -> Self {
    Self::ObjectKey {
      key_name: key_name.as_ref().to_string(),
      is_shorthand,
    }
  }

  fn function(name: impl AsRef<str>) -> Self {
    Self::Function(name.as_ref().to_string())
  }

  fn class(name: impl AsRef<str>) -> Self {
    Self::Class(name.as_ref().to_string())
  }

  fn type_alias(name: impl AsRef<str>) -> Self {
    Self::TypeAlias(name.as_ref().to_string())
  }

  fn interface(name: impl AsRef<str>) -> Self {
    Self::Interface(name.as_ref().to_string())
  }

  fn enum_name(name: impl AsRef<str>) -> Self {
    Self::EnumName(name.as_ref().to_string())
  }

  fn enum_variant(name: impl AsRef<str>) -> Self {
    Self::EnumVariant(name.as_ref().to_string())
  }

  fn namespace(name: impl AsRef<str>) -> Self {
    Self::Namespace(name.as_ref().to_string())
  }

  fn module(name: impl AsRef<str>) -> Self {
    Self::Module(name.as_ref().to_string())
  }

  fn object_pat<K, V>(
    key_name: &K,
    value_name: Option<&V>,
    has_default: bool,
    is_destructuring: bool,
  ) -> Self
  where
    K: AsRef<str> + ?Sized,
    V: AsRef<str> + ?Sized,
  {
    Self::ObjectPat {
      key_name: key_name.as_ref().to_string(),
      value_name: value_name.map(|v| v.as_ref().to_string()),
      has_default,
      is_destructuring,
    }
  }

  fn named_import<L, I>(local: &L, imported: Option<&I>) -> Self
  where
    L: AsRef<str>,
    I: AsRef<str>,
  {
    Self::NamedImport {
      local: local.as_ref().to_string(),
      imported: imported.map(|i| i.as_ref().to_string()),
    }
  }

  fn get_ident_name(&self) -> &str {
    match self {
      IdentToCheck::Variable(name)
      | IdentToCheck::Function(name)
      | IdentToCheck::Class(name)
      | IdentToCheck::TypeAlias(name)
      | IdentToCheck::Interface(name)
      | IdentToCheck::EnumName(name)
      | IdentToCheck::EnumVariant(name)
      | IdentToCheck::Namespace(name)
      | IdentToCheck::Module(name) => name,
      IdentToCheck::ObjectKey { ref key_name, .. } => key_name,
      IdentToCheck::ObjectPat {
        key_name,
        value_name,
        ..
      } => {
        if let Some(value_name) = value_name {
          value_name
        } else {
          key_name
        }
      }
      IdentToCheck::NamedImport { local, .. } => local,
    }
  }

  fn to_message(&self) -> String {
    format!(
      "Identifier '{}' is not in camel case.",
      self.get_ident_name()
    )
  }

  fn to_hint(&self) -> String {
    match self {
      IdentToCheck::Variable(name) | IdentToCheck::Function(name) => {
        format!("Consider renaming `{}` to `{}`", name, to_camelcase(name))
      }
      IdentToCheck::ObjectKey {
        ref key_name,
        is_shorthand,
      } => {
        if *is_shorthand {
          format!(
            r#"Consider writing `{camel_cased}: {original}` or `"{original}": {original}`"#,
            camel_cased = to_camelcase(key_name),
            original = key_name
          )
        } else {
          format!(
            r#"Consider renaming `{original}` to `{camel_cased}`, or wrapping it in quotation mark like `"{original}"`"#,
            camel_cased = to_camelcase(key_name),
            original = key_name
          )
        }
      }
      IdentToCheck::Class(name)
      | IdentToCheck::TypeAlias(name)
      | IdentToCheck::Interface(name)
      | IdentToCheck::EnumName(name)
      | IdentToCheck::EnumVariant(name)
      | IdentToCheck::Namespace(name)
      | IdentToCheck::Module(name) => {
        let camel_cased = to_camelcase(name);
        static FIRST_CHAR_LOWERCASE: Lazy<Regex> =
          Lazy::new(|| Regex::new(r"^[a-z]").unwrap());

        // The following names should be in pascal case
        // - class
        // - type alias
        // - interface
        // - enum
        // - enum variant
        // - namespace
        // - module
        let pascal_cased = FIRST_CHAR_LOWERCASE
          .replace(&camel_cased, |caps: &Captures| {
            caps[0].to_ascii_uppercase()
          });
        format!("Consider renaming `{}` to `{}`", name, pascal_cased)
      }
      IdentToCheck::ObjectPat {
        key_name,
        value_name,
        has_default,
        is_destructuring: in_var_decl,
      } => {
        let rename_name = if let Some(value_name) = value_name {
          Some(value_name)
        } else if *in_var_decl {
          None
        } else {
          Some(key_name)
        };
        if let Some(name) = rename_name {
          return format!(
            "Consider renaming `{}` to `{}`",
            name,
            to_camelcase(name),
          );
        }

        if *has_default {
          return format!(
            "Consider replacing `{{ {key} = .. }}` with `{{ {key}: {value} = .. }}`",
            key = key_name,
            value = to_camelcase(key_name),
          );
        }

        format!(
          "Consider replacing `{{ {key} }}` with `{{ {key}: {value} }}`",
          key = key_name,
          value = to_camelcase(key_name),
        )
      }
      IdentToCheck::NamedImport { local, imported } => {
        if imported.is_some() {
          format!("Consider renaming `{}` to `{}`", local, to_camelcase(local))
        } else {
          format!(
            "Consider replacing `{{ {local} }}` with `{{ {local} as {camel_cased_local} }}`",
            local = local,
            camel_cased_local = to_camelcase(local),
          )
        }
      }
    }
  }
}

#[derive(Default)]
struct CamelcaseHandler {
  /// Accumulated errors to report
  errors: BTreeMap<Span, IdentToCheck>,
  /// Already visited identifiers
  visited: BTreeSet<Span>,
}

impl CamelcaseHandler {
  /// Report accumulated errors, consuming `self`.
  fn report_errors(self, ctx: &mut Context) {
    for (span, error_ident) in self.errors {
      ctx.add_diagnostic_with_hint(
        span,
        CODE,
        error_ident.to_message(),
        error_ident.to_hint(),
      );
    }
  }

  /// Check if this ident is underscored only when it's not yet visited.
  fn check_ident(&mut self, span: Span, ident: IdentToCheck) {
    if self.visited.insert(span) && is_underscored(ident.get_ident_name()) {
      self.errors.insert(span, ident);
    }
  }

  fn check_ts_type(&mut self, ty: &TSType) {
    if let TSType::TSTypeLiteral(type_lit) = ty {
      for member in &type_lit.members {
        self.check_ts_type_element(member);
      }
    }
  }

  fn check_ts_type_element(&mut self, ty_el: &TSSignature) {
    match ty_el {
      TSSignature::TSPropertySignature(prop_sig) => {
        if let PropertyKey::StaticIdentifier(ident) = &prop_sig.key {
          self.check_ident(
            ident.span,
            IdentToCheck::object_key(ident.name.as_str(), false),
          );
        }
        if let Some(type_ann) = &prop_sig.type_annotation {
          self.check_ts_type(&type_ann.type_annotation);
        }
      }
      TSSignature::TSMethodSignature(method_sig) => {
        if let PropertyKey::StaticIdentifier(ident) = &method_sig.key {
          self.check_ident(
            ident.span,
            IdentToCheck::function(ident.name.as_str()),
          );
        }
        if let Some(type_ann) = &method_sig.return_type {
          self.check_ts_type(&type_ann.type_annotation);
        }
      }
      _ => {}
    }
  }

  fn check_binding_pattern(
    &mut self,
    pat: &BindingPattern,
    in_var_declarator: bool,
  ) {
    match pat {
      BindingPattern::BindingIdentifier(ident) => {
        self
          .check_ident(ident.span, IdentToCheck::variable(ident.name.as_str()));
      }
      BindingPattern::ArrayPattern(array_pat) => {
        for elem in array_pat.elements.iter().flatten() {
          self.check_binding_pattern(elem, in_var_declarator);
        }
        if let Some(rest) = &array_pat.rest {
          self.check_binding_pattern(&rest.argument, in_var_declarator);
        }
      }
      BindingPattern::ObjectPattern(obj_pat) => {
        for prop in &obj_pat.properties {
          match &prop.value {
            BindingPattern::BindingIdentifier(value_ident) => {
              if prop.shorthand {
                // shorthand: { foo_bar }
                if !in_var_declarator {
                  self.check_ident(
                    value_ident.span,
                    IdentToCheck::object_pat::<str, str>(
                      value_ident.name.as_str(),
                      None,
                      false,
                      in_var_declarator,
                    ),
                  );
                }
              } else {
                // key-value: { foo: bar_baz }
                let key_name = prop
                  .key
                  .string_repr()
                  .unwrap_or_else(|| "[KEY]".to_string());
                self.check_ident(
                  value_ident.span,
                  IdentToCheck::object_pat(
                    &key_name,
                    Some(&value_ident.name.as_str()),
                    false,
                    in_var_declarator,
                  ),
                );
              }
            }
            BindingPattern::AssignmentPattern(assign_pat) => {
              if let BindingPattern::BindingIdentifier(value_ident) =
                &assign_pat.left
              {
                if prop.shorthand {
                  if !in_var_declarator {
                    self.check_ident(
                      value_ident.span,
                      IdentToCheck::object_pat::<str, str>(
                        value_ident.name.as_str(),
                        None,
                        true,
                        in_var_declarator,
                      ),
                    );
                  }
                } else {
                  let key_name = prop
                    .key
                    .string_repr()
                    .unwrap_or_else(|| "[KEY]".to_string());
                  self.check_ident(
                    value_ident.span,
                    IdentToCheck::object_pat(
                      &key_name,
                      Some(&value_ident.name.as_str()),
                      true,
                      in_var_declarator,
                    ),
                  );
                }
              } else {
                self.check_binding_pattern(&assign_pat.left, in_var_declarator);
              }
            }
            _ => {
              self.check_binding_pattern(&prop.value, in_var_declarator);
            }
          }
        }
        if let Some(rest) = &obj_pat.rest {
          self.check_binding_pattern(&rest.argument, in_var_declarator);
        }
      }
      BindingPattern::AssignmentPattern(assign_pat) => {
        self.check_binding_pattern(&assign_pat.left, in_var_declarator);
      }
    }
  }
}

impl Handler<'_> for CamelcaseHandler {
  fn function(&mut self, func: &Function, ctx: &mut Context) {
    if func.declare {
      ctx.stop_traverse();
      return;
    }

    if let Some(id) = &func.id {
      self.check_ident(id.span, IdentToCheck::function(id.name.as_str()));
    }
  }

  fn class(&mut self, class: &Class, ctx: &mut Context) {
    if class.declare {
      ctx.stop_traverse();
      return;
    }

    if let Some(id) = &class.id {
      self.check_ident(id.span, IdentToCheck::class(id.name.as_str()));
    }
  }

  fn variable_declaration(
    &mut self,
    var_decl: &VariableDeclaration,
    ctx: &mut Context,
  ) {
    if var_decl.declare {
      ctx.stop_traverse();
      return;
    }

    for decl in &var_decl.declarations {
      self.check_binding_pattern(&decl.id, true);

      if let Some(expr) = &decl.init {
        match expr {
          Expression::ObjectExpression(obj_lit) => {
            for prop in &obj_lit.properties {
              match prop {
                ObjectPropertyKind::ObjectProperty(prop) => {
                  if prop.shorthand {
                    if let Expression::Identifier(ident) = &prop.value {
                      self.check_ident(
                        ident.span,
                        IdentToCheck::object_key(ident.name.as_str(), true),
                      );
                    }
                  } else if let PropertyKey::StaticIdentifier(ident) = &prop.key
                  {
                    self.check_ident(
                      ident.span,
                      IdentToCheck::object_key(ident.name.as_str(), false),
                    );
                  }
                }
                ObjectPropertyKind::SpreadProperty(_) => {}
              }
            }
          }
          Expression::FunctionExpression(func_expr) => {
            if let Some(id) = &func_expr.id {
              self
                .check_ident(id.span, IdentToCheck::function(id.name.as_str()));
            }
          }
          Expression::ClassExpression(class_expr) => {
            if let Some(id) = &class_expr.id {
              self.check_ident(id.span, IdentToCheck::class(id.name.as_str()));
            }
          }
          _ => {}
        }
      }
    }
  }

  fn formal_parameter(&mut self, param: &FormalParameter, _ctx: &mut Context) {
    self.check_binding_pattern(&param.pattern, false);
  }

  fn import_specifier(
    &mut self,
    import_specifier: &ImportSpecifier,
    _ctx: &mut Context,
  ) {
    let local = &import_specifier.local;
    let imported = &import_specifier.imported;
    let imported_name = match imported {
      ModuleExportName::IdentifierName(ident) => Some(ident.name.to_string()),
      ModuleExportName::StringLiteral(str) => Some(str.value.to_string()),
      ModuleExportName::IdentifierReference(ident) => {
        Some(ident.name.to_string())
      }
    };
    // Only check if there's an explicit rename (imported != local)
    if imported_name.as_deref() != Some(local.name.as_str()) {
      self.check_ident(
        local.span,
        IdentToCheck::named_import(
          &local.name.as_str(),
          imported_name.as_ref(),
        ),
      );
    }
  }

  fn import_default_specifier(
    &mut self,
    import_default_specifier: &ImportDefaultSpecifier,
    _ctx: &mut Context,
  ) {
    let local = &import_default_specifier.local;
    self.check_ident(local.span, IdentToCheck::variable(local.name.as_str()));
  }

  fn import_namespace_specifier(
    &mut self,
    import_namespace_specifier: &ImportNamespaceSpecifier,
    _ctx: &mut Context,
  ) {
    let local = &import_namespace_specifier.local;
    self.check_ident(local.span, IdentToCheck::variable(local.name.as_str()));
  }

  fn export_specifier(
    &mut self,
    _export_specifier: &ExportSpecifier,
    _ctx: &mut Context,
  ) {
    // Only check namespace exports (export * as name from ...)
    // For regular named exports, we don't check here
  }

  fn export_all_declaration(
    &mut self,
    export_all: &ExportAllDeclaration,
    _ctx: &mut Context,
  ) {
    if let Some(exported) = &export_all.exported {
      match exported {
        ModuleExportName::IdentifierName(name) => {
          self
            .check_ident(name.span, IdentToCheck::variable(name.name.as_str()));
        }
        _ => {}
      }
    }
  }

  fn ts_type_alias_declaration(
    &mut self,
    type_alias: &TSTypeAliasDeclaration,
    ctx: &mut Context,
  ) {
    if type_alias.declare {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      type_alias.id.span,
      IdentToCheck::type_alias(type_alias.id.name.as_str()),
    );
    self.check_ts_type(&type_alias.type_annotation);
  }

  fn ts_interface_declaration(
    &mut self,
    interface_decl: &TSInterfaceDeclaration,
    ctx: &mut Context,
  ) {
    if interface_decl.declare {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      interface_decl.id.span,
      IdentToCheck::interface(interface_decl.id.name.as_str()),
    );

    for ty_el in &interface_decl.body.body {
      self.check_ts_type_element(ty_el);
    }
  }

  fn ts_module_declaration(
    &mut self,
    module_decl: &TSModuleDeclaration,
    ctx: &mut Context,
  ) {
    if module_decl.declare {
      ctx.stop_traverse();
      return;
    }

    match &module_decl.id {
      TSModuleDeclarationName::Identifier(id) => {
        if module_decl.kind == TSModuleDeclarationKind::Namespace {
          self.check_ident(id.span, IdentToCheck::namespace(id.name.as_str()));
        } else {
          self.check_ident(id.span, IdentToCheck::module(id.name.as_str()));
        }
      }
      TSModuleDeclarationName::StringLiteral(_) => {}
    }
  }

  fn ts_enum_declaration(
    &mut self,
    enum_decl: &TSEnumDeclaration,
    ctx: &mut Context,
  ) {
    if enum_decl.declare {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      enum_decl.id.span,
      IdentToCheck::enum_name(enum_decl.id.name.as_str()),
    );
    for variant in &enum_decl.body.members {
      match &variant.id {
        TSEnumMemberName::Identifier(id) => {
          self
            .check_ident(id.span, IdentToCheck::enum_variant(id.name.as_str()));
        }
        _ => {}
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_underscored() {
    let tests = [
      ("foo_bar", true),
      ("fooBar", false),
      ("FooBar", false),
      ("foo_bar_baz", true),
      ("_foo_bar_baz", true),
      ("__foo_bar_baz__", true),
      ("__fooBar_baz__", true),
      ("__fooBarBaz__", false),
      ("Sha3_224", true),
      ("SHA3_224", false),
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(expected, is_underscored(input));
    }
  }

  #[test]
  fn test_to_camelcase() {
    let tests = [
      ("foo_bar", "fooBar"),
      ("fooBar", "fooBar"),
      ("FooBar", "FooBar"),
      ("foo_bar_baz", "fooBarBaz"),
      ("_foo_bar_baz", "_fooBarBaz"),
      ("__foo_bar_baz__", "__fooBarBaz__"),
      ("Sha3_224", "SHA3_224"),
      ("SHA3_224", "SHA3_224"),
      ("_leading", "_leading"),
      ("trailing_", "trailing_"),
      ("_bothEnds_", "_bothEnds_"),
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(expected, to_camelcase(input));
    }
  }

  #[test]
  fn test_to_hint() {
    fn s(s: &str) -> String {
      s.to_string()
    }

    let tests = [
      (
        IdentToCheck::Variable(s("foo_bar")),
        "Consider renaming `foo_bar` to `fooBar`",
      ),
      (
        IdentToCheck::Function(s("foo_bar")),
        "Consider renaming `foo_bar` to `fooBar`",
      ),
      (
        IdentToCheck::Class(s("foo_bar")),
        "Consider renaming `foo_bar` to `FooBar`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: None,
          has_default: false,
          is_destructuring: true,
        },
        "Consider replacing `{ foo_bar }` with `{ foo_bar: fooBar }`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: Some(s("snake_case")),
          has_default: false,
          is_destructuring: true,
        },
        "Consider renaming `snake_case` to `snakeCase`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: None,
          has_default: true,
          is_destructuring: true,
        },
        "Consider replacing `{ foo_bar = .. }` with `{ foo_bar: fooBar = .. }`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: None,
          has_default: true,
          is_destructuring: false,
        },
        // not destructuring, so suggest a rename
        "Consider renaming `foo_bar` to `fooBar`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: Some(s("snake_case")),
          has_default: true,
          is_destructuring: true,
        },
        "Consider renaming `snake_case` to `snakeCase`",
      ),
      (
        IdentToCheck::NamedImport {
          local: s("foo_bar"),
          imported: None,
        },
        "Consider replacing `{ foo_bar }` with `{ foo_bar as fooBar }`",
      ),
      (
        IdentToCheck::NamedImport {
          local: s("foo_bar"),
          imported: Some(s("snake_case")),
        },
        "Consider renaming `foo_bar` to `fooBar`",
      ),
    ];

    for (error_ident, expected) in tests.iter() {
      assert_eq!(*expected, error_ident.to_hint());
    }
  }

  // Based on https://github.com/eslint/eslint/blob/v7.8.1/tests/lib/rules/camelcase.js

  #[test]
  fn camelcase_valid() {
    assert_lint_ok! {
      Camelcase,
      r#"firstName = "Ichigo""#,
      r#"FIRST_NAME = "Ichigo""#,
      r#"__myPrivateVariable = "Hoshimiya""#,
      r#"myPrivateVariable_ = "Hoshimiya""#,
      r#"function doSomething(){}"#,
      r#"do_something()"#,
      r#"new do_something"#,
      r#"new do_something()"#,
      r#"foo.do_something()"#,
      r#"var foo = bar.baz_boom;"#,
      r#"var foo = bar.baz_boom.something;"#,
      r#"foo.boom_pow.qux = bar.baz_boom.something;"#,
      r#"if (bar.baz_boom) {}"#,
      r#"var obj = { key: foo.bar_baz };"#,
      r#"var arr = [foo.bar_baz];"#,
      r#"[foo.bar_baz]"#,
      r#"var arr = [foo.bar_baz.qux];"#,
      r#"[foo.bar_baz.nesting]"#,
      r#"if (foo.bar_baz === boom.bam_pow) { [foo.baz_boom] }"#,
      r#"var o = {key: 1}"#,
      r#"var o = {_leading: 1}"#,
      r#"var o = {trailing_: 1}"#,
      r#"const { ['foo']: _foo } = obj;"#,
      r#"const { [_foo_]: foo } = obj;"#,
      r#"var { category_id: category } = query;"#,
      r#"var { _leading } = query;"#,
      r#"var { trailing_ } = query;"#,
      r#"var { or_middle } = query;"#,
      r#"var { category_id = 1 } = query;"#,
      r#"var { category_id: { property_test } } = query;"#,
      r#"const { no_camelcased = false } = bar;"#,
      r#"import { camelCased } from "external module";"#,
      r#"import { _leading } from "external module";"#,
      r#"import { trailing_ } from "external module";"#,
      r#"import { or_middle } from "external module";"#,
      r#"import { no_camelcased as camelCased } from "external-module";"#,
      r#"import { no_camelcased as _leading } from "external-module";"#,
      r#"import { no_camelcased as trailing_ } from "external-module";"#,
      r#"import { no_camelcased as camelCased, anotherCamelCased } from "external-module";"#,
      r#"import { camelCased } from 'mod'"#,
      r#"var _camelCased = aGlobalVariable"#,
      r#"var camelCased = _aGlobalVariable"#,
      r#"function foo({ no_camelcased: camelCased }) {};"#,
      r#"function foo({ no_camelcased: _leading }) {};"#,
      r#"function foo({ no_camelcased: trailing_ }) {};"#,
      r#"function foo({ camelCased = 'default value' }) {};"#,
      r#"function foo({ _leading = 'default value' }) {};"#,
      r#"function foo({ trailing_ = 'default value' }) {};"#,
      r#"function foo({ camelCased }) {};"#,
      r#"function foo({ _leading }) {}"#,
      r#"function foo({ trailing_ }) {}"#,
      r#"({obj} = baz.fo_o);"#,
      r#"([obj] = baz.fo_o);"#,
      r#"([obj.foo = obj.fo_o] = bar);"#,
      r#"const f = function camelCased() {};"#,
      r#"const c = class camelCased {};"#,
      r#"class camelCased {};"#,

      // The following test cases are _invalid_ in ESLint, but we've decided to treat them as _valid_.
      // See background at https://github.com/denoland/deno_lint/pull/302
      r#"first_name = "Akari""#,
      r#"__private_first_name = "Akari""#,
      r#"obj.foo_bar = function(){};"#,
      r#"bar_baz.foo = function(){};"#,
      r#"[foo_bar.baz]"#,
      r#"if (foo.bar_baz === boom.bam_pow) { [foo_bar.baz] }"#,
      r#"foo.bar_baz = boom.bam_pow"#,
      r#"foo.qux.boom_pow = { bar: boom.bam_pow }"#,
      r#"obj.a_b = 2;"#,
      r#"var { [category_id]: categoryId } = query;"#,
      r#"a_global_variable.foo()"#,
      r#"a_global_variable[undefined]"#,
      r#"var camelCased = snake_cased"#,
      r#"({ a: obj.fo_o } = bar);"#,
      r#"({ a: obj.fo_o.b_ar } = baz);"#,
      r#"({ a: { b: { c: obj.fo_o } } } = bar);"#,
      r#"({ a: { b: { c: obj.fo_o.b_ar } } } = baz);"#,
      r#"([obj.fo_o] = bar);"#,
      r#"([obj.fo_o = 1] = bar);"#,
      r#"({ a: [obj.fo_o] } = bar);"#,
      r#"({ a: { b: [obj.fo_o] } } = bar);"#,
      r#"([obj.fo_o.ba_r] = baz);"#,
      r#"obj.o_k.non_camelcase = 0"#,
      r#"(obj?.o_k).non_camelcase = 0"#,
      r#"({...obj.fo_o} = baz);"#,
      r#"({...obj.fo_o.ba_r} = baz);"#,
      r#"({c: {...obj.fo_o }} = baz);"#,
      r#"not_ignored_foo = 0;"#,

      // https://github.com/denoland/deno_lint/issues/475
      // We are forced to use snake_case keys in object literals in some cases such as an object
      // representing database schema. In such cases, one is allowed to use snake_case by wrapping
      // keys in quotation marks.
      r#"const obj = { "created_at": "2020-10-30T13:16:45+09:00" }"#,
      r#"const obj = { "created_at": created_at }"#,

      // https://github.com/denoland/deno_lint/issues/587
      // The rule shouldn't be applied to ambient declarations since the user who writes them is
      // unable to fix their names.
      r#"declare function foo_bar(a_b: number): void;"#,
      r#"declare const foo_bar: number;"#,
      r#"declare const foo_bar: number, snake_case: string;"#,
      r#"declare let foo_bar: { some_property: string; };"#,
      r#"declare var foo_bar: number;"#,
      r#"declare class foo_bar { some_method(some_param: boolean): string; };"#,
      r#"export declare const foo_bar: number;"#,
      r#"declare type foo_bar = { some_var: string; };"#,
      r#"declare interface foo_bar { some_var: string; }"#,
      r#"declare namespace foo_bar {}"#,
      r#"declare enum foo_bar { variant_one, variant_two }"#,
    };
  }

  #[test]
  fn camelcase_invalid() {
    assert_lint_err! {
      Camelcase,
      r#"function foo_bar(){}"#: [
            {
              col: 9,
              message: "Identifier 'foo_bar' is not in camel case.",
              hint: "Consider renaming `foo_bar` to `fooBar`",
            }
          ],
    r#"var foo = { bar_baz: boom.bam_pow }"#: [
            {
              col: 12,
              message: "Identifier 'bar_baz' is not in camel case.",
              hint: r#"Consider renaming `bar_baz` to `barBaz`, or wrapping it in quotation mark like `"bar_baz"`"#,
            }
          ],
    r#"var o = { bar_baz: 1 }"#: [
            {
              col: 10,
              message: "Identifier 'bar_baz' is not in camel case.",
              hint: r#"Consider renaming `bar_baz` to `barBaz`, or wrapping it in quotation mark like `"bar_baz"`"#,
            }
          ],
    r#"var o = { bar_baz }"#: [
            {
              col: 10,
              message: "Identifier 'bar_baz' is not in camel case.",
              hint: r#"Consider writing `barBaz: bar_baz` or `"bar_baz": bar_baz`"#,
            }
          ],
    r#"var { category_id: category_alias } = query;"#: [
            {
              col: 19,
              message: "Identifier 'category_alias' is not in camel case.",
              hint: "Consider renaming `category_alias` to `categoryAlias`",
            }
          ],
    r#"var { category_id: category_id } = query;"#: [
            {
              col: 19,
              message: "Identifier 'category_id' is not in camel case.",
              hint: "Consider renaming `category_id` to `categoryId`",
            }
          ],
    r#"import * as no_camelcased from "external-module";"#: [
            {
              col: 12,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"import { no_camelcased as no_camel_cased } from "external module";"#: [
            {
              col: 26,
              message: "Identifier 'no_camel_cased' is not in camel case.",
              hint: "Consider renaming `no_camel_cased` to `noCamelCased`",
            }
          ],
    r#"import { camelCased as no_camel_cased } from "external module";"#: [
            {
              col: 23,
              message: "Identifier 'no_camel_cased' is not in camel case.",
              hint: "Consider renaming `no_camel_cased` to `noCamelCased`",
            }
          ],
    r#"import no_camelcased, { another_no_camelcased as camelCased } from "external-module";"#: [
            {
              col: 7,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"import snake_cased from 'mod'"#: [
            {
              col: 7,
              message: "Identifier 'snake_cased' is not in camel case.",
              hint: "Consider renaming `snake_cased` to `snakeCased`",
            }
          ],
    r#"import * as snake_cased from 'mod'"#: [
            {
              col: 12,
              message: "Identifier 'snake_cased' is not in camel case.",
              hint: "Consider renaming `snake_cased` to `snakeCased`",
            }
          ],
    r#"export * as snake_cased from 'mod'"#: [
            {
              col: 12,
              message: "Identifier 'snake_cased' is not in camel case.",
              hint: "Consider renaming `snake_cased` to `snakeCased`",
            }
          ],
    r#"function foo({ no_camelcased }) {};"#: [
            {
              col: 15,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"function foo({ no_camelcased = 'default value' }) {};"#: [
            {
              col: 15,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"const no_camelcased = 0; function foo({ camelcased_value = no_camelcased }) {}"#: [
            {
              col: 6,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            },
            {
              col: 40,
              message: "Identifier 'camelcased_value' is not in camel case.",
              hint: "Consider renaming `camelcased_value` to `camelcasedValue`",
            }
          ],
    r#"const { bar: no_camelcased } = foo;"#: [
            {
              col: 13,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"function foo({ value_1: my_default }) {}"#: [
            {
              col: 24,
              message: "Identifier 'my_default' is not in camel case.",
              hint: "Consider renaming `my_default` to `myDefault`",
            }
          ],
    r#"function foo({ isCamelcased: no_camelcased }) {};"#: [
            {
              col: 29,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"function foo({ isCamelcased: { no_camelcased } }) {};"#: [
            {
              col: 31,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"var { foo: bar_baz = 1 } = quz;"#: [
            {
              col: 11,
              message: "Identifier 'bar_baz' is not in camel case.",
              hint: "Consider renaming `bar_baz` to `barBaz`",
            }
          ],
    r#"const f = function no_camelcased() {};"#: [
            {
              col: 19,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"const c = class no_camelcased {};"#: [
            {
              col: 16,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `NoCamelcased`",
            }
          ],
    r#"class no_camelcased {}"#: [
            {
              col: 6,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `NoCamelcased`",
            }
          ],
    r#"type foo_bar = string;"#: [
            {
              col: 5,
              message: "Identifier 'foo_bar' is not in camel case.",
              hint: "Consider renaming `foo_bar` to `FooBar`",
            }
          ],
    r#"type Foo = { snake_case: number; };"#: [
            {
              col: 13,
              message: "Identifier 'snake_case' is not in camel case.",
              hint: r#"Consider renaming `snake_case` to `snakeCase`, or wrapping it in quotation mark like `"snake_case"`"#,
            }
          ],
    r#"interface foo_bar { ok: string; };"#: [
            {
              col: 10,
              message: "Identifier 'foo_bar' is not in camel case.",
              hint: "Consider renaming `foo_bar` to `FooBar`",
            }
          ],
    r#"interface Foo { snake_case: number; };"#: [
            {
              col: 16,
              message: "Identifier 'snake_case' is not in camel case.",
              hint: r#"Consider renaming `snake_case` to `snakeCase`, or wrapping it in quotation mark like `"snake_case"`"#,
            }
          ],
    r#"namespace foo_bar {}"#: [
            {
              col: 10,
              message: "Identifier 'foo_bar' is not in camel case.",
              hint: "Consider renaming `foo_bar` to `FooBar`",
            }
          ],
    r#"namespace FooBar { const snake_case = 42; }"#: [
            {
              col: 25,
              message: "Identifier 'snake_case' is not in camel case.",
              hint: "Consider renaming `snake_case` to `snakeCase`",
            }
          ],
    r#"enum foo_bar { VariantOne }"#: [
            {
              col: 5,
              message: "Identifier 'foo_bar' is not in camel case.",
              hint: "Consider renaming `foo_bar` to `FooBar`",
            }
          ],
    r#"enum FooBar { variant_one }"#: [
            {
              col: 14,
              message: "Identifier 'variant_one' is not in camel case.",
              hint: "Consider renaming `variant_one` to `VariantOne`",
            }
          ],
    };
  }
}
