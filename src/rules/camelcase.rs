// Copyright 2020 the Deno authors. All rights reserved. MIT license.
// TODO(magurotuna): remove next line
#![allow(unused)]
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{
  ArrayPat, AssignExpr, AssignPat, AssignPatProp, Expr, ExprOrSuper, FnDecl,
  FnExpr, Ident, KeyValuePatProp, MemberExpr, ObjectPat, ObjectPatProp, Pat,
  PatOrExpr, PropName, RestPat, VarDecl,
};
use swc_ecmascript::visit::{self, noop_visit_type, Node, Visit};

use std::sync::Arc;

pub struct Camelcase;

impl LintRule for Camelcase {
  fn new() -> Box<Self> {
    Box::new(Camelcase)
  }

  fn code(&self) -> &'static str {
    "camelcase"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = CamelcaseVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct CamelcaseVisitor {
  context: Arc<Context>,
}

/// Check if it contains underscores, except for leading and trailing ones
fn is_underscored(ident: &Ident) -> bool {
  let trimmed_ident = ident.as_ref().trim_matches('_');
  trimmed_ident.contains('_')
    && trimmed_ident != trimmed_ident.to_ascii_uppercase()
}

impl CamelcaseVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn report(&self, ident: &Ident) {
    self.context.add_diagnostic(
      ident.span,
      "camelcase",
      &format!("Identifier '{}' is not in camel case.", ident.as_ref()),
    );
  }
}

fn check_pat(pat: &Pat) -> Vec<Ident> {
  fn inner_check_pat(pat: &Pat, errors: &mut Vec<Ident>) {
    fn check_ident(ident: &Ident, errors: &mut Vec<Ident>) {
      if is_underscored(ident) {
        errors.push(ident.clone());
      }
    }
    match pat {
      Pat::Ident(ref ident) => {
        check_ident(ident, errors);
      }
      Pat::Array(ArrayPat { ref elems, .. }) => {
        for elem in elems {
          if let Some(pat) = elem {
            inner_check_pat(pat, errors);
          }
        }
      }
      Pat::Rest(RestPat { ref arg, .. }) => {
        inner_check_pat(&**arg, errors);
      }
      Pat::Object(ObjectPat { ref props, .. }) => {
        for prop in props {
          match prop {
            ObjectPatProp::KeyValue(KeyValuePatProp { ref key, .. }) => {
              match key {
                PropName::Ident(ref ident) => {
                  check_ident(ident, errors);
                }
                _ => {}
              }
            }
            ObjectPatProp::Assign(AssignPatProp { ref key, .. }) => {
              check_ident(key, errors);
            }
            ObjectPatProp::Rest(RestPat { ref arg, .. }) => {
              inner_check_pat(&**arg, errors);
            }
          }
        }
      }
      Pat::Assign(AssignPat { ref left, .. }) => {
        inner_check_pat(&**left, errors);
      }
      Pat::Expr(ref expr) => {
        errors.extend(check_expr(&**expr));
      }
      Pat::Invalid(_) => {}
    }
  }

  let mut errors = Vec::new();
  inner_check_pat(pat, &mut errors);
  errors
}

fn check_expr(expr: &Expr) -> Vec<Ident> {
  fn inner_check_expr(expr: &Expr, errors: &mut Vec<Ident>) {
    match expr {
      Expr::Member(MemberExpr { ref prop, .. }) => {
        inner_check_expr(&**prop, errors);
      }
      Expr::Ident(ref ident) => {
        if is_underscored(ident) {
          errors.push(ident.clone());
        }
      }
      _ => {}
    }
  }
  let mut errors = Vec::new();
  inner_check_expr(expr, &mut errors);
  errors
}

impl Visit for CamelcaseVisitor {
  noop_visit_type!();

