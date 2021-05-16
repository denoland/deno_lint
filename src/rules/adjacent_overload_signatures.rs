// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use crate::swc_util::StringRepr;
use dprint_swc_ecma_ast_view::{self as AstView, Spanned};
use std::collections::HashSet;

pub struct AdjacentOverloadSignatures;

const CODE: &str = "adjacent-overload-signatures";

impl LintRule for AdjacentOverloadSignatures {
  fn new() -> Box<Self> {
    Box::new(AdjacentOverloadSignatures)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: AstView::Program<'_>,
  ) {
    AdjacentOverloadSignaturesHandler.traverse(program, context);
  }

  fn docs(&self) -> &'static str {
    r#"Requires overload signatures to be adjacent to each other.

Overloaded signatures which are not next to each other can lead to code which is hard to read and maintain.

### Invalid:
(bar is declared in-between foo overloads)
```typescript
type FooType = {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void;
  foo(sn: string | number): void;
};
```
```typescript
interface FooInterface {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void;
  foo(sn: string | number): void;
}
```
```typescript
class FooClass {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void {}
  foo(sn: string | number): void {}
}
```
```typescript
export function foo(s: string): void;
export function foo(n: number): void;
export function bar(): void {}
export function foo(sn: string | number): void {}
```
### Valid:
(bar is declared after foo)
```typescript
type FooType = {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
};
```
```typescript
interface FooInterface {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
}
```
```typescript
class FooClass {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
}
```
```typescript
export function foo(s: string): void;
export function foo(n: number): void;
export function foo(sn: string | number): void {}
export function bar(): void {}
```"#
  }
}

struct AdjacentOverloadSignaturesHandler;

impl Handler for AdjacentOverloadSignaturesHandler {
  fn script(&mut self, script: &AstView::Script, ctx: &mut Context) {
    check(ctx, &script.body);
  }

  fn module(&mut self, module: &AstView::Module, ctx: &mut Context) {
    check(ctx, &module.body);
  }

  fn ts_module_block(
    &mut self,
    ts_module_block: &AstView::TsModuleBlock,
    ctx: &mut Context,
  ) {
    check(ctx, &ts_module_block.body);
  }

  fn class(&mut self, class: &AstView::Class, ctx: &mut Context) {
    check(ctx, &class.body);
  }

  fn ts_type_lit(
    &mut self,
    ts_type_lit: &AstView::TsTypeLit,
    ctx: &mut Context,
  ) {
    check(ctx, &ts_type_lit.members);
  }

  fn ts_interface_body(
    &mut self,
    ts_interface_body: &AstView::TsInterfaceBody,
    ctx: &mut Context,
  ) {
    check(ctx, &ts_interface_body.body);
  }
}

fn check<'a, T, U>(ctx: &'a mut Context, items: T)
where
  T: IntoIterator<Item = &'a U>,
  U: ExtractMethod + Spanned + 'a,
{
  let mut seen_methods = HashSet::new();
  let mut last_method = None;
  for item in items {
    if let Some(method) = item.get_method() {
      if seen_methods.contains(&method) && last_method.as_ref() != Some(&method)
      {
        ctx.add_diagnostic_with_hint(
          item.span(),
          CODE,
          format!("All '{}' signatures should be adjacent", method.get_name()),
          "Make sure all overloaded signatures are grouped together",
        );
      }

      seen_methods.insert(method.clone());
      last_method = Some(method);
    } else {
      last_method = None;
    }
  }
}

fn extract_ident_from_decl(decl: &AstView::Decl) -> Option<String> {
  match decl {
    AstView::Decl::Fn(AstView::FnDecl { ref ident, .. }) => {
      Some(ident.sym().to_string())
    }
    _ => None,
  }
}

trait ExtractMethod {
  fn get_method(&self) -> Option<Method>;
}

impl<'a> ExtractMethod for AstView::ExportDecl<'a> {
  fn get_method(&self) -> Option<Method> {
    let method_name = extract_ident_from_decl(&self.decl);
    method_name.map(Method::Method)
  }
}

impl<'a> ExtractMethod for AstView::Stmt<'a> {
  fn get_method(&self) -> Option<Method> {
    let method_name = match self {
      AstView::Stmt::Decl(ref decl) => extract_ident_from_decl(decl),
      _ => None,
    };
    method_name.map(Method::Method)
  }
}

impl<'a> ExtractMethod for AstView::ModuleItem<'a> {
  fn get_method(&self) -> Option<Method> {
    use AstView::{ModuleDecl, ModuleItem};
    match self {
      ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(export_decl)) => {
        export_decl.get_method()
      }
      ModuleItem::Stmt(stmt) => stmt.get_method(),
      _ => None,
    }
  }
}

