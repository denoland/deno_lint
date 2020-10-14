// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::collections::{BTreeMap, BTreeSet};
use swc_common::Span;
use swc_ecmascript::ast::{
  ArrayPat, AssignPat, AssignPatProp, ClassDecl, ClassExpr,
  ExportNamespaceSpecifier, Expr, FnDecl, FnExpr, GetterProp, Ident,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportStarAsSpecifier,
  KeyValuePatProp, KeyValueProp, MethodProp, ObjectLit, ObjectPat,
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

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = CamelcaseVisitor::new(context);
    visitor.visit_module(module, module);
    visitor.report_errors();
  }
}

/// Check if it contains underscores, except for leading and trailing ones
fn is_underscored(ident: &Ident) -> bool {
  let trimmed_ident = ident.as_ref().trim_matches('_');
  trimmed_ident.contains('_')
    && trimmed_ident != trimmed_ident.to_ascii_uppercase()
}

struct CamelcaseVisitor<'c> {
  context: &'c mut Context,
  errors: BTreeMap<Span, String>,
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
    for (span, ident_name) in &self.errors {
      self.context.add_diagnostic(
        *span,
        "camelcase",
        &format!("Identifier '{}' is not in camel case.", ident_name),
      );
    }
  }

  /// Check if this ident is underscored only when it's not yet visited.
  fn check_ident(&mut self, ident: &Ident) {
    if self.visited.insert(ident.span) && is_underscored(ident) {
      self.errors.insert(ident.span, ident.as_ref().to_string());
    }
  }

  fn check_prop_name(&mut self, prop_name: &PropName) {
    if let PropName::Ident(ident) = prop_name {
      self.check_ident(ident);
    }
  }

  fn check_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(ident) => self.check_ident(ident),
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
            ObjectPatProp::KeyValue(KeyValuePatProp { ref value, .. }) => {
              self.check_pat(&**value);
            }
            ObjectPatProp::Assign(AssignPatProp { ref key, .. }) => {
              self.check_ident(key);
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
      Pat::Expr(expr) => match &**expr {
        Expr::Ident(ident) => self.check_ident(ident),
        _ => {}
      },
      Pat::Invalid(_) => {}
    }
  }
}

impl<'c> Visit for CamelcaseVisitor<'c> {
  noop_visit_type!();

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _: &dyn Node) {
    self.check_ident(&fn_decl.ident);
    fn_decl.visit_children_with(self);
  }

  fn visit_class_decl(&mut self, class_decl: &ClassDecl, _: &dyn Node) {
    self.check_ident(&class_decl.ident);
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
                Prop::Shorthand(ident) => self.check_ident(ident),
                Prop::KeyValue(KeyValueProp { ref key, .. }) => {
                  self.check_prop_name(key);
                }
                Prop::Getter(GetterProp { ref key, .. }) => {
                  self.check_prop_name(key);
                }
                Prop::Setter(SetterProp { ref key, .. }) => {
                  self.check_prop_name(key);
                }
                Prop::Method(MethodProp { ref key, .. }) => {
                  self.check_prop_name(key);
                }
                Prop::Assign(_) => {}
              }
            }
          }
        }
        Expr::Fn(FnExpr { ref ident, .. }) => {
          if let Some(ident) = ident {
            self.check_ident(ident);
          }
        }
        Expr::Class(ClassExpr { ref ident, .. }) => {
          if let Some(ident) = ident {
            self.check_ident(ident);
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
    self.check_ident(&import_named_specifier.local);
    import_named_specifier.visit_children_with(self);
  }

  fn visit_import_default_specifier(
    &mut self,
    import_default_specifier: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    self.check_ident(&import_default_specifier.local);
    import_default_specifier.visit_children_with(self);
  }

  fn visit_import_star_as_specifier(
    &mut self,
    import_star_as_specifier: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    self.check_ident(&import_star_as_specifier.local);
    import_star_as_specifier.visit_children_with(self);
  }

  fn visit_export_namespace_specifier(
    &mut self,
    export_namespace_specifier: &ExportNamespaceSpecifier,
    _: &dyn Node,
  ) {
    self.check_ident(&export_namespace_specifier.name);
    export_namespace_specifier.visit_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
