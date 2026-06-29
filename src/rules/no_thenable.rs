// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{
  AssignExpr, AssignTarget, CallExpr, Callee, ClassMethod, ClassProp, Decl,
  ExportDecl, Expr, Lit, MemberProp, ModuleExportName, NamedExport, Node,
  ObjectLit, ObjectPatProp, Pat, Prop, PropName, PropOrSpread,
  SimpleAssignTarget,
};
use deno_ast::view::{ExportSpecifier, NodeTrait};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;
use std::collections::HashMap;

#[derive(Debug)]
pub struct NoThenable;

const CODE: &str = "no-thenable";

#[derive(Display)]
enum NoThenableMessage {
  #[display(fmt = "Do not add `then` to an object.")]
  Object,
  #[display(fmt = "Do not export `then`.")]
  Export,
  #[display(fmt = "Do not add `then` to a class.")]
  Class,
}

#[derive(Display)]
enum NoThenableHint {
  #[display(
    fmt = "If an object is defined as 'thenable', once it's accidentally used in an await expression, it may cause problems"
  )]
  Default,
}

impl LintRule for NoThenable {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    let mut then_vars = HashMap::new();
    collect_then_vars(program.as_node(), &mut then_vars);
    NoThenableHandler { then_vars }.traverse(program, context);
  }
}

/// Collects all `const`/`let`/`var` declarations whose initializer is the
/// string literal `"then"`, mapping the binding name to the range of that
/// string literal. This mirrors oxc resolving a computed-key identifier to its
/// declaration to see whether it evaluates to `"then"`.
fn collect_then_vars(node: Node, map: &mut HashMap<String, SourceRange>) {
  if let Node::VarDeclarator(declarator) = node {
    if let Pat::Ident(binding) = &declarator.name {
      if let Some(Expr::Lit(Lit::Str(s))) = &declarator.init {
        if s.value().as_str() == Some("then") {
          map.insert(binding.id.sym().to_string(), s.range());
        }
      }
    }
  }
  for child in node.children() {
    collect_then_vars(child, map);
  }
}

struct NoThenableHandler {
  then_vars: HashMap<String, SourceRange>,
}

impl NoThenableHandler {
  /// Returns the range to report if the given expression (used as a computed
  /// key, member property, or argument) evaluates to the string `"then"`.
  fn check_key_expr(&self, expr: &Expr) -> Option<SourceRange> {
    match expr {
      Expr::Lit(Lit::Str(s)) => {
        if s.value().as_str() == Some("then") {
          Some(s.range())
        } else {
          None
        }
      }
      Expr::Tpl(tpl) => {
        if tpl.exprs.is_empty()
          && tpl.quasis.len() == 1
          && tpl.quasis[0].cooked().as_ref().and_then(|c| c.as_str())
            == Some("then")
        {
          Some(tpl.range())
        } else {
          None
        }
      }
      Expr::Ident(ident) => self.then_vars.get(ident.sym().as_ref()).copied(),
      _ => None,
    }
  }

  /// Returns the range to report if a property name is `then`.
  fn contains_then(&self, key: &PropName) -> Option<SourceRange> {
    match key {
      PropName::Ident(ident) => {
        if ident.sym() == "then" {
          Some(ident.range())
        } else {
          None
        }
      }
      PropName::Str(s) => {
        if s.value().as_str() == Some("then") {
          Some(s.range())
        } else {
          None
        }
      }
      PropName::Computed(computed) => self.check_key_expr(&computed.expr),
      _ => None,
    }
  }
}