  fn visit_var_decl(&mut self, var_decl: &VarDecl, parent: &dyn Node) {
    var_decl
      .decls
      .iter()
      .map(|decl| check_pat(&decl.name))
      .flatten()
      .for_each(|ident| {
        self.report(&ident);
      });

    visit::visit_var_decl(self, var_decl, parent);
  }

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, parent: &dyn Node) {
    let errors = match assign_expr.left {
      PatOrExpr::Expr(ref expr) => check_expr(&**expr),
      PatOrExpr::Pat(ref pat) => check_pat(&**pat),
    };
    for ident in errors {
      self.report(&ident);
    }

    visit::visit_assign_expr(self, assign_expr, parent);
  }

  fn visit_fn_decl(&mut self, fn_decl: &FnDecl, parent: &dyn Node) {
    let ident = &fn_decl.ident;
    if is_underscored(ident) {
      self.report(ident);
    }
    visit::visit_fn_decl(self, fn_decl, parent);
  }

  fn visit_fn_expr(&mut self, fn_expr: &FnExpr, parent: &dyn Node) {
    if let Some(ref ident) = fn_expr.ident {
      if is_underscored(ident) {
        self.report(ident);
      }
    }
    visit::visit_fn_expr(self, fn_expr, parent);
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
    assert_lint_ok::<Camelcase>(r#"var o = {bar_baz: 1}"#); // TODO(magurotuna): isPropertyNameInObjectLiteral
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
  }

  #[test]
  fn camelcase_invalid() {
    assert_lint_err::<Camelcase>(r#"first_name = "Akari""#, 0);
    assert_lint_err::<Camelcase>(r#"__private_first_name = "Akari""#, 0);
    assert_lint_err::<Camelcase>(r#"function foo_bar(){}"#, 0);
    assert_lint_err::<Camelcase>(r#"obj.foo_bar = function(){};"#, 0);
    assert_lint_err::<Camelcase>(r#"bar_baz.foo = function(){};"#, 0);
    assert_lint_err::<Camelcase>(r#"[foo_bar.baz]"#, 0);
    assert_lint_err::<Camelcase>(
      r#"if (foo.bar_baz === boom.bam_pow) { [foo_bar.baz] }"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"foo.bar_baz = boom.bam_pow"#, 0);
    assert_lint_err::<Camelcase>(r#"var foo = { bar_baz: boom.bam_pow }"#, 0);
    assert_lint_err::<Camelcase>(
      r#"foo.qux.boom_pow = { bar: boom.bam_pow }"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var o = {bar_baz: 1}"#, 0); // TODO(magurotuna): isPropertyNameInObjectLiteral
    assert_lint_err::<Camelcase>(r#"obj.a_b = 2;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"var { category_id: category_alias } = query;"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"var { [category_id]: categoryId } = query;"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var { category_id } = query;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"var { category_id: category_id } = query;"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var { category_id = 1 } = query;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"import no_camelcased from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import * as no_camelcased from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased as no_camel_cased } from "external module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { camelCased as no_camel_cased } from "external module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { camelCased, no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased as camelCased, another_no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import camelCased, { no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import no_camelcased, { another_no_camelcased as camelCased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"import snake_cased from 'mod'"#, 0);
    assert_lint_err::<Camelcase>(r#"import * as snake_cased from 'mod'"#, 0);
    assert_lint_err::<Camelcase>(r#"a_global_variable.foo()"#, 0);
    assert_lint_err::<Camelcase>(r#"a_global_variable[undefined]"#, 0);
    assert_lint_err::<Camelcase>(r#"var camelCased = snake_cased"#, 0);
    assert_lint_err::<Camelcase>(r#"export * as snake_cased from 'mod'"#, 0);
    assert_lint_err::<Camelcase>(r#"function foo({ no_camelcased }) {};"#, 0);
    assert_lint_err::<Camelcase>(
      r#"function foo({ no_camelcased = 'default value' }) {};"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"const no_camelcased = 0; function foo({ camelcased_value = no_camelcased}) {}"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"const { bar: no_camelcased } = foo;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"function foo({ value_1: my_default }) {}"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"function foo({ isCamelcased: no_camelcased }) {};"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var { foo: bar_baz = 1 } = quz;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"const { no_camelcased = false } = bar;"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"const { no_camelcased = foo_bar } = bar;"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"not_ignored_foo = 0;"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: obj.fo_o } = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: obj.fo_o.b_ar } = baz);"#, 0);
    assert_lint_err::<Camelcase>(
      r#"({ a: { b: { c: obj.fo_o } } } = bar);"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"({ a: { b: { c: obj.fo_o.b_ar } } } = baz);"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"([obj.fo_o] = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"([obj.fo_o = 1] = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: [obj.fo_o] } = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: { b: [obj.fo_o] } } = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"([obj.fo_o.ba_r] = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"({...obj.fo_o} = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"({...obj.fo_o.ba_r} = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"({c: {...obj.fo_o }} = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"obj.o_k.non_camelcase = 0"#, 0);
    assert_lint_err::<Camelcase>(r#"(obj?.o_k).non_camelcase = 0"#, 0);
  }

  // TODO(magurotuna): remove this
  #[test]
  fn hogepiyo() {
    //assert_lint_ok::<Camelcase>(r#"const [a, ...b] = foo;"#);
    //assert_lint_err::<Camelcase>(r#"[a.fo_o.z] = b;"#, 3);
    assert_lint_ok::<Camelcase>(r#"foo.qu_x.boompow = { bar: boom.bam_pow }"#);
    assert_lint_ok::<Camelcase>(r#"var { category_id: category } = query;"#);
  }
}
