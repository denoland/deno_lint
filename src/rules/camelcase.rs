// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::Key;
use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use std::collections::{BTreeMap, BTreeSet};
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{
  ArrayPat, AssignPat, AssignPatProp, ClassDecl, ClassExpr,
  ExportNamespaceSpecifier, Expr, FnDecl, FnExpr, GetterProp, Ident,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportStarAsSpecifier,
  KeyValuePatProp, KeyValueProp, MethodProp, ObjectLit, ObjectPat,
  ObjectPatProp, Param, Pat, Program, Prop, PropName, PropOrSpread, RestPat,
  SetterProp, VarDeclarator,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

pub struct Camelcase;

impl LintRule for Camelcase {
  fn new() -> Box<Self> {
    Box::new(Camelcase)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "camelcase"
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = CamelcaseVisitor::new(context);
    visitor.visit_program(program, program);
    visitor.report_errors();
  }

  fn docs(&self) -> &'static str {
    r#"Enforces the use of camelCase in variable names

Consistency in a code base is key for readability and maintainability.  This rule
enforces variable declarations and object property names which you create to be
in camelCase.  Of note:
* `_` is allowed at the start or end of a variable
* All uppercase variable names (e.g. constants) may have `_` in their name
* If you have to use a snake_case key in an object for some reasons, wrap it in quatations
* This rule also applies to variables imported or exported via ES modules, but not to object properties of those variables
    
### Invalid:
```typescript
let first_name = "Ichigo";
const obj = { last_name: "Hoshimiya" };
const { last_name } = obj;

function do_something(){}
function foo({ snake_case = "default value" }) {}

class snake_case_class {}
class Also_Not_Valid_Class {}

import { not_camelCased } from "external-module.js";
export * as not_camelCased from "mod.ts";
```

### Valid:
```typescript
let firstName = "Ichigo";
const FIRST_NAME = "Ichigo";
const __myPrivateVariable = "Hoshimiya";
const myPrivateVariable_ = "Hoshimiya";
const obj = { "last_name": "Hoshimiya" }; // if an object key is wrapped in quotations, then it's valid
const { last_name: lastName } = obj;

function doSomething(){} // function declarations must be camelCase but...
do_something();  // ...snake_case function calls are allowed
function foo({ snake_case: camelCase = "default value" }) {}

class PascalCaseClass {}

import { not_camelCased as camelCased } from "external-module.js";
export * as camelCased from "mod.ts";
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
        static FIRST_CHAR_LOWERCASE: Lazy<Regex> =
          Lazy::new(|| Regex::new(r"^[a-z]").unwrap());

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
    r#"var o = {bar_baz: 1}"#: [
            {
              col: 9,
              message: "Identifier 'bar_baz' is not in camel case.",
              hint: r#"Consider renaming `bar_baz` to `barBaz`, or wrapping it in quotation mark like `"bar_baz"`"#,
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
              hint: "Consider replacing `{ category_id }` with `{ category_id: categoryId }`",
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
              hint: "Consider replacing `{ no_camelcased }` with `{ no_camelcased: noCamelcased }`",
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
              hint: "Consider replacing `{ camelcased_value }` with `{ camelcased_value: camelcasedValue }`",
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
              hint: "Consider replacing `{ no_camelcased }` with `{ no_camelcased: noCamelcased }`",
            }
          ],
    r#"const { no_camelcased = foo_bar } = bar;"#: [
            {
              col: 8,
              message: "Identifier 'no_camelcased' is not in camel case.",
              hint: "Consider replacing `{ no_camelcased }` with `{ no_camelcased: noCamelcased }`",
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
          ]
    };
  }
}
