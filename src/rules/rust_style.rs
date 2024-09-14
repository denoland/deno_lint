// Copyright 2018-2024 the Deno authors + c-antin. All rights reserved. MIT license.
// based on camelcase.rs

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::swc_util::{find_lhs_ids, StringRepr};

use deno_ast::view::{AssignExpr, Node, NodeKind, NodeTrait};
use deno_ast::{
  view as ast_view, SourceRange, SourceRanged, SourceRangedForSpanned,
};
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct RustStyle;

const CODE: &str = "rust_style";

impl LintRule for RustStyle {
  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: ast_view::Program,
  ) {
    let mut handler = RustStyleHandler::default();
    handler.traverse(program, context);
    handler.report_errors(context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/rust_style.md")
  }
}

/// Check if it contains underscores, except for leading and trailing ones
fn is_underscored(ident_name: &str) -> bool {
  let trimmed_ident = ident_name.trim_matches('_');
  trimmed_ident.contains('_')
    && trimmed_ident != trimmed_ident.to_ascii_uppercase()
}

/// Check if it is snake cased
fn is_snake_cased(ident_name: &str) -> bool {
  static UPPERCASE_CHAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[A-Z]").unwrap());
  !UPPERCASE_CHAR_RE.is_match(ident_name)
}

/// Check if it is screaming snake cased
fn is_screaming_snake_cased(ident_name: &str) -> bool {
  static LOWERCASE_CHAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"[a-z]").unwrap());
  !LOWERCASE_CHAR_RE.is_match(ident_name)
}

/// Check if it is upper camel cased
fn is_upper_camel_cased(ident_name: &str) -> bool {
  if is_underscored(ident_name) {
    return false;
  }
  static UPPERCASE_FIRST_CHAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^[A-Z]").unwrap());
  UPPERCASE_FIRST_CHAR_RE.is_match(ident_name)
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

/// Convert the name of identifier into rust style. If the name is originally in rust style, return
/// the name as it is. If name starts with uppercase letter, return as is. For more detail, see the test cases below.
fn to_rust_style(ident_name: &str) -> String {
  let trimmed_ident = ident_name.trim_matches('_');
  if let Some(first_char) = trimmed_ident.chars().next() {
    if first_char.is_uppercase() {
      return ident_name.to_string();
    }
  }

  static LOWERCASE_UPPERCASE_CHAR_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"([^A-Z])([A-Z])").unwrap());

  let result = LOWERCASE_UPPERCASE_CHAR_RE
    .replace_all(ident_name, |caps: &Captures| {
      format!("{}_{}", &caps[1], caps[2].to_ascii_lowercase())
    });

  if result != ident_name {
    return result.into_owned();
  }

  ident_name.to_string()
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
    K: AsRef<str>,
    V: AsRef<str>,
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
      "Identifier '{}' is not in rust style.",
      self.get_ident_name()
    )
  }

  fn to_hint(&self) -> String {
    match self {
      IdentToCheck::Variable(name) | IdentToCheck::Function(name) => {
        format!("Consider renaming `{}` to `{}`", name, to_rust_style(name))
      }
      IdentToCheck::ObjectKey {
        ref key_name,
        is_shorthand,
      } => {
        if *is_shorthand {
          format!(
            r#"Consider writing `{rust_styled}: {original}` or `"{original}": {original}`"#,
            rust_styled = to_rust_style(key_name),
            original = key_name
          )
        } else {
          format!(
            r#"Consider renaming `{original}` to `{rust_styled}`, or wrapping it in quotation mark like `"{original}"`"#,
            rust_styled = to_rust_style(key_name),
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
            to_rust_style(name),
          );
        }

        if *has_default {
          return format!(
            "Consider replacing `{{ {key} = .. }}` with `{{ {key}: {value} = .. }}`",
            key = key_name,
            value = to_rust_style(key_name),
          );
        }

        format!(
          "Consider replacing `{{ {key} }}` with `{{ {key}: {value} }}`",
          key = key_name,
          value = to_rust_style(key_name),
        )
      }
      IdentToCheck::NamedImport { local, imported } => {
        if imported.is_some() {
          format!(
            "Consider renaming `{}` to `{}`",
            local,
            to_rust_style(local)
          )
        } else {
          format!(
            "Consider replacing `{{ {local} }}` with `{{ {local} as {rust_styled_local} }}`",
            local = local,
            rust_styled_local = to_rust_style(local),
          )
        }
      }
    }
  }
}

#[derive(Default)]
struct RustStyleHandler {
  /// Accumulated errors to report
  errors: BTreeMap<SourceRange, IdentToCheck>,
  /// Already visited identifiers
  visited: BTreeSet<SourceRange>,
}

impl RustStyleHandler {
  /// Report accumulated errors, consuming `self`.
  fn report_errors(self, ctx: &mut Context) {
    for (range, error_ident) in self.errors {
      ctx.add_diagnostic_with_hint(
        range,
        CODE,
        error_ident.to_message(),
        error_ident.to_hint(),
      );
    }
  }

  // /// Check if this ident is not underscored only when it's not yet visited.
  // fn check_ident_not_underscored<S: SourceRanged>(
  //   &mut self,
  //   range: &S,
  //   ident: IdentToCheck,
  // ) {
  //   let range = range.range();
  //   if self.visited.insert(range) && is_underscored(ident.get_ident_name()) {
  //     self.errors.insert(range, ident);
  //   }
  // }