/// Recursively reports any binding identifier named `then` in an exported
/// destructuring pattern.
fn check_export_pat(pat: &Pat, ctx: &mut Context) {
  match pat {
    Pat::Ident(binding) => {
      if binding.id.sym() == "then" {
        ctx.add_diagnostic_with_hint(
          binding.id.range(),
          CODE,
          NoThenableMessage::Export,
          NoThenableHint::Default,
        );
      }
    }
    Pat::Array(arr) => {
      for elem in arr.elems.iter().flatten() {
        check_export_pat(elem, ctx);
      }
    }
    Pat::Object(obj) => {
      for prop in obj.props {
        match prop {
          ObjectPatProp::KeyValue(kv) => {
            check_export_pat(&kv.value, ctx);
          }
          ObjectPatProp::Assign(assign) => {
            if assign.key.id.sym() == "then" {
              ctx.add_diagnostic_with_hint(
                assign.key.id.range(),
                CODE,
                NoThenableMessage::Export,
                NoThenableHint::Default,
              );
            }
          }
          ObjectPatProp::Rest(rest) => {
            check_export_pat(&rest.arg, ctx);
          }
        }
      }
    }
    Pat::Rest(rest) => check_export_pat(&rest.arg, ctx),
    Pat::Assign(assign) => check_export_pat(&assign.left, ctx),
    _ => {}
  }
}

/// Returns true if `callee` is a member expression `<object>.<property>` where
/// `<object>` is one of `objects` and `<property>` matches `property`.
fn is_member_call(callee: &Callee, objects: &[&str], property: &str) -> bool {
  let Callee::Expr(Expr::Member(member)) = callee else {
    return false;
  };
  let Expr::Ident(object) = &member.obj else {
    return false;
  };
  if !objects.iter().any(|o| *o == object.sym().as_ref()) {
    return false;
  }
  matches!(&member.prop, MemberProp::Ident(p) if p.sym() == property)
}

impl Handler for NoThenableHandler {
  fn object_lit(&mut self, n: &ObjectLit, ctx: &mut Context) {
    for prop in n.props {
      if let PropOrSpread::Prop(prop) = prop {
        let range = match prop {
          Prop::Shorthand(ident) => {
            if ident.sym() == "then" {
              Some(ident.range())
            } else {
              None
            }
          }
          Prop::KeyValue(kv) => self.contains_then(&kv.key),
          Prop::Getter(getter) => self.contains_then(&getter.key),
          Prop::Setter(setter) => self.contains_then(&setter.key),
          Prop::Method(method) => self.contains_then(&method.key),
          Prop::Assign(_) => None,
        };
        if let Some(range) = range {
          ctx.add_diagnostic_with_hint(
            range,
            CODE,
            NoThenableMessage::Object,
            NoThenableHint::Default,
          );
        }
      }
    }
  }

  fn class_method(&mut self, n: &ClassMethod, ctx: &mut Context) {
    if let Some(range) = self.contains_then(&n.key) {
      ctx.add_diagnostic_with_hint(
        range,
        CODE,
        NoThenableMessage::Class,
        NoThenableHint::Default,
      );
    }
  }

  fn class_prop(&mut self, n: &ClassProp, ctx: &mut Context) {
    if let Some(range) = self.contains_then(&n.key) {
      ctx.add_diagnostic_with_hint(
        range,
        CODE,
        NoThenableMessage::Class,
        NoThenableHint::Default,
      );
    }
  }

  fn assign_expr(&mut self, n: &AssignExpr, ctx: &mut Context) {
    if let AssignTarget::Simple(SimpleAssignTarget::Member(member)) = &n.left {
      match &member.prop {
        MemberProp::Ident(ident) => {
          if ident.sym() == "then" {
            ctx.add_diagnostic_with_hint(
              member.range(),
              CODE,
              NoThenableMessage::Class,
              NoThenableHint::Default,
            );
          }
        }
        MemberProp::Computed(computed) => {
          if let Some(range) = self.check_key_expr(&computed.expr) {
            ctx.add_diagnostic_with_hint(
              range,
              CODE,
              NoThenableMessage::Class,
              NoThenableHint::Default,
            );
          }
        }
        MemberProp::PrivateName(_) => {}
      }
    }
  }

