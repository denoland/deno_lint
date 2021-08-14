// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use crate::swc_util::StringRepr;
use dprint_swc_ecma_ast_view::{self as ast_view, Spanned};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};
use swc_common::Span;

pub struct Camelcase;

const CODE: &str = "camelcase";

impl LintRule for Camelcase {
  fn new() -> Box<Self> {
    Box::new(Camelcase)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    _context: &mut Context<'view>,
    _program: ProgramRef<'view>,
  ) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: ast_view::Program,
  ) {
    let mut handler = CamelcaseHandler::default();
    handler.traverse(program, context);
    handler.report_errors(context);
  }

  fn docs(&self) -> &'static str {
    r#"Enforces the use of camelCase in variable names

Consistency in a code base is key for readability and maintainability.  This rule
enforces variable declarations and object property names which you create to be
in camelCase.

Of note:

* `_` is allowed at the start or end of a variable
* All uppercase variable names (e.g. constants) may have `_` in their name
* If you have to use a snake_case key in an object for some reasons, wrap it in quotation mark
* This rule also applies to variables imported or exported via ES modules, but not to object properties of those variables

### Invalid:

```typescript
let first_name = "Ichigo";
const obj1 = { last_name: "Hoshimiya" };
const obj2 = { first_name };
const { last_name } = obj1;

function do_something(){}
function foo({ snake_case = "default value" }) {}

class snake_case_class {}
class Also_Not_Valid_Class {}

import { not_camelCased } from "external-module.js";
export * as not_camelCased from "mod.ts";

enum snake_case_enum { snake_case_variant }

type snake_case_type = { some_property: number; };

interface snake_case_interface { some_property: number; }
```

### Valid:

```typescript
let firstName = "Ichigo";
const FIRST_NAME = "Ichigo";
const __myPrivateVariable = "Hoshimiya";
const myPrivateVariable_ = "Hoshimiya";
const obj1 = { "last_name": "Hoshimiya" }; // if an object key is wrapped in quotation mark, then it's valid
const obj2 = { "first_name": first_name };
const { last_name: lastName } = obj;

function doSomething(){} // function declarations must be camelCase but...
do_something();  // ...snake_case function calls are allowed
function foo({ snake_case: camelCase = "default value" }) {}

class PascalCaseClass {}

import { not_camelCased as camelCased } from "external-module.js";
export * as camelCased from "mod.ts";

enum PascalCaseEnum { PascalCaseVariant }

type PascalCaseType = { someProperty: number; };

interface PascalCaseInterface { someProperty: number; }
```
"#
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
  ) -> Self
  where
    K: AsRef<str>,
    V: AsRef<str>,
  {
    Self::ObjectPat {
      key_name: key_name.as_ref().to_string(),
      value_name: value_name.map(|v| v.as_ref().to_string()),
      has_default,
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
      } => {
        if let Some(value_name) = value_name {
          return format!(
            "Consider renaming `{}` to `{}`",
            value_name,
            to_camelcase(value_name),
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
  fn check_ident<S: Spanned>(&mut self, span: &S, ident: IdentToCheck) {
    let span = span.span();
    if self.visited.insert(span) && is_underscored(ident.get_ident_name()) {
      self.errors.insert(span, ident);
    }
  }

  fn check_ts_type(&mut self, ty: &ast_view::TsType) {
    if let ast_view::TsType::TsTypeLit(type_lit) = ty {
      for member in &type_lit.members {
        self.check_ts_type_element(member);
      }
    }
  }

  fn check_ts_type_element(&mut self, ty_el: &ast_view::TsTypeElement) {
    use ast_view::TsTypeElement::*;
    match ty_el {
      TsPropertySignature(prop_sig) => {
        if let ast_view::Expr::Ident(ident) = prop_sig.key {
          self.check_ident(ident, IdentToCheck::object_key(ident.inner, false));
        }
        if let Some(type_ann) = &prop_sig.type_ann {
          self.check_ts_type(&type_ann.type_ann);
        }
      }
      TsMethodSignature(method_sig) => {
        if let ast_view::Expr::Ident(ident) = method_sig.key {
          self.check_ident(ident, IdentToCheck::function(ident.inner));
        }
        if let Some(type_ann) = &method_sig.type_ann {
          self.check_ts_type(&type_ann.type_ann);
        }
      }
      TsGetterSignature(getter_sig) => {
        if let ast_view::Expr::Ident(ident) = getter_sig.key {
          self.check_ident(ident, IdentToCheck::function(ident.inner));
        }
        if let Some(type_ann) = &getter_sig.type_ann {
          self.check_ts_type(&type_ann.type_ann);
        }
      }
      TsSetterSignature(setter_sig) => {
        if let ast_view::Expr::Ident(ident) = setter_sig.key {
          self.check_ident(ident, IdentToCheck::function(ident.inner));
        }
      }
      TsIndexSignature(_)
      | TsCallSignatureDecl(_)
      | TsConstructSignatureDecl(_) => {}
    }
  }

  fn check_pat(&mut self, pat: &ast_view::Pat) {
    match pat {
      ast_view::Pat::Ident(ident) => {
        self.check_ident(ident, IdentToCheck::variable(&ident.id.inner));
      }
      ast_view::Pat::Array(ast_view::ArrayPat { ref elems, .. }) => {
        for pat in elems.iter().flatten() {
          self.check_pat(pat);
        }
      }
      ast_view::Pat::Rest(ast_view::RestPat { ref arg, .. }) => {
        self.check_pat(arg);
      }
      ast_view::Pat::Object(ast_view::ObjectPat { ref props, .. }) => {
        for prop in props {
          match prop {
            ast_view::ObjectPatProp::KeyValue(ast_view::KeyValuePatProp {
              ref key,
              ref value,
              ..
            }) => match value {
              ast_view::Pat::Ident(value_ident) => {
                self.check_ident(
                  value_ident,
                  IdentToCheck::object_pat(
                    &key.string_repr().unwrap_or_else(|| "[KEY]".to_string()),
                    Some(&value_ident.id.inner),
                    false,
                  ),
                );
              }
              ast_view::Pat::Assign(ast_view::AssignPat {
                ref left, ..
              }) => {
                if let ast_view::Pat::Ident(value_ident) = left {
                  self.check_ident(
                    value_ident,
                    IdentToCheck::object_pat(
                      &key.string_repr().unwrap_or_else(|| "[KEY]".to_string()),
                      Some(&value_ident.id.inner),
                      true,
                    ),
                  );
                } else {
                  self.check_pat(value);
                }
              }
              _ => {
                self.check_pat(value);
              }
            },
            ast_view::ObjectPatProp::Assign(ast_view::AssignPatProp {
              ref key,
              ref value,
              ..
            }) => {
              let has_default = value.is_some();
              self.check_ident(
                key,
                IdentToCheck::object_pat::<&str, &str>(
                  &key.inner.as_ref(),
                  None,
                  has_default,
                ),
              );
            }
            ast_view::ObjectPatProp::Rest(ast_view::RestPat {
              ref arg,
              ..
            }) => {
              self.check_pat(arg);
            }
          }
        }
      }
      ast_view::Pat::Assign(ast_view::AssignPat { ref left, .. }) => {
        self.check_pat(left);
      }
      ast_view::Pat::Expr(expr) => {
        if let ast_view::Expr::Ident(ident) = expr {
          self.check_ident(ident, IdentToCheck::variable(ident.inner));
        }
      }
      ast_view::Pat::Invalid(_) => {}
    }
  }
}

impl Handler for CamelcaseHandler {
  fn fn_decl(&mut self, fn_decl: &ast_view::FnDecl, ctx: &mut Context) {
    if fn_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      &fn_decl.ident,
      IdentToCheck::function(&fn_decl.ident.inner),
    );
  }

  fn class_decl(
    &mut self,
    class_decl: &ast_view::ClassDecl,
    ctx: &mut Context,
  ) {
    if class_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      &class_decl.ident,
      IdentToCheck::class(&class_decl.ident.inner),
    );
  }

  fn var_decl(&mut self, var_decl: &ast_view::VarDecl, ctx: &mut Context) {
    if var_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    for decl in &var_decl.decls {
      self.check_pat(&decl.name);

      if let Some(expr) = &decl.init {
        match expr {
          ast_view::Expr::Object(ast_view::ObjectLit { ref props, .. }) => {
            for prop in props {
              if let ast_view::PropOrSpread::Prop(prop) = prop {
                match prop {
                  ast_view::Prop::Shorthand(ident) => self.check_ident(
                    ident,
                    IdentToCheck::object_key(ident.inner, true),
                  ),
                  ast_view::Prop::KeyValue(ast_view::KeyValueProp {
                    ref key,
                    ..
                  })
                  | ast_view::Prop::Getter(ast_view::GetterProp {
                    ref key,
                    ..
                  })
                  | ast_view::Prop::Setter(ast_view::SetterProp {
                    ref key,
                    ..
                  })
                  | ast_view::Prop::Method(ast_view::MethodProp {
                    ref key,
                    ..
                  }) => {
                    if let ast_view::PropName::Ident(ident) = key {
                      self.check_ident(
                        ident,
                        IdentToCheck::object_key(ident.inner, false),
                      );
                    }
                  }
                  ast_view::Prop::Assign(_) => {}
                }
              }
            }
          }
          ast_view::Expr::Fn(ast_view::FnExpr {
            ident: Some(ident), ..
          }) => {
            self.check_ident(ident, IdentToCheck::function(ident.inner));
          }
          ast_view::Expr::Class(ast_view::ClassExpr {
            ident: Some(ident),
            ..
          }) => {
            self.check_ident(ident, IdentToCheck::class(ident.inner));
          }
          _ => {}
        }
      }
    }
  }

  fn param(&mut self, param: &ast_view::Param, _ctx: &mut Context) {
    self.check_pat(&param.pat);
  }

  fn import_named_specifier(
    &mut self,
    import_named_specifier: &ast_view::ImportNamedSpecifier,
    _ctx: &mut Context,
  ) {
    let ast_view::ImportNamedSpecifier {
      local, imported, ..
    } = import_named_specifier;
    self.check_ident(
      local,
      IdentToCheck::named_import(
        local.inner,
        imported.as_ref().map(|i| i.inner),
      ),
    );
  }

  fn import_default_specifier(
    &mut self,
    import_default_specifier: &ast_view::ImportDefaultSpecifier,
    _ctx: &mut Context,
  ) {
    let ast_view::ImportDefaultSpecifier { local, .. } =
      import_default_specifier;
    self.check_ident(local, IdentToCheck::variable(local.inner));
  }

  fn import_star_as_specifier(
    &mut self,
    import_star_as_specifier: &ast_view::ImportStarAsSpecifier,
    _ctx: &mut Context,
  ) {
    let ast_view::ImportStarAsSpecifier { local, .. } =
      import_star_as_specifier;
    self.check_ident(local, IdentToCheck::variable(local.inner));
  }

  fn export_namespace_specifier(
    &mut self,
    export_namespace_specifier: &ast_view::ExportNamespaceSpecifier,
    _ctx: &mut Context,
  ) {
    let ast_view::ExportNamespaceSpecifier { name, .. } =
      export_namespace_specifier;
    self.check_ident(name, IdentToCheck::variable(name.inner));
  }

  fn ts_type_alias_decl(
    &mut self,
    type_alias: &ast_view::TsTypeAliasDecl,
    ctx: &mut Context,
  ) {
    if type_alias.declare() {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      &type_alias.id,
      IdentToCheck::type_alias(&type_alias.id.inner),
    );
    self.check_ts_type(&type_alias.type_ann);
  }

  fn ts_interface_decl(
    &mut self,
    interface_decl: &ast_view::TsInterfaceDecl,
    ctx: &mut Context,
  ) {
    if interface_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      &interface_decl.id,
      IdentToCheck::interface(&interface_decl.id.inner),
    );

    for ty_el in &interface_decl.body.body {
      self.check_ts_type_element(ty_el);
    }
  }

  fn ts_namespace_decl(
    &mut self,
    namespace_decl: &ast_view::TsNamespaceDecl,
    ctx: &mut Context,
  ) {
    if namespace_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    self.check_ident(
      &namespace_decl.id,
      IdentToCheck::namespace(&namespace_decl.id.inner),
    );
  }

  fn ts_module_decl(
    &mut self,
    module_decl: &ast_view::TsModuleDecl,
    ctx: &mut Context,
  ) {
    if module_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    if let ast_view::TsModuleName::Ident(id) = &module_decl.id {
      self.check_ident(id, IdentToCheck::module(id.inner));
    }
  }

  fn ts_enum_decl(
    &mut self,
    enum_decl: &ast_view::TsEnumDecl,
    ctx: &mut Context,
  ) {
    if enum_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    self
      .check_ident(&enum_decl.id, IdentToCheck::enum_name(&enum_decl.id.inner));
    for variant in &enum_decl.members {
      if let ast_view::TsEnumMemberId::Ident(id) = &variant.id {
        self.check_ident(id, IdentToCheck::enum_variant(id.inner));
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
        },
        "Consider replacing `{ foo_bar }` with `{ foo_bar: fooBar }`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: Some(s("snake_case")),
          has_default: false,
        },
        "Consider renaming `snake_case` to `snakeCase`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: None,
          has_default: true,
        },
        "Consider replacing `{ foo_bar = .. }` with `{ foo_bar: fooBar = .. }`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: Some(s("snake_case")),
          has_default: true,
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
      r#"import { camelCased } from "external module";"#,
      r#"import { _leading } from "external module";"#,
      r#"import { trailing_ } from "external module";"#,
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
    r#"var { category_id } = query;"#: [
            {
              col: 6,
              message: "Identifier 'category_id' is not in camel case.",
              hint: "Consider replacing `{ category_id }` with `{ category_id: categoryId }`",
            }
          ],
    r#"var { category_id: category_id } = query;"#: [
            {
              col: 19,
              message: "Identifier 'category_id' is not in camel case.",
              hint: "Consider renaming `category_id` to `categoryId`",
            }
          ],
    r#"var { category_id = 1 } = query;"#: [
            {
              col: 6,
              message: "Identifier 'category_id' is not in camel case.",
              hint: "Consider replacing `{ category_id = .. }` with `{ category_id: categoryId = .. }`",
            }
          ],
    r#"import no_camelcased from "external-module";"#: [
            {
              col: 7,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"import * as no_camelcased from "external-module";"#: [
            {
              col: 12,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider renaming `no_camelcased` to `noCamelcased`",
            }
          ],
    r#"import { no_camelcased } from "external-module";"#: [
            {
              col: 9,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ no_camelcased }` with `{ no_camelcased as noCamelcased }`",
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
    r#"import { camelCased, no_camelcased } from "external-module";"#: [
            {
              col: 21,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ no_camelcased }` with `{ no_camelcased as noCamelcased }`",
            }
          ],
    r#"import { no_camelcased as camelCased, another_no_camelcased } from "external-module";"#: [
            {
              col: 38,
              message: "Identifier 'another_no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ another_no_camelcased }` with `{ another_no_camelcased as anotherNoCamelcased }`",
            }
          ],
    r#"import camelCased, { no_camelcased } from "external-module";"#: [
            {
              col: 21,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ no_camelcased }` with `{ no_camelcased as noCamelcased }`",
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
              hint: "Consider replacing `{ no_camelcased }` with `{ no_camelcased: noCamelcased }`",
            }
          ],
    r#"function foo({ no_camelcased = 'default value' }) {};"#: [
            {
              col: 15,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ no_camelcased = .. }` with `{ no_camelcased: noCamelcased = .. }`",
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
              hint: "Consider replacing `{ camelcased_value = .. }` with `{ camelcased_value: camelcasedValue = .. }`",
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
    r#"var { foo: bar_baz = 1 } = quz;"#: [
            {
              col: 11,
              message: "Identifier 'bar_baz' is not in camel case.",
              hint: "Consider renaming `bar_baz` to `barBaz`",
            }
          ],
    r#"const { no_camelcased = false } = bar;"#: [
            {
              col: 8,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ no_camelcased = .. }` with `{ no_camelcased: noCamelcased = .. }`",
            }
          ],
    r#"const { no_camelcased = foo_bar } = bar;"#: [
            {
              col: 8,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ no_camelcased = .. }` with `{ no_camelcased: noCamelcased = .. }`",
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