impl<'a> ExtractMethod for AstView::ClassMember<'a> {
  fn get_method(&self) -> Option<Method> {
    use AstView::{ClassMember, ClassMethod};
    match self {
      ClassMember::Method(ClassMethod { ref inner, .. }) => {
        inner.key.string_repr().map(|k| {
          if inner.is_static {
            Method::Static(k)
          } else {
            Method::Method(k)
          }
        })
      }
      ClassMember::Constructor(_) => {
        Some(Method::Method("constructor".to_string()))
      }
      _ => None,
    }
  }
}

impl<'a> ExtractMethod for AstView::TsTypeElement<'a> {
  fn get_method(&self) -> Option<Method> {
    use AstView::{Expr, Lit, TsMethodSignature, TsTypeElement};
    match self {
      TsTypeElement::TsMethodSignature(TsMethodSignature {
        ref key, ..
      }) => match &*key {
        Expr::Ident(ident) => Some(Method::Method(ident.sym().to_string())),
        Expr::Lit(Lit::Str(s)) => Some(Method::Method(s.value().to_string())),
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

  #[test]
  fn adjacent_overload_signatures_valid() {
    assert_lint_ok! {
      AdjacentOverloadSignatures,
      r#"
function error(a: string);
function error(b: number);
function error(ab: string | number) {}
export { error };
      "#,
      r#"
import { connect } from 'react-redux';
export interface ErrorMessageModel {
  message: string;
}
function mapStateToProps() {}
function mapDispatchToProps() {}
export default connect(mapStateToProps, mapDispatchToProps)(ErrorMessage);
      "#,
      r#"
export const foo = 'a',
  bar = 'b';
export interface Foo {}
export class Foo {}
      "#,
      r#"
export interface Foo {}
export const foo = 'a',
  bar = 'b';
export class Foo {}
      "#,
      r#"
const foo = 'a',
  bar = 'b';
interface Foo {}
class Foo {}
      "#,
      r#"
interface Foo {}
const foo = 'a',
  bar = 'b';
class Foo {}
      "#,
      r#"
export class Foo {}
export class Bar {}
export type FooBar = Foo | Bar;
      "#,
      r#"
export interface Foo {}
export class Foo {}
export class Bar {}
export type FooBar = Foo | Bar;
      "#,
      r#"
export function foo(s: string);
export function foo(n: number);
export function foo(sn: string | number) {}
export function bar(): void {}
export function baz(): void {}
      "#,
      r#"
function foo(s: string);
function foo(n: number);
function foo(sn: string | number) {}
function bar(): void {}
function baz(): void {}
      "#,
      r#"
declare function foo(s: string);
declare function foo(n: number);
declare function foo(sn: string | number);
declare function bar(): void;
declare function baz(): void;
      "#,
      r#"
declare module 'Foo' {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function foo(sn: string | number): void;
  export function bar(): void;
  export function baz(): void;
}
      "#,
      r#"
declare namespace Foo {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function foo(sn: string | number): void;
  export function bar(): void;
  export function baz(): void;
}
      "#,
      r#"
type Foo = {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
};
      "#,
      r#"
type Foo = {
  foo(s: string): void;
  ['foo'](n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
};
      "#,
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
      r#"
interface Foo {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
}
      "#,
      r#"
interface Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
}
      "#,
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
      r#"
interface Foo {
  new (s: string);
  new (n: number);
  new (sn: string | number);
  foo(): void;
}
      "#,
      r#"
class Foo {
  constructor(s: string);
  constructor(n: number);
  constructor(sn: string | number) {}
  bar(): void {}
  baz(): void {}
}
    "#,
      r#"
class Foo {
  foo(s: string): void;
  foo(n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
    "#,
      r#"
class Foo {
  foo(s: string): void;
  "foo"(n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
    "#,
      r#"
class Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
      "#,
      r#"
class Foo {
  foo(s: string): void;
  [`foo`](n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
      "#,
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
      r#"
class Test {
  static test() {}
  untest() {}
  test() {}
}
      "#,
      r#"export default function <T>(foo: T) {}"#,
      r#"export default function named<T>(foo: T) {}"#,
      r#"
interface Foo {
  [Symbol.toStringTag](): void;
  [Symbol.iterator](): void;
}
      "#,
    };
  }

  #[test]
  fn adjacent_overload_signatures_invalid() {
    assert_lint_err! {
      AdjacentOverloadSignatures,
      r#"
export function foo(s: string);
export function foo(n: number);
export function bar(): void {}
export function baz(): void {}
export function foo(sn: string | number) {}
      "#: [
            {
              line: 6,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
export function foo(s: string);
export function foo(n: number);
export type bar = number;
export type baz = number | string;
export function foo(sn: string | number) {}
      "#: [
            {
              line: 6,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
function foo(s: string);
function foo(n: number);
function bar(): void {}
function baz(): void {}
function foo(sn: string | number) {}
      "#: [
            {
              line: 6,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
function foo(s: string);
function foo(n: number);
type bar = number;
type baz = number | string;
function foo(sn: string | number) {}
      "#: [
            {
              line: 6,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
function foo(s: string) {}
function foo(n: number) {}
const a = '';
const b = '';
function foo(sn: string | number) {}
      "#: [
            {
              line: 6,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
function foo(s: string) {}
function foo(n: number) {}
class Bar {}
function foo(sn: string | number) {}
      "#: [
            {
              line: 5,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
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
      "#: [
            {
              line: 9,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
declare function foo(s: string);
declare function foo(n: number);
declare function bar(): void;
declare function baz(): void;
declare function foo(sn: string | number);
      "#: [
            {
              line: 6,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
declare function foo(s: string);
declare function foo(n: number);
const a = '';
const b = '';
declare function foo(sn: string | number);
      "#: [
            {
              line: 6,
              col: 0,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
declare module 'Foo' {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function bar(): void;
  export function baz(): void;
  export function foo(sn: string | number): void;
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
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
      "#: [
            {
              line: 8,
              col: 2,
              message: "All 'baz' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
declare namespace Foo {
  export function foo(s: string): void;
  export function foo(n: number): void;
  export function bar(): void;
  export function baz(): void;
  export function foo(sn: string | number): void;
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
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
      "#: [
            {
              line: 8,
              col: 2,
              message: "All 'baz' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
type Foo = {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
};
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
type Foo = {
  foo(s: string): void;
  ['foo'](n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
};
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
type Foo = {
  foo(s: string): void;
  name: string;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
};
      "#: [
            {
              line: 5,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
interface Foo {
  (s: string): void;
  foo(n: number): void;
  (n: number): void;
  (sn: string | number): void;
  bar(): void;
  baz(): void;
}
      "#: [
            {
              line: 5,
              col: 2,
              message: "All 'call' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
interface Foo {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
interface Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
interface Foo {
  foo(s: string): void;
  'foo'(n: number): void;
  bar(): void;
  baz(): void;
  foo(sn: string | number): void;
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
interface Foo {
  foo(s: string): void;
  name: string;
  foo(n: number): void;
  foo(sn: string | number): void;
  bar(): void;
  baz(): void;
}
      "#: [
            {
              line: 5,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
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
      "#: [
            {
              line: 8,
              col: 4,
              message: "All 'baz' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
interface Foo {
  new (s: string);
  new (n: number);
  foo(): void;
  bar(): void;
  new (sn: string | number);
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'new' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
interface Foo {
  new (s: string);
  foo(): void;
  new (n: number);
  bar(): void;
  new (sn: string | number);
}
      "#: [
            {
              line: 5,
              col: 2,
              message: "All 'new' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            },
            {
              line: 7,
              col: 2,
              message: "All 'new' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
class Foo {
  constructor(s: string);
  constructor(n: number);
  bar(): void {}
  baz(): void {}
  constructor(sn: string | number) {}
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'constructor' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
class Foo {
  foo(s: string): void;
  foo(n: number): void;
  bar(): void {}
  baz(): void {}
  foo(sn: string | number): void {}
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
class Foo {
  foo(s: string): void;
  ['foo'](n: number): void;
  bar(): void {}
  baz(): void {}
  foo(sn: string | number): void {}
}
      "#: [
            {
              line: 7,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
class Foo {
  // prettier-ignore
  "foo"(s: string): void;
  foo(n: number): void;
  bar(): void {}
  baz(): void {}
  foo(sn: string | number): void {}
}
      "#: [
            {
              line: 8,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
class Foo {
  constructor(s: string);
  name: string;
  constructor(n: number);
  constructor(sn: string | number) {}
  bar(): void {}
  baz(): void {}
}
      "#: [
            {
              line: 5,
              col: 2,
              message: "All 'constructor' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
class Foo {
  foo(s: string): void;
  name: string;
  foo(n: number): void;
  foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
      "#: [
            {
              line: 5,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
r#"
class Foo {
  static foo(s: string): void;
  name: string;
  static foo(n: number): void;
  static foo(sn: string | number): void {}
  bar(): void {}
  baz(): void {}
}
      "#: [
            {
              line: 5,
              col: 2,
              message: "All 'foo' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
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
      "#: [
            {
              line: 7,
              col: 6,
              message: "All 'bar' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
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
      "#: [
            {
              line: 7,
              col: 6,
              message: "All 'bar' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ],
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
      "#: [
            {
              line: 8,
              col: 4,
              message: "All 'baz' signatures should be adjacent",
              hint: "Make sure all overloaded signatures are grouped together"
            }
          ]
    };
  }
}