  fn call_expr(&mut self, n: &CallExpr, ctx: &mut Context) {
    // `Object.defineProperty(foo, "then", …)`
    // `Reflect.defineProperty(foo, "then", …)`
    if n.args.len() >= 3
      && n.args[0].spread().is_none()
      && is_member_call(&n.callee, &["Reflect", "Object"], "defineProperty")
    {
      let arg = n.args[1];
      if arg.spread().is_none() {
        if let Some(range) = self.check_key_expr(&arg.expr) {
          ctx.add_diagnostic_with_hint(
            range,
            CODE,
            NoThenableMessage::Object,
            NoThenableHint::Default,
          );
        }
      }
    }

    // `Object.fromEntries([["then", …]])`
    if n.args.len() == 1
      && n.args[0].spread().is_none()
      && is_member_call(&n.callee, &["Object"], "fromEntries")
    {
      if let Expr::Array(outer) = &n.args[0].expr {
        for elem in outer.elems {
          let Some(elem) = elem else {
            continue;
          };
          if elem.spread().is_some() {
            continue;
          }
          let Expr::Array(inner) = &elem.expr else {
            continue;
          };
          let Some(Some(first)) = inner.elems.first() else {
            continue;
          };
          if first.spread().is_some() {
            continue;
          }
          if let Some(range) = self.check_key_expr(&first.expr) {
            ctx.add_diagnostic_with_hint(
              range,
              CODE,
              NoThenableMessage::Object,
              NoThenableHint::Default,
            );
          }
        }
      }
    }
  }

  fn export_decl(&mut self, n: &ExportDecl, ctx: &mut Context) {
    match &n.decl {
      Decl::Var(var) => {
        for declarator in var.decls {
          check_export_pat(&declarator.name, ctx);
        }
      }
      Decl::Fn(fn_decl) => {
        if fn_decl.ident.sym() == "then" {
          ctx.add_diagnostic_with_hint(
            fn_decl.ident.range(),
            CODE,
            NoThenableMessage::Export,
            NoThenableHint::Default,
          );
        }
      }
      Decl::Class(class_decl) => {
        if class_decl.ident.sym() == "then" {
          ctx.add_diagnostic_with_hint(
            class_decl.ident.range(),
            CODE,
            NoThenableMessage::Export,
            NoThenableHint::Default,
          );
        }
      }
      _ => {}
    }
  }

