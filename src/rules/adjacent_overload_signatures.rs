// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_util::Key;
use std::collections::HashSet;
use swc_common::Span;
use swc_common::Spanned;
use swc_ecmascript::ast::{
  Class, ClassMember, ClassMethod, Decl, ExportDecl, Expr, FnDecl, Ident, Lit,
  Module, ModuleDecl, ModuleItem, Stmt, Str, TsInterfaceBody,
  TsMethodSignature, TsModuleBlock, TsTypeElement, TsTypeLit,
};
use swc_ecmascript::visit::{Node, Visit};

pub struct AdjacentOverloadSignatures;

impl LintRule for AdjacentOverloadSignatures {
  fn new() -> Box<Self> {
    Box::new(AdjacentOverloadSignatures)
  }

  fn code(&self) -> &'static str {
    "adjacent-overload-signatures"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = AdjacentOverloadSignaturesVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct AdjacentOverloadSignaturesVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> AdjacentOverloadSignaturesVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span, fn_name: &str) {
    self.context.add_diagnostic(
      span,
      "adjacent-overload-signatures",
      &format!("All '{}' signatures should be adjacent", fn_name),
    );
  }

  fn check<'a, 'b, T, U>(&'a self, items: T)
  where
    T: IntoIterator<Item = &'b U>,
    U: ExtractMethod + Spanned + 'b,
  {
    let mut seen_methods = HashSet::new();
    let mut last_method = None;
    for item in items {
      if let Some(method) = item.get_method() {
        if seen_methods.contains(&method)
          && last_method.as_ref() != Some(&method)
        {
          self.add_diagnostic(item.span(), method.get_name());
        }

        seen_methods.insert(method.clone());
        last_method = Some(method);
      } else {
        last_method = None;
      }
    }
  }
}

trait ExtractMethod {
  fn get_method(&self) -> Option<Method>;
}

impl ExtractMethod for ModuleItem {
  fn get_method(&self) -> Option<Method> {
    let extract_ident = |decl: &Decl| match decl {
      Decl::Fn(FnDecl { ref ident, .. }) => Some(ident.sym.to_string()),
      _ => None,
    };

    let method_name = match self {
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

    method_name.map(Method::Method)
  }
}

impl ExtractMethod for ClassMember {
  fn get_method(&self) -> Option<Method> {
    match self {
      ClassMember::Method(ClassMethod {
        ref key, is_static, ..
      }) => key.get_key().map(|k| {
        if *is_static {
          Method::Static(k)
        } else {
          Method::Method(k)
        }
      }),
      ClassMember::Constructor(_) => {
        Some(Method::Method("constructor".to_string()))
      }
      _ => None,
    }
  }
}

impl ExtractMethod for TsTypeElement {
  fn get_method(&self) -> Option<Method> {
    match self {
      TsTypeElement::TsMethodSignature(TsMethodSignature {
        ref key, ..
      }) => match &**key {
        Expr::Ident(Ident { ref sym, .. }) => {
          Some(Method::Method(sym.to_string()))
        }
        Expr::Lit(Lit::Str(Str { ref value, .. })) => {
          Some(Method::Method(value.to_string()))
        }
        _ => None,
      },
      TsTypeElement::TsCallSignatureDecl(_) => Some(Method::CallSignature),
      TsTypeElement::TsConstructSignatureDecl(_) => {
        Some(Method::ConstructSignature)
      }
      _ => None,
    }
  }
}

impl<'c> Visit for AdjacentOverloadSignaturesVisitor<'c> {
  fn visit_module(&mut self, module: &Module, parent: &dyn Node) {
    self.check(&module.body);
    swc_ecmascript::visit::visit_module(self, module, parent);
  }

  fn visit_ts_module_block(
    &mut self,
    ts_module_block: &TsModuleBlock,
    parent: &dyn Node,
  ) {
    self.check(&ts_module_block.body);
    swc_ecmascript::visit::visit_ts_module_block(self, ts_module_block, parent);
  }

  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    self.check(&class.body);
    swc_ecmascript::visit::visit_class(self, class, parent);
  }

  fn visit_ts_type_lit(&mut self, ts_type_lit: &TsTypeLit, parent: &dyn Node) {
    self.check(&ts_type_lit.members);
    swc_ecmascript::visit::visit_ts_type_lit(self, ts_type_lit, parent);
  }

  fn visit_ts_interface_body(
    &mut self,
    ts_inteface_body: &TsInterfaceBody,
    parent: &dyn Node,
  ) {
    self.check(&ts_inteface_body.body);
    swc_ecmascript::visit::visit_ts_interface_body(
      self,
      ts_inteface_body,
      parent,
    );
  }
}

#[derive(PartialEq, Eq, Hash, Clone)]
enum Method {
  Method(String),
  Static(String),
  CallSignature,
  ConstructSignature,
}

impl Method {
  fn get_name(&self) -> &str {
    match self {
      Method::Method(ref s) | Method::Static(ref s) => s,
      Method::CallSignature => "call",
      Method::ConstructSignature => "new",
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

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
  (s: string): void;
  (n: number): void;
  (sn: string | number): void;
  foo(n: number): void;
  bar(): void;
  baz(): void;
  call(): void;
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
  "foo"(n: number): void;
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
  foo(s: string): void;
  [`foo`](n: number): void;
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
  foo() {}
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
      7,
      2,
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
      7,
      2,
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
      5,
      2,
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
      5,
      2,
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
      7,
      2,
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
      7,
      2,
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
      7,
      2,
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
      5,
      2,
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
      8,
      4,
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
      7,
      2,
    );
    assert_lint_err_on_line_n::<AdjacentOverloadSignatures>(
      r#"
interface Foo {
  new (s: string);
  foo(): void;
  new (n: number);
  bar(): void;
  new (sn: string | number);
}
      "#,
      vec![(5, 2), (7, 2)],
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
      7,
      2,
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
      7,
      2,
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
      7,
      2,
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
      8,
      2,
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
      5,
      2,
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
      5,
      2,
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
      5,
      2,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo() {
    class Bar {
      bar(): void;
      baz() {}
      bar(s: string): void;
    }
  }
}
      "#,
      7,
      6,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
class Foo {
  foo() {
    class Bar {
      bar(): void;
      baz() {}
      bar(s: string): void;
    }
  }
}
      "#,
      7,
      6,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
type Foo = {
  foo(): void;
  bar: {
    baz(s: string): void;
    baz(n: number): void;
    foo(): void;
    baz(sn: string | number): void;
  };
}
      "#,
      8,
      4,
    );
  }
}