  /// Check if this ident is snake cased only when it's not yet visited.
  fn check_ident_snake_cased<S: SourceRanged>(
    &mut self,
    range: &S,
    ident: IdentToCheck,
  ) {
    let range = range.range();
    if self.visited.insert(range) && !is_snake_cased(ident.get_ident_name()) {
      self.errors.insert(range, ident);
    }
  }

  /// Check if this ident is snake cased or screaming snake cased only when it's not yet visited.
  fn check_ident_snake_cased_or_screaming_snake_cased<S: SourceRanged>(
    &mut self,
    range: &S,
    ident: IdentToCheck,
  ) {
    let range = range.range();
    if self.visited.insert(range)
      && !is_snake_cased(ident.get_ident_name())
      && !is_screaming_snake_cased(ident.get_ident_name())
    {
      self.errors.insert(range, ident);
    }
  }

  /// Check if this ident is upper camel cased only when it's not yet visited.
  fn check_ident_upper_camel_cased<S: SourceRanged>(
    &mut self,
    range: &S,
    ident: IdentToCheck,
  ) {
    let range = range.range();
    if self.visited.insert(range)
      && !is_upper_camel_cased(ident.get_ident_name())
    {
      self.errors.insert(range, ident);
    }
  }

  fn check_ts_type(&mut self, ty: &ast_view::TsType) {
    if let ast_view::TsType::TsTypeLit(type_lit) = ty {
      for member in type_lit.members {
        self.check_ts_type_element(member);
      }
    }
  }

