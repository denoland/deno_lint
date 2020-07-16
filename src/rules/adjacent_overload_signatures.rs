#![allow(dead_code, unused)]
// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_atoms::JsWord;
use crate::swc_common::Spanned;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::{
  Class, ClassMember, ClassMethod, Decl, ExportDecl, ExprStmt, FnDecl, Module,
  ModuleDecl, ModuleItem, Program, Stmt, TsModuleBlock, TsNamespaceBody,
};
use crate::swc_util::Key;
use std::collections::HashSet;
use std::sync::Arc;
use swc_ecma_visit::{Node, Visit};

pub struct AdjacentOverloadSignatures;

impl LintRule for AdjacentOverloadSignatures {
  fn new() -> Box<Self> {
    Box::new(AdjacentOverloadSignatures)
  }

  fn code(&self) -> &'static str {
    "adjacent-overload-signatures"
  }

  fn lint_module(&self, context: Arc<Context>, module: &Module) {
    let mut visitor = AdjacentOverloadSignaturesVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct AdjacentOverloadSignaturesVisitor {
  context: Arc<Context>,
}

impl AdjacentOverloadSignaturesVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

fn extract_ident(decl: &Decl) -> Option<String> {
  match decl {
    Decl::Fn(FnDecl { ref ident, .. }) => Some(ident.sym.to_string()),
    _ => None,
  }
}

impl Visit for AdjacentOverloadSignaturesVisitor {
  fn visit_module(&mut self, module: &Module, parent: &dyn Node) {
    let mut seen_methods = HashSet::new();
    let mut last_method = None;
    module.body.iter().for_each(|module_item| {
      let fn_name = match module_item {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
          ref decl,
          ..
        })) => extract_ident(decl),
        ModuleItem::Stmt(ref stmt) => match stmt {
          Stmt::Decl(ref decl) => extract_ident(decl),
          _ => None,
        },
        _ => None,
      };

      if let Some(fn_name) = fn_name {
        let m = Method {
          name: fn_name,
          is_static: false,
        };

        if seen_methods.contains(&m) && last_method.as_ref() != Some(&m) {
          self.context.add_diagnostic(
            module_item.span(),
            "adjacent-overload-signatures",
            "piyo", // TODO(magurotuna)
          );
        }

        seen_methods.insert(m.clone());
        last_method = Some(m);
      } else {
        last_method = None;
      }
    });
    swc_ecma_visit::visit_module(self, module, parent);
  }

  fn visit_ts_module_block(
    &mut self,
    ts_module_block: &TsModuleBlock,
    parent: &dyn Node,
  ) {
    let mut seen_methods = HashSet::new();
    let mut last_method = None;

    ts_module_block.body.iter().for_each(|module_item| {
      let fn_name = match module_item {
        ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl {
          ref decl,
          ..
        })) => extract_ident(decl),
        ModuleItem::Stmt(ref stmt) => match stmt {
          Stmt::Decl(ref decl) => extract_ident(decl),
          _ => None,
        },
        _ => None,
      };

      if let Some(fn_name) = fn_name {
        let m = Method {
          name: fn_name,
          is_static: false,
        };

        if seen_methods.contains(&m) && last_method.as_ref() != Some(&m) {
          self.context.add_diagnostic(
            module_item.span(),
            "adjacent-overload-signatures",
            "piyo", // TODO(magurotuna)
          );
        }

        seen_methods.insert(m.clone());
        last_method = Some(m);
      } else {
        last_method = None;
      }
    });

    swc_ecma_visit::visit_ts_module_block(self, ts_module_block, parent);
  }

  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let mut seen_methods = HashSet::new();
    let mut last_method = None;

    class.body.iter().for_each(|member| {
      let method = match member {
        ClassMember::Method(ClassMethod {
          ref key, is_static, ..
        }) => key.get_key().map(|k| Method {
          name: dbg!(k),
          is_static: *is_static,
        }),
        _ => None,
      };

      if let Some(m) = method {
        if seen_methods.contains(&m) && last_method.as_ref() != Some(&m) {
          self.context.add_diagnostic(
            member.span(),
            "adjacent-overload-signatures",
            "piyo", // TODO(magurotuna)
          );
        }

        seen_methods.insert(m.clone());
        last_method = Some(m);
      } else {
        last_method = None;
      }
    });
    swc_ecma_visit::visit_class(self, class, parent);
  }
}