  fn named_export(&mut self, n: &NamedExport, ctx: &mut Context) {
    for spec in n.specifiers {
      if let ExportSpecifier::Named(named) = spec {
        let exported = named.exported.as_ref().unwrap_or(&named.orig);
        let is_then = match exported {
          ModuleExportName::Ident(ident) => ident.sym() == "then",
          ModuleExportName::Str(s) => s.value().as_str() == Some("then"),
        };
        if is_then {
          ctx.add_diagnostic_with_hint(
            exported.range(),
            CODE,
            NoThenableMessage::Export,
            NoThenableHint::Default,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_thenable.rs
  // MIT Licensed.

  #[test]
  fn no_thenable_valid() {
    assert_lint_ok! {
      NoThenable,
      "const then = {}",
      "const notThen = then",
      "const then = then.then",
      "const foo = {notThen: 1}",
      "const foo = {notThen() {}}",
      "const foo = {[then]: 1}",
      r#"const NOT_THEN = "no-then";const foo = {[NOT_THEN]: 1}"#,
      "function foo({then}) {}",
      "({[Symbol.prototype]: 1})",
      "class then {}",
      "class Foo {notThen}",
      "class Foo {notThen() {}}",
      "class Foo {[then]}",
      "class Foo {#then}",
      "class Foo {#then() {}}",
      "class Foo {[then]() {}}",
      "class Foo {get notThen() {}}",
      "class Foo {get #then() {}}",
      "class Foo {get [then]() {}}",
      "class Foo {static notThen}",
      "class Foo {static notThen() {}}",
      "class Foo {static #then}",
      "class Foo {static #then() {}}",
      "class Foo {static [then]}",
      "class Foo {static [then]() {}}",
      "class Foo {static get notThen() {}}",
      "class Foo {static get #then() {}}",
      "class Foo {static get [then]() {}}",
      "class Foo {notThen = then}",
      "class Foo {[Symbol.property]}",
      "class Foo {static [Symbol.property]}",
      "class Foo {get [Symbol.property]() {}}",
      "class Foo {[Symbol.property]() {}}",
      "class Foo {static get [Symbol.property]() {}}",
      "foo[then] = 1",
      "foo.notThen = 1",
      "then.notThen = then.then",
      r#"const NOT_THEN = "no-then";foo[NOT_THEN] = 1"#,
      "foo.then ++",
      "++ foo.then",
      "delete foo.then",
      "typeof foo.then",
      "foo.then != 1",
      "foo[Symbol.property] = 1",
      "Object.fromEntries([then, 1])",
      "Object.fromEntries([,,])",
      "Object.fromEntries([[,,],[]])",
      r#"const NOT_THEN = "not-then";Object.fromEntries([[NOT_THEN, 1]])"#,
      r#"Object.fromEntries([[["then", 1]]])"#,
      r#"NotObject.fromEntries([["then", 1]])"#,
      r#"Object.notFromEntries([["then", 1]])"#,
      r#"Object.fromEntries?.([["then", 1]])"#,
      r#"Object?.fromEntries([["then", 1]])"#,
      r#"Object.fromEntries([[..."then", 1]])"#,
      r#"Object.fromEntries([["then", 1]], extraArgument)"#,
      r#"Object.fromEntries(...[["then", 1]])"#,
      "Object.fromEntries([[Symbol.property, 1]])",
      "Object.defineProperty(foo, then, 1)",
      r#"Object.defineProperty(foo, "not-then", 1)"#,
      r#"const then = "no-then";Object.defineProperty(foo, then, 1)"#,
      "Reflect.defineProperty(foo, then, 1)",
      r#"Reflect.defineProperty(foo, "not-then", 1)"#,
      r#"const then = "no-then";Reflect.defineProperty(foo, then, 1)"#,
      r#"Object.defineProperty(foo, "then", )"#,
      r#"Object.defineProperty(...foo, "then", 1)"#,
      r#"Object.defineProperty(foo, ...["then", 1])"#,
      "Object.defineProperty(foo, Symbol.property, 1)",
      "Reflect.defineProperty(foo, Symbol.property, 1)",
      r#"export {default} from "then""#,
      "const then = 1; export {then as notThen}",
      "export default then",
      "export function notThen(){}",
      "export class notThen {}",
      "export default function then (){}",
      "export default class then {}",
      "export default function (){}",
      "export default class {}",
      "export const notThen = 1",
      "export const {then: notThen} = 1",
      "export const {then: notThen = then} = 1",
    };
  }

  #[test]
  fn no_thenable_object_invalid() {
    assert_lint_err! {
      NoThenable,
      "const foo = {then: 1}": [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const foo = {["then"]: 1}"#: [
        { col: 14, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "const foo = {[`then`]: 1}": [
        { col: 14, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";const foo = {[THEN]: 1}"#: [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "const foo = {then() {}}": [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const foo = {["then"]() {}}"#: [
        { col: 14, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "const foo = {[`then`]() {}}": [
        { col: 14, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";const foo = {[THEN]() {}}"#: [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "const foo = {get then() {}}": [
        { col: 17, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const foo = {get ["then"]() {}}"#: [
        { col: 18, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "const foo = {get [`then`]() {}}": [
        { col: 18, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";const foo = {get [THEN]() {}}"#: [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"Object.defineProperty(foo, "then", 1)"#: [
        { col: 27, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "Object.defineProperty(foo, `then`, 1)": [
        { col: 27, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";Object.defineProperty(foo, THEN, 1)"#: [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"Reflect.defineProperty(foo, "then", 1)"#: [
        { col: 28, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "Reflect.defineProperty(foo, `then`, 1)": [
        { col: 28, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";Reflect.defineProperty(foo, THEN, 1)"#: [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"Object.fromEntries([["then", 1]])"#: [
        { col: 21, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"Object.fromEntries([["then"]])"#: [
        { col: 21, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      "Object.fromEntries([[`then`, 1]])": [
        { col: 21, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";Object.fromEntries([[THEN, 1]])"#: [
        { col: 13, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ],
      r#"Object.fromEntries([foo, ["then", 1]])"#: [
        { col: 26, message: NoThenableMessage::Object, hint: NoThenableHint::Default }
      ]
    };
  }

  #[test]
  fn no_thenable_class_invalid() {
    assert_lint_err! {
      NoThenable,
      "class Foo {then}": [
        { col: 11, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "const Foo = class {then}": [
        { col: 19, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"class Foo {["then"]}"#: [
        { col: 12, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {[`then`]}": [
        { col: 12, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";class Foo {[THEN]}"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {then() {}}": [
        { col: 11, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"class Foo {["then"]() {}}"#: [
        { col: 12, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {[`then`]() {}}": [
        { col: 12, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";class Foo {[THEN]() {}}"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {static then}": [
        { col: 18, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"class Foo {static ["then"]}"#: [
        { col: 19, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {static [`then`]}": [
        { col: 19, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";class Foo {static [THEN]}"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {static then() {}}": [
        { col: 18, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"class Foo {static ["then"]() {}}"#: [
        { col: 19, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {static [`then`]() {}}": [
        { col: 19, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";class Foo {static [THEN]() {}}"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {get then() {}}": [
        { col: 15, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"class Foo {get ["then"]() {}}"#: [
        { col: 16, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {get [`then`]() {}}": [
        { col: 16, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";class Foo {get [THEN]() {}}"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {set then(v) {}}": [
        { col: 15, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"class Foo {set ["then"](v) {}}"#: [
        { col: 16, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {set [`then`](v) {}}": [
        { col: 16, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";class Foo {set [THEN](v) {}}"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {static get then() {}}": [
        { col: 22, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"class Foo {static get ["then"]() {}}"#: [
        { col: 23, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "class Foo {static get [`then`]() {}}": [
        { col: 23, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";class Foo {static get [THEN]() {}}"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "foo.then = 1": [
        { col: 0, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"foo["then"] = 1"#: [
        { col: 4, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "foo[`then`] = 1": [
        { col: 4, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      r#"const THEN = "then";foo[THEN] = 1"#: [
        { col: 13, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "foo.then += 1": [
        { col: 0, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "foo.then ||= 1": [
        { col: 0, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ],
      "foo.then ??= 1": [
        { col: 0, message: NoThenableMessage::Class, hint: NoThenableHint::Default }
      ]
    };
  }

  #[test]
  fn no_thenable_export_invalid() {
    assert_lint_err! {
      NoThenable,
      "const then = 1; export {then}": [
        { col: 24, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "const notThen = 1; export {notThen as then}": [
        { col: 38, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      r#"export {then} from "foo""#: [
        { col: 8, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export function then() {}": [
        { col: 16, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export async function then() {}": [
        { col: 22, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export function * then() {}": [
        { col: 18, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export async function * then() {}": [
        { col: 24, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export class then {}": [
        { col: 13, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const then = 1": [
        { col: 13, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export let then = 1": [
        { col: 11, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export var then = 1": [
        { col: 11, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const [then] = 1": [
        { col: 14, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export let [then] = 1": [
        { col: 12, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export var [then] = 1": [
        { col: 12, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const [, then] = 1": [
        { col: 16, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export let [, then] = 1": [
        { col: 14, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export var [, then] = 1": [
        { col: 14, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const [, ...then] = 1": [
        { col: 19, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export let [, ...then] = 1": [
        { col: 17, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export var [, ...then] = 1": [
        { col: 17, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const {then} = 1": [
        { col: 14, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export let {then} = 1": [
        { col: 12, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export var {then} = 1": [
        { col: 12, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const {foo, ...then} = 1": [
        { col: 22, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export let {foo, ...then} = 1": [
        { col: 20, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export var {foo, ...then} = 1": [
        { col: 20, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const {foo: {bar: [{baz: then}]}} = 1": [
        { col: 32, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ],
      "export const notThen = 1, then = 1": [
        { col: 26, message: NoThenableMessage::Export, hint: NoThenableHint::Default }
      ]
    };
  }
}
