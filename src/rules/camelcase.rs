// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::Key;
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{
  ArrayPat, AssignPat, AssignPatProp, ClassDecl, ClassExpr,
  ExportNamespaceSpecifier, Expr, FnDecl, FnExpr, GetterProp, Ident,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportStarAsSpecifier,
  KeyValuePatProp, KeyValueProp, MethodProp, Module, ObjectLit, ObjectPat,
  ObjectPatProp, Param, Pat, Prop, PropName, PropOrSpread, RestPat, SetterProp,
  VarDeclarator,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

pub struct Camelcase;

impl LintRule for Camelcase {
  fn new() -> Box<Self> {
    Box::new(Camelcase)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "camelcase"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = CamelcaseVisitor::new(context);
    visitor.visit_module(module, module);
    visitor.report_errors();
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

  lazy_static! {
    static ref UNDERSCORE_CHAR_RE: Regex =
      Regex::new(r"([^_])_([a-z])").unwrap();
  }

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
  /// Function name e.g. `foo` in `function foo() {}`
  Function(String),
  /// Class name e.g. `Foo` in `class Foo {}`
  Class(String),
  /// Key and value name in object pattern, for example:
  ///
  /// ```typescript
  /// const { foo } = obj1; // key_name: foo, value_name: None
  ///
  /// const { foo: bar } = obj2; // key_name: foo, value_name: Some(bar)
  ///
  /// const { foo: bar = default_value } = obj3; // key_name: foo, value_name: Some(bar)
  ///
  /// function f({ foo }) {} // key_name: foo, value_name: None
  /// ```
  ObjectPat {
    key_name: String,
    value_name: Option<String>,
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

  fn function(name: impl AsRef<str>) -> Self {
    Self::Function(name.as_ref().to_string())
  }

  fn class(name: impl AsRef<str>) -> Self {
    Self::Class(name.as_ref().to_string())
  }

  fn object_pat<K, V>(key_name: &K, value_name: Option<&V>) -> Self
  where
    K: AsRef<str>,
    V: AsRef<str>,
  {
    Self::ObjectPat {
      key_name: key_name.as_ref().to_string(),
      value_name: value_name.map(|v| v.as_ref().to_string()),
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
      | IdentToCheck::Class(name) => name,
      IdentToCheck::ObjectPat {
        key_name,
        value_name,
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
      IdentToCheck::Class(name) => {
        let camel_cased = to_camelcase(name);
        lazy_static! {
          static ref FIRST_CHAR_LOWERCASE: Regex =
            Regex::new(r"^[a-z]").unwrap();
        }
        // Class name should be in pascal case
        let pascal_cased = FIRST_CHAR_LOWERCASE
          .replace(&camel_cased, |caps: &Captures| {
            caps[0].to_ascii_uppercase()
          });
        format!("Consider renaming `{}` to `{}`", name, pascal_cased)
      }
      IdentToCheck::ObjectPat {
        key_name,
        value_name,
      } => {
        if let Some(value_name) = value_name {
          format!(
            "Consider renaming `{}` to `{}`",
            value_name,
            to_camelcase(value_name),
          )
        } else {
          format!(
            "Consider replacing `{{ {key} }}` with `{{ {key}: {value} }}`",
            key = key_name,
            value = to_camelcase(key_name),
          )
        }
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

struct CamelcaseVisitor<'c> {
  context: &'c mut Context,
  errors: BTreeMap<Span, IdentToCheck>,
  /// Already visited identifiers
  visited: BTreeSet<Span>,
}

impl<'c> CamelcaseVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self {
      context,
      errors: BTreeMap::new(),
      visited: BTreeSet::new(),
    }
  }

  /// Report accumulated errors
  fn report_errors(&mut self) {
    for (span, error_ident) in &self.errors {
      self.context.add_diagnostic_with_hint(
        *span,
        "camelcase",
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

  fn check_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(ident) => {
        self.check_ident(ident, IdentToCheck::variable(ident));
      }
      Pat::Array(ArrayPat { ref elems, .. }) => {
        for elem in elems {
          if let Some(pat) = elem {
            self.check_pat(pat);
          }
        }
      }
      Pat::Rest(RestPat { ref arg, .. }) => {
        self.check_pat(&**arg);
      }
      Pat::Object(ObjectPat { ref props, .. }) => {
        for prop in props {
          match prop {
            ObjectPatProp::KeyValue(KeyValuePatProp { ref key, ref value }) => {
              if let Pat::Ident(value_ident) = &**value {
                self.check_ident(
                  value_ident,
                  IdentToCheck::object_pat(
                    &key.get_key().unwrap_or_else(|| "[KEY]".to_string()),
                    Some(value_ident),
                  ),
                );
              } else {
                self.check_pat(&**value);
              }
            }
            ObjectPatProp::Assign(AssignPatProp { ref key, .. }) => {
              self.check_ident(
                key,
                IdentToCheck::object_pat::<Ident, Ident>(key, None),
              );
            }
            ObjectPatProp::Rest(RestPat { ref arg, .. }) => {
              self.check_pat(&**arg);
            }
          }
        }
      }
      Pat::Assign(AssignPat { ref left, .. }) => {
        self.check_pat(&**left);
      }
      Pat::Expr(expr) => {
        if let Expr::Ident(ident) = &**expr {
          self.check_ident(ident, IdentToCheck::variable(ident));
        }
      }
      Pat::Invalid(_) => {}
    }
  }
}

impl<'c> Visit for CamelcaseVisitor<'c> {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _: &dyn Node) {
    self.check_ident(&fn_decl.ident, IdentToCheck::function(&fn_decl.ident));
    fn_decl.visit_children_with(self);
  }

  fn visit_class_decl(&mut self, class_decl: &ClassDecl, _: &dyn Node) {
    self.check_ident(&class_decl.ident, IdentToCheck::class(&class_decl.ident));
    class_decl.visit_children_with(self);
  }

  fn visit_var_declarator(
    &mut self,
    var_declarator: &VarDeclarator,
    _: &dyn Node,
  ) {
    self.check_pat(&var_declarator.name);

    if let Some(expr) = &var_declarator.init {
      match &**expr {
        Expr::Object(ObjectLit { ref props, .. }) => {
          for prop in props {
            if let PropOrSpread::Prop(prop) = prop {
              match &**prop {
                Prop::Shorthand(ident) => self.check_ident(
                  ident,
                  IdentToCheck::object_pat::<Ident, Ident>(ident, None),
                ),
                Prop::KeyValue(KeyValueProp { ref key, .. }) => {
                  if let PropName::Ident(ident) = key {
                    self.check_ident(ident, IdentToCheck::variable(ident));
                  }
                }
                Prop::Getter(GetterProp { ref key, .. }) => {
                  if let PropName::Ident(ident) = key {
                    self.check_ident(ident, IdentToCheck::function(ident));
                  }
                }
                Prop::Setter(SetterProp { ref key, .. }) => {
                  if let PropName::Ident(ident) = key {
                    self.check_ident(ident, IdentToCheck::function(ident));
                  }
                }
                Prop::Method(MethodProp { ref key, .. }) => {
                  if let PropName::Ident(ident) = key {
                    self.check_ident(ident, IdentToCheck::function(ident));
                  }
                }
                Prop::Assign(_) => {}
              }
            }
          }
        }
        Expr::Fn(FnExpr { ref ident, .. }) => {
          if let Some(ident) = ident {
            self.check_ident(ident, IdentToCheck::function(ident));
          }
        }
        Expr::Class(ClassExpr { ref ident, .. }) => {
          if let Some(ident) = ident {
            self.check_ident(ident, IdentToCheck::class(ident));
          }
        }
        _ => {}
      }
    }

    var_declarator.visit_children_with(self);
  }

  fn visit_param(&mut self, param: &Param, _: &dyn Node) {
    self.check_pat(&param.pat);
    param.visit_children_with(self);
  }

  fn visit_import_named_specifier(
    &mut self,
    import_named_specifier: &ImportNamedSpecifier,
    _: &dyn Node,
  ) {
    let ImportNamedSpecifier {
      local, imported, ..
    } = import_named_specifier;
    self
      .check_ident(local, IdentToCheck::named_import(local, imported.as_ref()));
    import_named_specifier.visit_children_with(self);
  }

  fn visit_import_default_specifier(
    &mut self,
    import_default_specifier: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    let ImportDefaultSpecifier { local, .. } = import_default_specifier;
    self.check_ident(local, IdentToCheck::variable(local));
    import_default_specifier.visit_children_with(self);
  }

  fn visit_import_star_as_specifier(
    &mut self,
    import_star_as_specifier: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    let ImportStarAsSpecifier { local, .. } = import_star_as_specifier;
    self.check_ident(local, IdentToCheck::variable(local));
    import_star_as_specifier.visit_children_with(self);
  }

  fn visit_export_namespace_specifier(
    &mut self,
    export_namespace_specifier: &ExportNamespaceSpecifier,
    _: &dyn Node,
  ) {
    let ExportNamespaceSpecifier { name, .. } = export_namespace_specifier;
    self.check_ident(name, IdentToCheck::variable(name));
    export_namespace_specifier.visit_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
        },
        "Consider replacing `{ foo_bar }` with `{ foo_bar: fooBar }`",
      ),
      (
        IdentToCheck::ObjectPat {
          key_name: s("foo_bar"),
          value_name: Some(s("snake_case")),
        },
        "Consider renaming `snake_case` to `snakeCase`",
      ),
    ];

    for (error_ident, expected) in tests.iter() {
      assert_eq!(*expected, error_ident.to_hint());
    }
  }

  // Based on https://github.com/eslint/eslint/blob/v7.8.1/tests/lib/rules/camelcase.js

  #[test]
  fn camelcase_valid() {
    assert_lint_ok::<Camelcase>(r#"firstName = "Ichigo""#);
    assert_lint_ok::<Camelcase>(r#"FIRST_NAME = "Ichigo""#);
    assert_lint_ok::<Camelcase>(r#"__myPrivateVariable = "Hoshimiya""#);
    assert_lint_ok::<Camelcase>(r#"myPrivateVariable_ = "Hoshimiya""#);
    assert_lint_ok::<Camelcase>(r#"function doSomething(){}"#);
    assert_lint_ok::<Camelcase>(r#"do_something()"#);
    assert_lint_ok::<Camelcase>(r#"new do_something"#);
    assert_lint_ok::<Camelcase>(r#"new do_something()"#);
    assert_lint_ok::<Camelcase>(r#"foo.do_something()"#);
    assert_lint_ok::<Camelcase>(r#"var foo = bar.baz_boom;"#);
    assert_lint_ok::<Camelcase>(r#"var foo = bar.baz_boom.something;"#);
    assert_lint_ok::<Camelcase>(
      r#"foo.boom_pow.qux = bar.baz_boom.something;"#,
    );
    assert_lint_ok::<Camelcase>(r#"if (bar.baz_boom) {}"#);
    assert_lint_ok::<Camelcase>(r#"var obj = { key: foo.bar_baz };"#);
    assert_lint_ok::<Camelcase>(r#"var arr = [foo.bar_baz];"#);
    assert_lint_ok::<Camelcase>(r#"[foo.bar_baz]"#);
    assert_lint_ok::<Camelcase>(r#"var arr = [foo.bar_baz.qux];"#);
    assert_lint_ok::<Camelcase>(r#"[foo.bar_baz.nesting]"#);
    assert_lint_ok::<Camelcase>(
      r#"if (foo.bar_baz === boom.bam_pow) { [foo.baz_boom] }"#,
    );
    assert_lint_ok::<Camelcase>(r#"var o = {key: 1}"#);
    assert_lint_ok::<Camelcase>(r#"var o = {_leading: 1}"#);
    assert_lint_ok::<Camelcase>(r#"var o = {trailing_: 1}"#);
    assert_lint_ok::<Camelcase>(r#"const { ['foo']: _foo } = obj;"#);
    assert_lint_ok::<Camelcase>(r#"const { [_foo_]: foo } = obj;"#);
    assert_lint_ok::<Camelcase>(r#"var { category_id: category } = query;"#);
    assert_lint_ok::<Camelcase>(r#"var { _leading } = query;"#);
    assert_lint_ok::<Camelcase>(r#"var { trailing_ } = query;"#);
    assert_lint_ok::<Camelcase>(
      r#"import { camelCased } from "external module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { _leading } from "external module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { trailing_ } from "external module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as camelCased } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as _leading } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as trailing_ } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as camelCased, anotherCamelCased } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(r#"import { camelCased } from 'mod'"#);
    assert_lint_ok::<Camelcase>(r#"var _camelCased = aGlobalVariable"#);
    assert_lint_ok::<Camelcase>(r#"var camelCased = _aGlobalVariable"#);
    assert_lint_ok::<Camelcase>(
      r#"function foo({ no_camelcased: camelCased }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ no_camelcased: _leading }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ no_camelcased: trailing_ }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ camelCased = 'default value' }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ _leading = 'default value' }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ trailing_ = 'default value' }) {};"#,
    );
    assert_lint_ok::<Camelcase>(r#"function foo({ camelCased }) {};"#);
    assert_lint_ok::<Camelcase>(r#"function foo({ _leading }) {}"#);
    assert_lint_ok::<Camelcase>(r#"function foo({ trailing_ }) {}"#);
    assert_lint_ok::<Camelcase>(r#"({obj} = baz.fo_o);"#);
    assert_lint_ok::<Camelcase>(r#"([obj] = baz.fo_o);"#);
    assert_lint_ok::<Camelcase>(r#"([obj.foo = obj.fo_o] = bar);"#);
    assert_lint_ok::<Camelcase>(r#"const f = function camelCased() {};"#);
    assert_lint_ok::<Camelcase>(r#"const c = class camelCased {};"#);
    assert_lint_ok::<Camelcase>(r#"class camelCased {};"#);

    // The following test cases are _invalid_ in ESLint, but we've decided to treat them as _valid_.
    // See background at https://github.com/denoland/deno_lint/pull/302
    assert_lint_ok::<Camelcase>(r#"first_name = "Akari""#);
    assert_lint_ok::<Camelcase>(r#"__private_first_name = "Akari""#);
    assert_lint_ok::<Camelcase>(r#"obj.foo_bar = function(){};"#);
    assert_lint_ok::<Camelcase>(r#"bar_baz.foo = function(){};"#);
    assert_lint_ok::<Camelcase>(r#"[foo_bar.baz]"#);
    assert_lint_ok::<Camelcase>(
      r#"if (foo.bar_baz === boom.bam_pow) { [foo_bar.baz] }"#,
    );
    assert_lint_ok::<Camelcase>(r#"foo.bar_baz = boom.bam_pow"#);
    assert_lint_ok::<Camelcase>(r#"foo.qux.boom_pow = { bar: boom.bam_pow }"#);
    assert_lint_ok::<Camelcase>(r#"obj.a_b = 2;"#);
    assert_lint_ok::<Camelcase>(
      r#"var { [category_id]: categoryId } = query;"#,
    );
    assert_lint_ok::<Camelcase>(r#"a_global_variable.foo()"#);
    assert_lint_ok::<Camelcase>(r#"a_global_variable[undefined]"#);
    assert_lint_ok::<Camelcase>(r#"var camelCased = snake_cased"#);
    assert_lint_ok::<Camelcase>(r#"({ a: obj.fo_o } = bar);"#);
    assert_lint_ok::<Camelcase>(r#"({ a: obj.fo_o.b_ar } = baz);"#);
    assert_lint_ok::<Camelcase>(r#"({ a: { b: { c: obj.fo_o } } } = bar);"#);
    assert_lint_ok::<Camelcase>(
      r#"({ a: { b: { c: obj.fo_o.b_ar } } } = baz);"#,
    );
    assert_lint_ok::<Camelcase>(r#"([obj.fo_o] = bar);"#);
    assert_lint_ok::<Camelcase>(r#"([obj.fo_o = 1] = bar);"#);
    assert_lint_ok::<Camelcase>(r#"({ a: [obj.fo_o] } = bar);"#);
    assert_lint_ok::<Camelcase>(r#"({ a: { b: [obj.fo_o] } } = bar);"#);
    assert_lint_ok::<Camelcase>(r#"([obj.fo_o.ba_r] = baz);"#);
    assert_lint_ok::<Camelcase>(r#"obj.o_k.non_camelcase = 0"#);
    assert_lint_ok::<Camelcase>(r#"(obj?.o_k).non_camelcase = 0"#);
    assert_lint_ok::<Camelcase>(r#"({...obj.fo_o} = baz);"#);
    assert_lint_ok::<Camelcase>(r#"({...obj.fo_o.ba_r} = baz);"#);
    assert_lint_ok::<Camelcase>(r#"({c: {...obj.fo_o }} = baz);"#);
    assert_lint_ok::<Camelcase>(r#"not_ignored_foo = 0;"#);
  }

  #[test]
  fn camelcase_invalid() {
    assert_lint_err::<Camelcase>(r#"function foo_bar(){}"#, 9);
    assert_lint_err::<Camelcase>(r#"var foo = { bar_baz: boom.bam_pow }"#, 12);
    assert_lint_err::<Camelcase>(r#"var o = {bar_baz: 1}"#, 9);
    assert_lint_err::<Camelcase>(
      r#"var { category_id: category_alias } = query;"#,
      19,
    );
    assert_lint_err::<Camelcase>(r#"var { category_id } = query;"#, 6);

    assert_lint_err::<Camelcase>(
      r#"var { category_id: category_id } = query;"#,
      19,
    );
    assert_lint_err::<Camelcase>(r#"var { category_id = 1 } = query;"#, 6);
    assert_lint_err::<Camelcase>(
      r#"import no_camelcased from "external-module";"#,
      7,
    );
    assert_lint_err::<Camelcase>(
      r#"import * as no_camelcased from "external-module";"#,
      12,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased } from "external-module";"#,
      9,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased as no_camel_cased } from "external module";"#,
      26,
    );
    assert_lint_err::<Camelcase>(
      r#"import { camelCased as no_camel_cased } from "external module";"#,
      23,
    );
    assert_lint_err::<Camelcase>(
      r#"import { camelCased, no_camelcased } from "external-module";"#,
      21,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased as camelCased, another_no_camelcased } from "external-module";"#,
      38,
    );
    assert_lint_err::<Camelcase>(
      r#"import camelCased, { no_camelcased } from "external-module";"#,
      21,
    );
    assert_lint_err::<Camelcase>(
      r#"import no_camelcased, { another_no_camelcased as camelCased } from "external-module";"#,
      7,
    );
    assert_lint_err::<Camelcase>(r#"import snake_cased from 'mod'"#, 7);
    assert_lint_err::<Camelcase>(r#"import * as snake_cased from 'mod'"#, 12);
    assert_lint_err::<Camelcase>(r#"export * as snake_cased from 'mod'"#, 12);
    assert_lint_err::<Camelcase>(r#"function foo({ no_camelcased }) {};"#, 15);
    assert_lint_err::<Camelcase>(
      r#"function foo({ no_camelcased = 'default value' }) {};"#,
      15,
    );
    assert_lint_err_n::<Camelcase>(
      r#"const no_camelcased = 0; function foo({ camelcased_value = no_camelcased }) {}"#,
      vec![6, 40],
    );
    assert_lint_err::<Camelcase>(r#"const { bar: no_camelcased } = foo;"#, 13);
    assert_lint_err::<Camelcase>(
      r#"function foo({ value_1: my_default }) {}"#,
      24,
    );
    assert_lint_err::<Camelcase>(
      r#"function foo({ isCamelcased: no_camelcased }) {};"#,
      29,
    );
    assert_lint_err::<Camelcase>(r#"var { foo: bar_baz = 1 } = quz;"#, 11);
    assert_lint_err::<Camelcase>(
      r#"const { no_camelcased = false } = bar;"#,
      8,
    );
    assert_lint_err::<Camelcase>(
      r#"const { no_camelcased = foo_bar } = bar;"#,
      8,
    );
    assert_lint_err::<Camelcase>(
      r#"const f = function no_camelcased() {};"#,
      19,
    );
    assert_lint_err::<Camelcase>(r#"const c = class no_camelcased {};"#, 16);
    assert_lint_err::<Camelcase>(r#"class no_camelcased {}"#, 6);
  }
}