#[derive(PartialEq, Eq, Hash, Clone)]
struct Method {
  name: String,
  is_static: bool,
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn hogepiyo() {
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
    "#,
    );
  }

  #[test]
  fn adjacent_overload_signatures_valid() {
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
function error(a: string);
function error(b: number);
function error(ab: string | number) {}
export { error };
      "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
import { connect } from 'react-redux';
export interface ErrorMessageModel {
  message: string;
}
function mapStateToProps() {}
function mapDispatchToProps() {}
export default connect(mapStateToProps, mapDispatchToProps)(ErrorMessage);
      "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
export const foo = 'a',
  bar = 'b';
export interface Foo {}
export class Foo {}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
export interface Foo {}
export const foo = 'a',
  bar = 'b';
export class Foo {}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
const foo = 'a',
  bar = 'b';
interface Foo {}
class Foo {}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
interface Foo {}
const foo = 'a',
  bar = 'b';
class Foo {}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
export class Foo {}
export class Bar {}
export type FooBar = Foo | Bar;
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
export interface Foo {}
export class Foo {}
export class Bar {}
export type FooBar = Foo | Bar;
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
export function foo(s: string);
export function foo(n: number);
export function foo(sn: string | number) {}
export function bar(): void {}
export function baz(): void {}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
function foo(s: string);
function foo(n: number);
function foo(sn: string | number) {}
function bar(): void {}
function baz(): void {}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
declare function foo(s: string);
declare function foo(n: number);
declare function foo(sn: string | number);
declare function bar(): void;
declare function baz(): void;
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
declare module 'Foo' {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function foo(sn: string | number): void;
  export function bar(): void;
  export function baz(): void;
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
declare namespace Foo {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function foo(sn: string | number): void;
  export function bar(): void;
  export function baz(): void;
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
type Foo = {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
};
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
type Foo = {
  foo(s: string): void;
  ['foo'](n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
};
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  (s: string): void;
  (n: number): void;
  (sn: string | number): void;
  foo(n: number): void;
  bar(): void;
  baz(): void;
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(): void;
  bar: {
    baz(s: string): void;
    baz(n: number): void;
    baz(sn: string | number): void;
  };
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  new (s: string);
  new (n: number);
  new (sn: string | number);
  foo(): void;
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  constructor(s: string);
  constructor(n: number);
  constructor(sn: string | number) {}
  bar(): void {}
  baz(): void {}
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  name: string;
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  name: string;
  static foo(s: string): void;
  static foo(n: number): void;
  static foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
class Test {
  static test() {}
  untest() {}
  test() {}
}
    "#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"export default function <T>(foo: T) {}"#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"export default function named<T>(foo: T) {}"#,
    );
    assert_lint_ok::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  [Symbol.toStringTag](): void;
  [Symbol.iterator](): void;
}
    "#,
    );
  }

  #[test]
  fn adjacent_overload_signatures_invalid() {
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
export function foo(s: string);
export function foo(n: number);
export function bar(): void {}
export function baz(): void {}
export function foo(sn: string | number) {}
      "#,
      6,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
export function foo(s: string);
export function foo(n: number);
export type bar = number;
export type baz = number | string;
export function foo(sn: string | number) {}
      "#,
      6,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
function foo(s: string);
function foo(n: number);
function bar(): void {}
function baz(): void {}
function foo(sn: string | number) {}
      "#,
      6,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
function foo(s: string);
function foo(n: number);
type bar = number;
type baz = number | string;
function foo(sn: string | number) {}
      "#,
      6,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
function foo(s: string) {}
function foo(n: number) {}
const a = '';
const b = '';
function foo(sn: string | number) {}
      "#,
      6,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
function foo(s: string) {}
function foo(n: number) {}
class Bar {}
function foo(sn: string | number) {}
      "#,
      5,
      0,
    );

    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
function foo(s: string) {}
function foo(n: number) {}
function foo(sn: string | number) {}
class Bar {
  foo(s: string);
  foo(n: number);
  name: string;
  foo(sn: string | number) {}
}
      "#,
      9,
      2,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare function foo(s: string);
declare function foo(n: number);
declare function bar(): void;
declare function baz(): void;
declare function foo(sn: string | number);
      "#,
      6,
      8,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare function foo(s: string);
declare function foo(n: number);
const a = '';
const b = '';
declare function foo(sn: string | number);
      "#,
      6,
      8,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare module 'Foo' {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function bar(): void;
  export function baz(): void;
  export function foo(sn: string | number): void;
}
      "#,
      7,
      2,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare module 'Foo' {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function foo(sn: string | number): void;
  function baz(s: string): void;
  export function bar(): void;
  function baz(n: number): void;
  function baz(sn: string | number): void;
}
      "#,
      8,
      2,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare namespace Foo {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function bar(): void;
  export function baz(): void;
  export function foo(sn: string | number): void;
}
      "#,
      7,
      2,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare namespace Foo {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function foo(sn: string | number): void;
  function baz(s: string): void;
  export function bar(): void;
  function baz(n: number): void;
  function baz(sn: string | number): void;
}
      "#,
      8,
      2,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
type Foo = {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
};
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
type Foo = {
  foo(s: string): void;
  ['foo'](n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
};
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
type Foo = {
  foo(s: string): void;
  name: string;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
};
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  (s: string): void;
  foo(n: number): void;
  (n: number): void;
  (sn: string | number): void;
  bar(): void;
  baz(): void;
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(s: string): void;
  'foo'(n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(s: string): void;
  name: string;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  foo(): void;
  bar: {
    baz(s: string): void;
    baz(n: number): void;
    foo(): void;
    baz(sn: string | number): void;
  };
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  new (s: string);
  new (n: number);
  foo(): void;
  bar(): void;
  new (sn: string | number);
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  new (s: string);
  foo(): void;
  new (n: number);
  bar(): void;
  new (sn: string | number);
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  constructor(s: string);
  constructor(n: number);
  bar(): void {}
  baz(): void {}
  constructor(sn: string | number) {}
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void {}
  baz(): void {}
  foo(sn: string | number): void {}
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  bar(): void {}
  baz(): void {}
  foo(sn: string | number): void {}
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  // prettier-ignore
  "foo"(s: string): void;
  foo(n: number): void;
  bar(): void {}
  baz(): void {}
  foo(sn: string | number): void {}
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  constructor(s: string);
  name: string;
  constructor(n: number);
  constructor(sn: string | number) {}
  bar(): void {}
  baz(): void {}
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo(s: string): void;
  name: string;
  foo(n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  static foo(s: string): void;
  name: string;
  static foo(n: number): void;
  static foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
      "#,
      0,
      0,
    );
  }
}