  fn check_ts_type_element(&mut self, ty_el: &ast_view::TsTypeElement) {
    use deno_ast::view::TsTypeElement::*;
    match ty_el {
      TsPropertySignature(prop_sig) => {
        if let ast_view::Expr::Ident(ident) = prop_sig.key {
          self.check_ident_snake_cased(
            ident,
            IdentToCheck::object_key(ident.inner, false),
          );
        }
        if let Some(type_ann) = &prop_sig.type_ann {
          self.check_ts_type(&type_ann.type_ann);
        }
      }
      TsMethodSignature(method_sig) => {
        if let ast_view::Expr::Ident(ident) = method_sig.key {
          self.check_ident_snake_cased(
            ident,
            IdentToCheck::function(ident.inner),
          );
        }
        if let Some(type_ann) = &method_sig.type_ann {
          self.check_ts_type(&type_ann.type_ann);
        }
      }
      TsGetterSignature(getter_sig) => {
        if let ast_view::Expr::Ident(ident) = getter_sig.key {
          self.check_ident_snake_cased(
            ident,
            IdentToCheck::function(ident.inner),
          );
        }
        if let Some(type_ann) = &getter_sig.type_ann {
          self.check_ts_type(&type_ann.type_ann);
        }
      }
      TsSetterSignature(setter_sig) => {
        if let ast_view::Expr::Ident(ident) = setter_sig.key {
          self.check_ident_snake_cased(
            ident,
            IdentToCheck::function(ident.inner),
          );
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
        self.check_ident_snake_cased(
          ident,
          IdentToCheck::variable(ident.id.inner),
        );
      }
      ast_view::Pat::Array(ast_view::ArrayPat { elems, .. }) => {
        for pat in elems.iter().flatten() {
          self.check_pat(pat);
        }
      }
      ast_view::Pat::Rest(ast_view::RestPat { ref arg, .. }) => {
        self.check_pat(arg);
      }
      ast_view::Pat::Object(ast_view::ObjectPat { props, .. }) => {
        for prop in *props {
          match prop {
            ast_view::ObjectPatProp::KeyValue(ast_view::KeyValuePatProp {
              ref key,
              ref value,
              ..
            }) => match value {
              ast_view::Pat::Ident(value_ident) => {
                self.check_ident_snake_cased(
                  value_ident,
                  IdentToCheck::object_pat(
                    &key.string_repr().unwrap_or_else(|| "[KEY]".to_string()),
                    Some(&value_ident.id.inner),
                    false,
                    pat_in_var_declarator(pat.into()),
                  ),
                );
              }
              ast_view::Pat::Assign(ast_view::AssignPat {
                left: ast_view::Pat::Ident(value_ident),
                ..
              }) => {
                self.check_ident_snake_cased(
                  value_ident,
                  IdentToCheck::object_pat(
                    &key.string_repr().unwrap_or_else(|| "[KEY]".to_string()),
                    Some(&value_ident.id.inner),
                    true,
                    pat_in_var_declarator(pat.into()),
                  ),
                );
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
              let in_var_declarator = pat_in_var_declarator(pat.into());
              if !in_var_declarator {
                self.check_ident_snake_cased(
                  key,
                  IdentToCheck::object_pat::<&str, &str>(
                    &key.inner.as_ref(),
                    None,
                    has_default,
                    in_var_declarator,
                  ),
                );
              }
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
          self.check_ident_snake_cased(
            ident,
            IdentToCheck::variable(ident.inner),
          );
        }
      }
      ast_view::Pat::Invalid(_) => {}
    }
  }
}

impl Handler for RustStyleHandler {
  fn assign_expr(&mut self, e: &AssignExpr, _ctx: &mut Context) {
    let idents: Vec<deno_ast::swc::ast::Ident> = find_lhs_ids(&e.left);

    for ident in idents {
      self.check_ident_snake_cased_or_screaming_snake_cased(
        &ident.range(),
        IdentToCheck::variable(ident.to_id().0),
      );
    }
  }

  fn fn_decl(&mut self, fn_decl: &ast_view::FnDecl, ctx: &mut Context) {
    if fn_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    self.check_ident_snake_cased(
      &fn_decl.ident,
      IdentToCheck::function(fn_decl.ident.inner),
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

    self.check_ident_upper_camel_cased(
      &class_decl.ident,
      IdentToCheck::class(class_decl.ident.inner),
    );
  }

  fn var_decl(&mut self, var_decl: &ast_view::VarDecl, ctx: &mut Context) {
    if var_decl.declare() {
      ctx.stop_traverse();
      return;
    }

    for decl in var_decl.decls {
      self.check_pat(&decl.name);

      if let Some(expr) = &decl.init {
        match expr {
          ast_view::Expr::Object(ast_view::ObjectLit { props, .. }) => {
            for prop in *props {
              if let ast_view::PropOrSpread::Prop(prop) = prop {
                match prop {
                  ast_view::Prop::Shorthand(ident) => self
                    .check_ident_snake_cased(
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
                      self.check_ident_snake_cased(
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
            self.check_ident_snake_cased(
              ident,
              IdentToCheck::function(ident.inner),
            );
          }
          ast_view::Expr::Class(ast_view::ClassExpr {
            ident: Some(ident),
            ..
          }) => {
            self.check_ident_upper_camel_cased(
              ident,
              IdentToCheck::class(ident.inner),
            );
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
    if let Some(imported) = &imported {
      self.check_ident_snake_cased(
        local,
        IdentToCheck::named_import(
          local.inner,
          Some(match imported {
            ast_view::ModuleExportName::Ident(ident) => ident.sym(),
            ast_view::ModuleExportName::Str(str) => str.value(),
          }),
        ),
      );
    }
  }

  fn import_default_specifier(
    &mut self,
    import_default_specifier: &ast_view::ImportDefaultSpecifier,
    _ctx: &mut Context,
  ) {
    let ast_view::ImportDefaultSpecifier { local, .. } =
      import_default_specifier;
    self.check_ident_snake_cased(local, IdentToCheck::variable(local.inner));
  }

  fn import_star_as_specifier(
    &mut self,
    import_star_as_specifier: &ast_view::ImportStarAsSpecifier,
    _ctx: &mut Context,
  ) {
    let ast_view::ImportStarAsSpecifier { local, .. } =
      import_star_as_specifier;
    self.check_ident_snake_cased(local, IdentToCheck::variable(local.inner));
  }

  fn export_namespace_specifier(
    &mut self,
    export_namespace_specifier: &ast_view::ExportNamespaceSpecifier,
    _ctx: &mut Context,
  ) {
    let ast_view::ExportNamespaceSpecifier { name, .. } =
      export_namespace_specifier;
    if let ast_view::ModuleExportName::Ident(name) = name {
      self.check_ident_snake_cased(name, IdentToCheck::variable(name.inner));
    }
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

    self.check_ident_upper_camel_cased(
      &type_alias.id,
      IdentToCheck::type_alias(type_alias.id.inner),
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

    self.check_ident_upper_camel_cased(
      &interface_decl.id,
      IdentToCheck::interface(interface_decl.id.inner),
    );

    for ty_el in interface_decl.body.body {
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

    self.check_ident_upper_camel_cased(
      &namespace_decl.id,
      IdentToCheck::namespace(namespace_decl.id.inner),
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
      self.check_ident_upper_camel_cased(id, IdentToCheck::module(id.inner));
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

    self.check_ident_upper_camel_cased(
      &enum_decl.id,
      IdentToCheck::enum_name(enum_decl.id.inner),
    );
    for variant in enum_decl.members {
      if let ast_view::TsEnumMemberId::Ident(id) = &variant.id {
        self.check_ident_upper_camel_cased(
          id,
          IdentToCheck::enum_variant(id.inner),
        );
      }
    }
  }
}

fn pat_in_var_declarator(pat: Node) -> bool {
  for ancestor in pat.ancestors() {
    match ancestor.kind() {
      NodeKind::VarDeclarator => {
        return true;
      }
      NodeKind::ArrayPat
      | NodeKind::ObjectPat
      | NodeKind::AssignPat
      | NodeKind::AssignPatProp
      | NodeKind::RestPat
      | NodeKind::KeyValuePatProp => {
        // keep going
      }
      _ => {
        return false;
      }
    }
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_is_underscored() {
    let tests = [
      ("foo_bar", true),         //snake
      ("fooBar", false),         //camel
      ("FooBar", false),         //upper camel
      ("foo_bar_baz", true),     //snake
      ("_foo_bar_baz", true),    //snake
      ("__foo_bar_baz__", true), //snake
      ("__fooBar_baz__", true),  //snake
      ("__fooBarBaz__", false),  //camel
      ("Sha3_224", true),        //not snake
      ("SHA3_224", false),       //screaming snake
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(expected, is_underscored(input));
    }
  }

  #[test]
  fn test_is_snake_cased() {
    let tests = [
      ("foo_bar", true),         //snake
      ("fooBar", false),         //camel
      ("FooBar", false),         //upper camel
      ("foo_bar_baz", true),     //snake
      ("_foo_bar_baz", true),    //snake
      ("__foo_bar_baz__", true), //snake
      ("__fooBar_baz__", false), //not snake
      ("__fooBarBaz__", false),  //camel
      ("Sha3_224", false),       //not snake
      ("SHA3_224", false),       //screaming snake
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(expected, is_snake_cased(input));
    }
  }

  #[test]
  fn test_is_screaming_snake_cased() {
    let tests = [
      ("foo_bar", false),         //snake
      ("fooBar", false),          //camel
      ("FooBar", false),          //upper camel
      ("foo_bar_baz", false),     //snake
      ("_foo_bar_baz", false),    //snake
      ("__foo_bar_baz__", false), //snake
      ("__fooBar_baz__", false),  //not snake
      ("__fooBarBaz__", false),   //camel
      ("Sha3_224", false),        //not snake
      ("SHA3_224", true),         //screaming snake
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(expected, is_screaming_snake_cased(input));
    }
  }

  #[test]
  fn test_is_upper_camel_cased() {
    let tests = [
      ("foo_bar", false),         //snake
      ("fooBar", false),          //camel
      ("FooBar", true),           //upper camel
      ("foo_bar_baz", false),     //snake
      ("_foo_bar_baz", false),    //snake
      ("__foo_bar_baz__", false), //snake
      ("__fooBar_baz__", false),  //not snake
      ("__fooBarBaz__", false),   //camel
      ("Sha3_224", false),        //not snake
      ("SHA3_224", true),         //screaming snake; todo: should this be true?
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(expected, is_upper_camel_cased(input));
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
  fn test_to_rust_style() {
    let tests = [
      ("fooBar", "foo_bar"),
      ("foo_bar", "foo_bar"),
      ("FooBar", "FooBar"),
      ("_FooBar", "_FooBar"),
      ("fooBarBaz", "foo_bar_baz"),
      ("_fooBarBaz", "_foo_bar_baz"),
      ("__fooBarBaz__", "__foo_bar_baz__"),
      ("Sha3_224", "Sha3_224"),
      ("SHA3_224", "SHA3_224"),
      ("_leading", "_leading"),
      ("trailing_", "trailing_"),
      ("_bothends_", "_bothends_"),
    ];

    for &(input, expected) in tests.iter() {
      assert_eq!(expected, to_rust_style(input));
    }
  }

  #[test]
  fn test_to_hint() {
    fn s(s: &str) -> String {
      s.to_string()
    }

    let tests = [
      (
        IdentToCheck::Variable(s("fooBar")),
        "Consider renaming `fooBar` to `foo_bar`",
      ),
      (
        IdentToCheck::Function(s("fooBar")),
        "Consider renaming `fooBar` to `foo_bar`",
      ),
      (
        IdentToCheck::Class(s("foo_bar")),
        "Consider renaming `foo_bar` to `FooBar`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("fooBar"),
          value_name: None,
          has_default: false,
          is_destructuring: true,
        },
        "Consider replacing `{ fooBar }` with `{ fooBar: foo_bar }`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("fooBar"),
          value_name: Some(s("camelCase")),
          has_default: false,
          is_destructuring: true,
        },
        "Consider renaming `camelCase` to `camel_case`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("fooBar"),
          value_name: None,
          has_default: true,
          is_destructuring: true,
        },
        "Consider replacing `{ fooBar = .. }` with `{ fooBar: foo_bar = .. }`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("fooBar"),
          value_name: None,
          has_default: true,
          is_destructuring: false,
        },
        // not destructuring, so suggest a rename
        "Consider renaming `fooBar` to `foo_bar`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: Some(s("camelCase")),
          has_default: true,
          is_destructuring: true,
        },
        "Consider renaming `camelCase` to `camel_case`",
      ),
      (
        IdentToCheck::NamedImport {
          local: s("fooBar"),
          imported: None,
        },
        "Consider replacing `{ fooBar }` with `{ fooBar as foo_bar }`",
      ),
      (
        IdentToCheck::NamedImport {
          local: s("fooBar"),
          imported: Some(s("camelCase")),
        },
        "Consider renaming `fooBar` to `foo_bar`",
      ),
    ];

    for (error_ident, expected) in tests.iter() {
      assert_eq!(*expected, error_ident.to_hint());
    }
  }

  // Based on https://github.com/eslint/eslint/blob/v7.8.1/tests/lib/rules/camelcase.js

  #[test]
  fn rust_style_valid() {
    assert_lint_ok! {
      RustStyle,
      // r#"firstName = "Ichigo""#,// see rust_style_invalid below
      r#"first_name = "Ichigo""#,// new
      r#"FIRST_NAME = "Ichigo""#,
      // r#"__myPrivateVariable = "Hoshimiya""#,// see rust_style_invalid below
      r#"__my_private_variable = "Hoshimiya""#,// new
      // r#"myPrivateVariable_ = "Hoshimiya""#,// see rust_style_invalid below
      r#"my_private_variable_ = "Hoshimiya""#,// new
      // r#"function doSomething(){}"#,// see rust_style_invalid below
      r#"do_something()"#,// new
      r#"new do_something"#,// still valid, if external class
      r#"new DoSomething"#,// new
      r#"new do_something()"#,// still valid, if external class
      r#"new DoSomething()"#,// new
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
      // r#"import { no_camelcased as camelCased } from "external-module";"#,// see rust_style_invalid below
      r#"import { camelCased as no_camelcased } from "external-module";"#,// new
      r#"import { no_camelcased as _leading } from "external-module";"#,
      r#"import { no_camelcased as trailing_ } from "external-module";"#,
      // r#"import { no_camelcased as camelCased, anotherCamelCased } from "external-module";"#,// see rust_style_invalid below
      r#"import { camelCased as no_camelcased, anotherCamelCased } from "external-module";"#,// new
      r#"import { camelCased } from 'mod'"#,// still valid, if external module
      r#"import { no_camelcased } from 'mod'"#,// new
      // r#"var _camelCased = aGlobalVariable"#,// see rust_style_invalid below
      r#"var _no_camelcased = aGlobalVariable"#,// new
      // r#"var camelCased = _aGlobalVariable"#,// see rust_style_invalid below
      r#"var no_camelcased = _aGlobalVariable"#,// new
      // r#"function foo({ no_camelcased: camelCased }) {};"#,// see rust_style_invalid below
      r#"function foo({ camelCased: no_camelcased }) {};"#,// new
      r#"function foo({ no_camelcased: _leading }) {};"#,
      r#"function foo({ no_camelcased: trailing_ }) {};"#,
      // r#"function foo({ camelCased = 'default value' }) {};"#,// see rust_style_invalid below
      r#"function foo({ no_camelcased = 'default value' }) {};"#,// new
      r#"function foo({ _leading = 'default value' }) {};"#,
      r#"function foo({ trailing_ = 'default value' }) {};"#,
      // r#"function foo({ camelCased }) {};"#,// see rust_style_invalid below
      r#"function foo({ no_camelcased }) {};"#,// new
      r#"function foo({ _leading }) {}"#,
      r#"function foo({ trailing_ }) {}"#,
      r#"({obj} = baz.fo_o);"#,
      r#"([obj] = baz.fo_o);"#,
      r#"([obj.foo = obj.fo_o] = bar);"#,
      // r#"const f = function camelCased() {};"#,// see rust_style_invalid below
      r#"const f = function no_camelcased() {};"#,// new
      // r#"const c = class camelCased {};"#,// see rust_style_invalid below
      r#"const c = class UpperCamelCased {};"#,// new
      // r#"class camelCased {};"#,// see rust_style_invalid below
      r#"class UpperCamelCased {};"#,// new

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
      // r#"var { [category_id]: categoryId } = query;"#,// see rust_style_invalid below
      r#"var { [categoryId]: category_id } = query;"#,// new
      r#"a_global_variable.foo()"#,
      r#"a_global_variable[undefined]"#,
      // r#"var camelCased = snake_cased"#,// see rust_style_invalid below
      r#"var snake_cased = camelCased"#,// new
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
      r#"const obj = { "createdAt": "2020-10-30T13:16:45+09:00" }"#,// new
      r#"const obj = { "created_at": created_at }"#,
      r#"const obj = { "createdAt": created_at }"#,// new

      // https://github.com/denoland/deno_lint/issues/587
      // The rule shouldn't be applied to ambient declarations since the user who writes them is
      // unable to fix their names.
      r#"declare function foo_bar(a_b: number): void;"#,
      r#"declare function fooBar(aB: number): void;"#,// new
      r#"declare const foo_bar: number;"#,
      r#"declare const fooBar: number;"#,// new
      r#"declare const foo_bar: number, snake_case: string;"#,
      r#"declare const fooBar: number, camelCase: string;"#,// new
      r#"declare let foo_bar: { some_property: string; };"#,
      r#"declare let fooBar: { someProperty: string; };"#,// new
      r#"declare var foo_bar: number;"#,
      r#"declare var fooBar: number;"#,// new
      r#"declare class foo_bar { some_method(some_param: boolean): string; };"#,
      r#"declare class fooBar { someMethod(someParam: boolean): string; };"#,// new
      r#"export declare const foo_bar: number;"#,
      r#"export declare const fooBar: number;"#,// new
      r#"declare type foo_bar = { some_var: string; };"#,
      r#"declare type fooBar = { someVar: string; };"#,// new
      r#"declare interface foo_bar { some_var: string; }"#,
      r#"declare interface fooBar { someVar: string; }"#,// new
      r#"declare namespace foo_bar {}"#,
      r#"declare namespace fooBar {}"#,// new
      r#"declare enum foo_bar { variant_one, variant_two }"#,
      r#"declare enum fooVar { variantOne, variantTwo }"#,// new

      //new valid test cases:
      r#"function foo_bar(){}"#,
      r#"var foo = { bar_baz: boom.bam_pow }"#,
      r#"var o = { bar_baz: 1 }"#,
      r#"var o = { bar_baz }"#,
      r#"var { category_id: category_alias } = query;"#,
      r#"var { category_id: category_id } = query;"#,
      r#"import * as no_camelcased from "external-module";"#,
      r#"import { no_camelcased as no_camel_cased } from "external module";"#,
      r#"import { camelCased as no_camel_cased } from "external module";"#,
      r#"import no_camelcased, { anotherCamelCased as another_no_camelcased } from "external-module";"#,// new
      r#"import snake_cased from 'mod'"#,
      r#"import * as snake_cased from 'mod'"#,
      r#"export * as snake_cased from 'mod'"#,
      r#"function foo({ no_camelcased }) {};"#,
      r#"function foo({ no_camelcased = 'default value' }) {};"#,
      r#"const no_camelcased = 0; function foo({ camelcased_value = no_camelcased }) {}"#,
      r#"const { bar: no_camelcased } = foo;"#,
      r#"function foo({ value_1: my_default }) {}"#,
      r#"function foo({ isCamelcased: no_camelcased }) {};"#,
      r#"function foo({ isCamelcased: { no_camelcased } }) {};"#,
      r#"var { foo: bar_baz = 1 } = quz;"#,
      r#"const f = function no_camelcased() {};"#,
      r#"type Foo = { snake_case: number; };"#,
      r#"interface Foo { snake_case: number; };"#,
      r#"namespace FooBar { const snake_case = 42; }"#,
    };
  }

  #[test]
  fn rust_style_invalid() {
    assert_lint_err! {
      RustStyle,
      // r#"function foo_bar(){}"#: [
      //   {
      //     col: 9,
      //     message: "Identifier 'foo_bar' is not in rust style.",
      //     hint: "Consider renaming `foo_bar` to `fooBar`",
      //   }
      // ],
      r#"function fooBar(){}"#: [// new
        {
          col: 9,
          message: "Identifier 'fooBar' is not in rust style.",
          hint: "Consider renaming `fooBar` to `foo_bar`",
        }
      ],
      // r#"var foo = { bar_baz: boom.bam_pow }"#: [
      //   {
      //     col: 12,
      //     message: "Identifier 'bar_baz' is not in rust style.",
      //     hint: r#"Consider renaming `bar_baz` to `barBaz`, or wrapping it in quotation mark like `"bar_baz"`"#,
      //   }
      // ],
      r#"var foo = { barBaz: boom.bam_pow }"#: [// new
        {
          col: 12,
          message: "Identifier 'barBaz' is not in rust style.",
          hint: r#"Consider renaming `barBaz` to `bar_baz`, or wrapping it in quotation mark like `"barBaz"`"#,
        }
      ],
      // r#"var o = { bar_baz: 1 }"#: [
      //   {
      //     col: 10,
      //     message: "Identifier 'bar_baz' is not in rust style.",
      //     hint: r#"Consider renaming `bar_baz` to `barBaz`, or wrapping it in quotation mark like `"bar_baz"`"#,
      //   }
      // ],
      r#"var o = { barBaz: 1 }"#: [// new
        {
          col: 10,
          message: "Identifier 'barBaz' is not in rust style.",
          hint: r#"Consider renaming `barBaz` to `bar_baz`, or wrapping it in quotation mark like `"barBaz"`"#,
        }
      ],
      // r#"var o = { bar_baz }"#: [
      //   {
      //     col: 10,
      //     message: "Identifier 'bar_baz' is not in rust style.",
      //     hint: r#"Consider writing `barBaz: bar_baz` or `"bar_baz": bar_baz`"#,
      //   }
      // ],
      r#"var o = { barBaz }"#: [// new
        {
          col: 10,
          message: "Identifier 'barBaz' is not in rust style.",
          hint: r#"Consider writing `bar_baz: barBaz` or `"barBaz": barBaz`"#,
        }
      ],
      // r#"var { category_id: category_alias } = query;"#: [
      //   {
      //     col: 19,
      //     message: "Identifier 'category_alias' is not in rust style.",
      //     hint: "Consider renaming `category_alias` to `categoryAlias`",
      //   }
      // ],
      r#"var { category_id: categoryAlias } = query;"#: [// new
        {
          col: 19,
          message: "Identifier 'categoryAlias' is not in rust style.",
          hint: "Consider renaming `categoryAlias` to `category_alias`",
        }
      ],
      // r#"var { category_id: category_id } = query;"#: [
      //   {
      //     col: 19,
      //     message: "Identifier 'category_id' is not in rust style.",
      //     hint: "Consider renaming `category_id` to `categoryId`",
      //   }
      // ],
      r#"var { category_id: categoryId } = query;"#: [// new
        {
          col: 19,
          message: "Identifier 'categoryId' is not in rust style.",
          hint: "Consider renaming `categoryId` to `category_id`",
        }
      ],
      // r#"import * as no_camelcased from "external-module";"#: [
      //   {
      //     col: 12,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"import * as camelCased from "external-module";"#: [// new
        {
          col: 12,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"import { no_camelcased as no_camel_cased } from "external module";"#: [
      //   {
      //     col: 26,
      //     message: "Identifier 'no_camel_cased' is not in rust style.",
      //     hint: "Consider renaming `no_camel_cased` to `noCamelCased`",
      //   }
      // ],
      r#"import { no_camelcased as camelCased } from "external module";"#: [// new
        {
          col: 26,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"import { camelCased as no_camel_cased } from "external module";"#: [
      //   {
      //     col: 23,
      //     message: "Identifier 'no_camel_cased' is not in rust style.",
      //     hint: "Consider renaming `no_camel_cased` to `noCamelCased`",
      //   }
      // ],
      r#"import { camelCased as camelCased } from "external module";"#: [// new
        {
          col: 23,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"import no_camelcased, { another_no_camelcased as camelCased } from "external-module";"#: [
      //   {
      //     col: 7,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"import camelCased, { anotherCamelCased as no_camelcased } from "external-module";"#: [// new
        {
          col: 7,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"import no_camelcased, { another_no_camelcased as camelCased } from "external-module";"#: [// new
        {
          col: 49,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"import snake_cased from 'mod'"#: [
      //   {
      //     col: 7,
      //     message: "Identifier 'snake_cased' is not in rust style.",
      //     hint: "Consider renaming `snake_cased` to `snakeCased`",
      //   }
      // ],
      r#"import camelCased from 'mod'"#: [// new
        {
          col: 7,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"import * as snake_cased from 'mod'"#: [
      //   {
      //     col: 12,
      //     message: "Identifier 'snake_cased' is not in rust style.",
      //     hint: "Consider renaming `snake_cased` to `snakeCased`",
      //   }
      // ],
      r#"import * as camelCased from 'mod'"#: [// new
        {
          col: 12,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"export * as snake_cased from 'mod'"#: [
      //   {
      //     col: 12,
      //     message: "Identifier 'snake_cased' is not in rust style.",
      //     hint: "Consider renaming `snake_cased` to `snakeCased`",
      //   }
      // ],
      r#"export * as camelCased from 'mod'"#: [// new
        {
          col: 12,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"function foo({ no_camelcased }) {};"#: [
      //   {
      //     col: 15,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"function foo({ camelCased }) {};"#: [// new
        {
          col: 15,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"function foo({ no_camelcased = 'default value' }) {};"#: [
      //   {
      //     col: 15,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"function foo({ camelCased = 'default value' }) {};"#: [// new
        {
          col: 15,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"const no_camelcased = 0; function foo({ camelcased_value = no_camelcased }) {}"#: [
      //   {
      //     col: 6,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   },
      //   {
      //     col: 40,
      //     message: "Identifier 'camelcased_value' is not in rust style.",
      //     hint: "Consider renaming `camelcased_value` to `camelcasedValue`",
      //   }
      // ],
      r#"const camelCased = 0; function foo({ camelCased_value = no_camelcased }) {}"#: [// new
        {
          col: 6,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        },
        {
          col: 37,
          message: "Identifier 'camelCased_value' is not in rust style.",
          hint: "Consider renaming `camelCased_value` to `camel_cased_value`",
        }
      ],
      // r#"const { bar: no_camelcased } = foo;"#: [
      //   {
      //     col: 13,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"const { bar: camelCased } = foo;"#: [// new
        {
          col: 13,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"function foo({ value_1: my_default }) {}"#: [
      //   {
      //     col: 24,
      //     message: "Identifier 'my_default' is not in rust style.",
      //     hint: "Consider renaming `my_default` to `myDefault`",
      //   }
      // ],
      r#"function foo({ value_1: myDefault }) {}"#: [// new
        {
          col: 24,
          message: "Identifier 'myDefault' is not in rust style.",
          hint: "Consider renaming `myDefault` to `my_default`",
        }
      ],
      // r#"function foo({ isCamelcased: no_camelcased }) {};"#: [
      //   {
      //     col: 29,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"function foo({ is_not_camelcased: camelCased }) {};"#: [// new
        {
          col: 34,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"function foo({ isCamelcased: { no_camelcased } }) {};"#: [
      //   {
      //     col: 31,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"function foo({ is_not_camelcased: { camelCased } }) {};"#: [// new
        {
          col: 36,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      // r#"var { foo: bar_baz = 1 } = quz;"#: [
      //   {
      //     col: 11,
      //     message: "Identifier 'bar_baz' is not in rust style.",
      //     hint: "Consider renaming `bar_baz` to `barBaz`",
      //   }
      // ],
      r#"var { foo: barBaz = 1 } = quz;"#: [// new
        {
          col: 11,
          message: "Identifier 'barBaz' is not in rust style.",
          hint: "Consider renaming `barBaz` to `bar_baz`",
        }
      ],
      // r#"const f = function no_camelcased() {};"#: [
      //   {
      //     col: 19,
      //     message: "Identifier 'no_camelcased' is not in rust style.",
      //     hint: "Consider renaming `no_camelcased` to `noCamelcased`",
      //   }
      // ],
      r#"const f = function camelCased() {};"#: [// new
        {
          col: 19,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"const c = class no_camelcased {};"#: [
        {
          col: 16,
          message: "Identifier 'no_camelcased' is not in rust style.",
          hint: "Consider renaming `no_camelcased` to `NoCamelcased`",
        }
      ],
      r#"class no_camelcased {}"#: [
        {
          col: 6,
          message: "Identifier 'no_camelcased' is not in rust style.",
          hint: "Consider renaming `no_camelcased` to `NoCamelcased`",
        }
      ],
      r#"type foo_bar = string;"#: [
        {
          col: 5,
          message: "Identifier 'foo_bar' is not in rust style.",
          hint: "Consider renaming `foo_bar` to `FooBar`",
        }
      ],
      // r#"type Foo = { snake_case: number; };"#: [
      //   {
      //     col: 13,
      //     message: "Identifier 'snake_case' is not in rust style.",
      //     hint: r#"Consider renaming `snake_case` to `snakeCase`, or wrapping it in quotation mark like `"snake_case"`"#,
      //   }
      // ],
      r#"type Foo = { camelCase: number; };"#: [// new
        {
          col: 13,
          message: "Identifier 'camelCase' is not in rust style.",
          hint: r#"Consider renaming `camelCase` to `camel_case`, or wrapping it in quotation mark like `"camelCase"`"#,
        }
      ],
      r#"interface foo_bar { ok: string; };"#: [
        {
          col: 10,
          message: "Identifier 'foo_bar' is not in rust style.",
          hint: "Consider renaming `foo_bar` to `FooBar`",
        }
      ],
      // r#"interface Foo { snake_case: number; };"#: [
      //   {
      //     col: 16,
      //     message: "Identifier 'snake_case' is not in rust style.",
      //     hint: r#"Consider renaming `snake_case` to `snakeCase`, or wrapping it in quotation mark like `"snake_case"`"#,
      //   }
      // ],
      r#"interface Foo { camelCase: number; };"#: [// new
        {
          col: 16,
          message: "Identifier 'camelCase' is not in rust style.",
          hint: r#"Consider renaming `camelCase` to `camel_case`, or wrapping it in quotation mark like `"camelCase"`"#,
        }
      ],
      r#"namespace foo_bar {}"#: [
        {
          col: 10,
          message: "Identifier 'foo_bar' is not in rust style.",
          hint: "Consider renaming `foo_bar` to `FooBar`",
        }
      ],
      // r#"namespace FooBar { const snake_case = 42; }"#: [
      //   {
      //     col: 25,
      //     message: "Identifier 'snake_case' is not in rust style.",
      //     hint: "Consider renaming `snake_case` to `snakeCase`",
      //   }
      // ],
      r#"namespace FooBar { const camelCase = 42; }"#: [// new
        {
          col: 25,
          message: "Identifier 'camelCase' is not in rust style.",
          hint: "Consider renaming `camelCase` to `camel_case`",
        }
      ],
      r#"enum foo_bar { VariantOne }"#: [
        {
          col: 5,
          message: "Identifier 'foo_bar' is not in rust style.",
          hint: "Consider renaming `foo_bar` to `FooBar`",
        }
      ],
      r#"enum FooBar { variant_one }"#: [
        {
          col: 14,
          message: "Identifier 'variant_one' is not in rust style.",
          hint: "Consider renaming `variant_one` to `VariantOne`",
        }
      ],
      //new invalid test cases:
      r#"firstName = "Ichigo""#: [
        {
          col: 0,

          message: "Identifier 'firstName' is not in rust style.",
          hint: "Consider renaming `firstName` to `first_name`",
        }
      ],
      r#"__myPrivateVariable = "Hoshimiya""#: [
        {
          col: 0,
          message: "Identifier '__myPrivateVariable' is not in rust style.",
          hint: "Consider renaming `__myPrivateVariable` to `__my_private_variable`",
        }
      ],
      r#"myPrivateVariable_ = "Hoshimiya""#: [
        {
          col: 0,
          message: "Identifier 'myPrivateVariable_' is not in rust style.",
          hint: "Consider renaming `myPrivateVariable_` to `my_private_variable_`",
        }
      ],
      r#"function doSomething(){}"#: [
        {
          col: 9,
          message: "Identifier 'doSomething' is not in rust style.",
          hint: "Consider renaming `doSomething` to `do_something`",
        }
      ],
      r#"import { no_camelcased as camelCased } from "external-module";"#: [
        {
          col: 26,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"import { no_camelcased as camelCased, anotherCamelCased } from "external-module";"#: [
        {
          col: 26,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"var _camelCased = aGlobalVariable"#: [
        {
          col: 4,
          message: "Identifier '_camelCased' is not in rust style.",
          hint: "Consider renaming `_camelCased` to `_camel_cased`",
        }
      ],
      r#"var camelCased = _aGlobalVariable"#: [
        {
          col: 4,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"function foo({ no_camelcased: camelCased }) {};"#: [
        {
          col: 30,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"function foo({ camelCased = 'default value' }) {};"#: [
        {
          col: 15,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"function foo({ camelCased }) {};"#: [
        {
          col: 15,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"const f = function camelCased() {};"#: [
        {
          col: 19,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
      r#"const c = class camelCased {};"#: [
        {
          col: 16,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `CamelCased`",
        }
      ],
      r#"const c = class snake_cased {};"#: [
        {
          col: 16,
          message: "Identifier 'snake_cased' is not in rust style.",
          hint: "Consider renaming `snake_cased` to `SnakeCased`",
        }
      ],
      r#"class snake_cased {};"#:[
        {
          col: 6,
          message: "Identifier 'snake_cased' is not in rust style.",
          hint: "Consider renaming `snake_cased` to `SnakeCased`",
        }
      ],
      r#"class camelCased {};"#:[
        {
          col: 6,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `CamelCased`",
        }
      ],
      r#"var { [category_id]: categoryId } = query;"#: [
        {
          col: 21,
          message: "Identifier 'categoryId' is not in rust style.",
          hint: "Consider renaming `categoryId` to `category_id`",
        }
      ],
      r#"var camelCased = snake_cased"#: [
        {
          col: 4,
          message: "Identifier 'camelCased' is not in rust style.",
          hint: "Consider renaming `camelCased` to `camel_cased`",
        }
      ],
    };
  }
}
