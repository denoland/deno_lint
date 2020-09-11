// Copyright 2020 the Deno authors. All rights reserved. MIT license.
// TODO(magurotuna): remove next line
#![allow(unused)]
use super::Context;
use super::LintRule;
use std::collections::{BTreeMap, BTreeSet};
use swc_common::{Span, Spanned};
use swc_ecmascript::ast::{
  ArrayLit, ArrayPat, AssignExpr, AssignPat, AssignPatProp, CallExpr,
  ComputedPropName, Expr, ExprOrSpread, ExprOrSuper, FnDecl, FnExpr, Ident,
  ImportDefaultSpecifier, ImportNamedSpecifier, ImportSpecifier,
  ImportStarAsSpecifier, KeyValuePatProp, KeyValueProp, MemberExpr, Module,
  NewExpr, ObjectPat, ObjectPatProp, Pat, PatOrExpr, Prop, PropName, RestPat,
  VarDecl,
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
    visitor.report_errors();
  }
}

/// Check if it contains underscores, except for leading and trailing ones
fn is_underscored(ident: &Ident) -> bool {
  let trimmed_ident = ident.as_ref().trim_matches('_');
  trimmed_ident.contains('_')
    && trimmed_ident != trimmed_ident.to_ascii_uppercase()
}

struct CamelcaseVisitor {
  context: Arc<Context>,
  errors: BTreeMap<Span, String>,
  // Already checked identifiers
  checked: BTreeSet<Span>,
}
impl CamelcaseVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self {
      context,
      errors: BTreeMap::new(),
      checked: BTreeSet::new(),
    }
  }

  fn report_errors(&self) {
    for (span, ident_name) in &self.errors {
      self.context.add_diagnostic(
        *span,
        "camelcase",
        &format!("Identifier '{}' is not in camel case.", ident_name),
      );
    }
  }

  fn check_and_insert(&mut self, ident: &Ident) {
    if self.checked.insert(ident.span) {
      if is_underscored(ident) {
        self.errors.insert(ident.span, ident.as_ref().to_string());
      }
    }
  }

  fn check_ident_in_member_expr(&mut self, member_expr: &MemberExpr) {
    let MemberExpr {
      ref obj, ref prop, ..
    } = member_expr;
    if let ExprOrSuper::Expr(ref expr) = obj {
      match &**expr {
        Expr::Member(ref m) => {
          self.check_ident_in_member_expr(m);
        }
        Expr::Ident(ref ident) => {
          self.check_and_insert(ident);
        }
        _ => {}
      }
    }

    if let Expr::Ident(ref ident) = &**prop {
      self.check_and_insert(ident);
    }
  }

  fn mark_member_ident_in_expr(&mut self, expr: &Expr) {
    match expr {
      Expr::Member(MemberExpr {
        ref obj, ref prop, ..
      }) => {
        if let ExprOrSuper::Expr(ref expr) = obj {
          self.mark_member_ident_in_expr(expr);
        }
        self.mark_member_ident_in_expr(&**prop);
      }
      Expr::Ident(ref ident) => {
        self.checked.insert(ident.span);
      }
      _ => {}
    }
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
            ObjectPatProp::KeyValue(KeyValuePatProp { ref value, .. }) => {
              inner_check_pat(&**value, errors);
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
  let mut errors = Vec::new();

  match expr {
    Expr::Member(MemberExpr {
      ref obj, ref prop, ..
    }) => {
      // Extract first ident from object if exists.
      // For example: foo.bar.baz -> foo
      fn extract_first_ident(o: &ExprOrSuper) -> Option<&Ident> {
        match o {
          ExprOrSuper::Super(_) => None,
          ExprOrSuper::Expr(ref expr) => match &**expr {
            Expr::Ident(ref ident) => Some(ident),
            Expr::Member(MemberExpr { ref obj, .. }) => {
              extract_first_ident(obj)
            }
            _ => None,
          },
        }
      }

      if let Expr::Ident(ref first_ident) = &**prop {
        if is_underscored(first_ident) {
          errors.push(first_ident.clone());
        }
      }

      if let Some(last_ident) = extract_first_ident(obj) {
        if is_underscored(last_ident) {
          errors.push(last_ident.clone());
        }
      }
    }
    Expr::Ident(ref ident) => {
      if is_underscored(ident) {
        errors.push(ident.clone());
      }
    }
    _ => {}
  }

  errors
}

//macro_rules! check_struct {
//( $( $fn_name: ident, $ty_name: ident );* ) => {
//$(
//fn $fn_name(&mut self, t: &swc_ecmascript::ast::$ty_name, parent: &dyn Node) {
//swc_ecmascript::visit::$fn_name(self, t, parent);
//}
//)*
//};
//( $( $fn_name: ident, $ty_name: ident );* ;) => {
//check_struct!( $( $fn_name, $ty_name );* );
//};
//}

impl Visit for CamelcaseVisitor {
  noop_visit_type!();

  //check_struct!(
  //visit_fn_decl, FnDecl;
  //visit_class_decl, ClassDecl;
  //visit_fn_expr, FnExpr;
  //visit_class_expr, ClassExpr;
  //visit_meta_prop_expr, MetaPropExpr;
  //);

  // TODO(magurotuna): remove this
  fn visit_module(&mut self, module: &Module, parent: &dyn Node) {
    //dbg!(module);
    visit::visit_module(self, module, parent);
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, parent: &dyn Node) {
    if let ExprOrSuper::Expr(ref expr) = &call_expr.callee {
      match &**expr {
        Expr::Ident(ref ident) => {
          // Mark as checked without checking
          self.checked.insert(ident.span);
        }
        _ => {}
      }
    }
    visit::visit_call_expr(self, call_expr, parent);
  }

  fn visit_new_expr(&mut self, new_expr: &NewExpr, parent: &dyn Node) {
    if let Expr::Ident(ref ident) = &*new_expr.callee {
      // Mark as checked without checking
      self.checked.insert(ident.span);
    }
    visit::visit_new_expr(self, new_expr, parent);
  }

  fn visit_ident(&mut self, ident: &Ident, parent: &dyn Node) {
    self.check_and_insert(ident);
    visit::visit_ident(self, ident, parent);
  }

  fn visit_object_pat(&mut self, object_pat: &ObjectPat, parent: &dyn Node) {
    for prop in &object_pat.props {
      match prop {
        ObjectPatProp::KeyValue(KeyValuePatProp {
          ref key, ref value, ..
        }) => {
          match key {
            PropName::Ident(ref ident) => {
              self.checked.insert(ident.span);
            }
            PropName::Computed(ComputedPropName { ref expr, .. }) => {
              if let Expr::Ident(ref ident) = &**expr {
                self.check_and_insert(ident);
              }
            }
            _ => {}
          }
          // e.g. {a: b.foo_bar} = c
          self.checked.insert(key.span());
          if let Pat::Expr(ref expr) = &**value {
            if let Expr::Member(MemberExpr { ref prop, .. }) = &**expr {
              if let Expr::Ident(ref ident) = &**prop {
                self.check_and_insert(ident);
              }
            }
          } else if let Pat::Ident(ref ident) = &**value {
            self.check_and_insert(ident);
          }
        }
        ObjectPatProp::Assign(AssignPatProp {
          ref key, ref value, ..
        }) => {
          dbg!(key);
          dbg!(value);
          self.check_and_insert(key);
          if let Some(ref expr) = value {
            if let Expr::Ident(ref ident) = &**expr {
              self.checked.insert(ident.span);
            }
          }
        }
        _ => {}
      }
    }
    visit::visit_object_pat(self, object_pat, parent);
  }

  fn visit_array_pat(&mut self, array_pat: &ArrayPat, parent: &dyn Node) {
    // e.g. [a.foo_bar] = b
    for elem in &array_pat.elems {
      if let Some(Pat::Expr(ref expr)) = elem {
        if let Expr::Member(MemberExpr { ref prop, .. }) = &**expr {
          if let Expr::Ident(ref ident) = &**prop {
            self.check_and_insert(ident);
          }
        }
      }
    }
    visit::visit_array_pat(self, array_pat, parent);
  }

  fn visit_rest_pat(&mut self, rest_pat: &RestPat, parent: &dyn Node) {
    // e.g. {...a.foo_bar} = b
    if let Pat::Expr(ref expr) = &*rest_pat.arg {
      if let Expr::Member(MemberExpr { ref prop, .. }) = &**expr {
        if let Expr::Ident(ref ident) = &**prop {
          self.check_and_insert(ident);
        }
      }
    }
    visit::visit_rest_pat(self, rest_pat, parent);
  }

  fn visit_assign_pat(&mut self, assign_pat: &AssignPat, parent: &dyn Node) {
    match &*assign_pat.left {
      Pat::Expr(ref expr) => {
        // e.g. [a.foo_bar = 1] = b
        if let Expr::Member(MemberExpr { ref prop, .. }) = &**expr {
          if let Expr::Ident(ref ident) = &**prop {
            self.check_and_insert(ident);
          }
        }
      }
      Pat::Ident(ref ident) => {
        self.check_and_insert(ident);
      }
      _ => {}
    }
    visit::visit_assign_pat(self, assign_pat, parent);
  }

  fn visit_prop(&mut self, prop: &Prop, parent: &dyn Node) {
    match prop {
      Prop::Shorthand(ref ident) => {
        self.check_and_insert(ident);
      }
      Prop::KeyValue(KeyValueProp { ref key, .. }) => {
        if let PropName::Ident(ref ident) = key {
          self.check_and_insert(ident);
        }
      }
      _ => {}
    }
    visit::visit_prop(self, prop, parent);
  }

  fn visit_member_expr(&mut self, member_expr: &MemberExpr, parent: &dyn Node) {
    eprintln!("visit_member_expr");
    let MemberExpr {
      ref obj, ref prop, ..
    } = member_expr;

    if let ExprOrSuper::Expr(ref expr) = obj {
      if let Expr::Ident(ref ident) = &**expr {
        self.check_and_insert(ident);
      }
    }
    self.mark_member_ident_in_expr(&**prop);
    visit::visit_member_expr(self, member_expr, parent);
  }

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, parent: &dyn Node) {
    eprintln!("visit_assign_expr");
    let lhs = &assign_expr.left;
    let rhs = &*assign_expr.right;
    match rhs {
      Expr::Member(_) => {
        match lhs {
          PatOrExpr::Expr(ref expr) => {
            if let Expr::Member(MemberExpr { ref prop, .. }) = &**expr {
              if let Expr::Ident(ref ident) = &**prop {
                self.check_and_insert(ident);
              }
            }
          }
          PatOrExpr::Pat(ref pat) => {
            if let Pat::Expr(ref expr) = &**pat {
              match &**expr {
                Expr::Member(ref member_expr) => {
                  self.check_ident_in_member_expr(member_expr);
                }
                Expr::Ident(ref ident) => {
                  self.check_and_insert(ident);
                }
                _ => {}
              }
            }
          }
        }
        if let PatOrExpr::Expr(ref expr) = lhs {}
      }
      _ => match lhs {
        PatOrExpr::Expr(ref expr) => match &**expr {
          Expr::Member(ref member_expr) => {
            self.check_ident_in_member_expr(member_expr);
          }
          Expr::Ident(ref ident) => {
            self.check_and_insert(ident);
          }
          _ => {}
        },
        PatOrExpr::Pat(ref pat) => match &**pat {
          Pat::Ident(ref ident) => {
            self.check_and_insert(ident);
          }
          Pat::Expr(ref expr) => match &**expr {
            Expr::Member(ref member_expr) => {
              self.check_ident_in_member_expr(member_expr);
            }
            Expr::Ident(ref ident) => {
              self.check_and_insert(ident);
            }
            _ => {}
          },
          _ => {}
        },
      },
    }
    visit::visit_assign_expr(self, assign_expr, parent);
  }

  fn visit_import_specifier(
    &mut self,
    import_specifier: &ImportSpecifier,
    parent: &dyn Node,
  ) {
    use ImportSpecifier::*;
    match import_specifier {
      Named(ImportNamedSpecifier {
        ref local,
        ref imported,
        ..
      }) => {
        self.check_and_insert(local);
        if let Some(ref ident) = imported {
          self.checked.insert(ident.span);
        }
      }
      Default(ImportDefaultSpecifier { ref local, .. }) => {
        self.check_and_insert(local);
      }
      Namespace(ImportStarAsSpecifier { ref local, .. }) => {
        self.check_and_insert(local);
      }
    }
    visit::visit_import_specifier(self, import_specifier, parent);
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
  }

  #[test]
  fn camelcase_invalid() {
    assert_lint_err::<Camelcase>(r#"first_name = "Akari""#, 0);
    assert_lint_err::<Camelcase>(r#"__private_first_name = "Akari""#, 0);
    assert_lint_err::<Camelcase>(r#"function foo_bar(){}"#, 9);
    assert_lint_err::<Camelcase>(r#"obj.foo_bar = function(){};"#, 4);
    assert_lint_err::<Camelcase>(r#"bar_baz.foo = function(){};"#, 0);
    assert_lint_err::<Camelcase>(r#"[foo_bar.baz]"#, 1);
    assert_lint_err::<Camelcase>(
      r#"if (foo.bar_baz === boom.bam_pow) { [foo_bar.baz] }"#,
      37,
    );
    assert_lint_err::<Camelcase>(r#"foo.bar_baz = boom.bam_pow"#, 4);
    assert_lint_err::<Camelcase>(r#"var foo = { bar_baz: boom.bam_pow }"#, 12);
    assert_lint_err::<Camelcase>(
      r#"foo.qux.boom_pow = { bar: boom.bam_pow }"#,
      8,
    );
    assert_lint_err::<Camelcase>(r#"var o = {bar_baz: 1}"#, 9);
    assert_lint_err::<Camelcase>(r#"obj.a_b = 2;"#, 4);
    assert_lint_err::<Camelcase>(
      r#"var { category_id: category_alias } = query;"#,
      19,
    );
    assert_lint_err::<Camelcase>(
      r#"var { [category_id]: categoryId } = query;"#,
      7,
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
    assert_lint_err::<Camelcase>(r#"a_global_variable.foo()"#, 0);
    assert_lint_err::<Camelcase>(r#"a_global_variable[undefined]"#, 0);
    assert_lint_err::<Camelcase>(r#"var camelCased = snake_cased"#, 17);
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
    assert_lint_err::<Camelcase>(r#"not_ignored_foo = 0;"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: obj.fo_o } = bar);"#, 10);
    assert_lint_err::<Camelcase>(r#"({ a: obj.fo_o.b_ar } = baz);"#, 15);
    assert_lint_err::<Camelcase>(
      r#"({ a: { b: { c: obj.fo_o } } } = bar);"#,
      20,
    );
    assert_lint_err::<Camelcase>(
      r#"({ a: { b: { c: obj.fo_o.b_ar } } } = baz);"#,
      25,
    );
    assert_lint_err::<Camelcase>(r#"([obj.fo_o] = bar);"#, 6);
    assert_lint_err::<Camelcase>(r#"([obj.fo_o = 1] = bar);"#, 6);
    assert_lint_err::<Camelcase>(r#"({ a: [obj.fo_o] } = bar);"#, 11);
    assert_lint_err::<Camelcase>(r#"({ a: { b: [obj.fo_o] } } = bar);"#, 16);
    assert_lint_err::<Camelcase>(r#"([obj.fo_o.ba_r] = baz);"#, 11);
    assert_lint_err::<Camelcase>(r#"({...obj.fo_o} = baz);"#, 9);
    assert_lint_err::<Camelcase>(r#"({...obj.fo_o.ba_r} = baz);"#, 14);
    assert_lint_err::<Camelcase>(r#"({c: {...obj.fo_o }} = baz);"#, 13);
    assert_lint_err::<Camelcase>(r#"obj.o_k.non_camelcase = 0"#, 8);
    assert_lint_err::<Camelcase>(r#"(obj?.o_k).non_camelcase = 0"#, 11);
  }

  // TODO(magurotuna): remove this
  #[test]
  fn hogepiyo() {
    //assert_lint_ok::<Camelcase>(r#"const [a, ...b] = foo;"#);
    //assert_lint_err::<Camelcase>(r#"[a.fo_o.z] = b;"#, 3);
    //assert_lint_ok::<Camelcase>(r#"foo.qu_x.boompow = { bar: boom.bam_pow }"#);
    //assert_lint_ok::<Camelcase>(r#"var { category_id: category } = query;"#);
    //assert_lint_err::<Camelcase>(r#"[foo_bar.baz]"#, 0);

    //assert_lint_ok::<Camelcase>(r#"[foo.bar_baz]"#);
    //assert_lint_ok::<Camelcase>(r#"var arr = [foo.bar_baz.qux];"#);
    //assert_lint_ok::<Camelcase>(r#"[foo.bar_baz.nesting]"#);

    //assert_lint_err::<Camelcase>(r#"({...obj.fo_o} = baz);"#, 9);
    assert_lint_err::<Camelcase>(r#"({...obj.a} = baz);"#, 9);
  }
}
