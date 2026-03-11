// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::swc_util::StringRepr;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::GetSpan;
use deno_ast::oxc::span::Span;
use derive_more::Display;
use std::collections::HashSet;

#[derive(Debug)]
pub struct AdjacentOverloadSignatures;

const CODE: &str = "adjacent-overload-signatures";

#[derive(Display)]
enum AdjacentOverloadSignaturesMessage {
  #[display(fmt = "All `{}` signatures should be adjacent", _0)]
  ShouldBeAdjacent(String),
}

#[derive(Display)]
enum AdjacentOverloadSignaturesHint {
  #[display(fmt = "Make sure all overloaded signatures are grouped together")]
  GroupedTogether,
}

impl LintRule for AdjacentOverloadSignatures {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = AdjacentOverloadSignaturesHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct AdjacentOverloadSignaturesHandler;

impl Handler<'_> for AdjacentOverloadSignaturesHandler {
  fn program(&mut self, program: &Program, ctx: &mut Context) {
    check_stmts(&program.body, ctx);
  }

  fn ts_module_block(
    &mut self,
    ts_module_block: &TSModuleBlock,
    ctx: &mut Context,
  ) {
    check_stmts(&ts_module_block.body, ctx);
  }

  fn class_body(&mut self, class_body: &ClassBody, ctx: &mut Context) {
    check_class_body(class_body, ctx);
  }

  fn ts_interface_body(
    &mut self,
    ts_interface_body: &TSInterfaceBody,
    ctx: &mut Context,
  ) {
    check_ts_signatures(&ts_interface_body.body, ctx);
  }

  fn ts_type_literal(
    &mut self,
    ts_type_lit: &TSTypeLiteral,
    ctx: &mut Context,
  ) {
    check_ts_signatures(&ts_type_lit.members, ctx);
  }
}

fn check_stmts(stmts: &[Statement], ctx: &mut Context) {
  let mut seen_methods = HashSet::new();
  let mut last_method = None;
  for stmt in stmts {
    if let Some((method, span)) = extract_method_from_stmt(stmt) {
      if seen_methods.contains(&method) && last_method.as_ref() != Some(&method)
      {
        ctx.add_diagnostic_with_hint(
          span,
          CODE,
          AdjacentOverloadSignaturesMessage::ShouldBeAdjacent(
            method.to_string(),
          ),
          AdjacentOverloadSignaturesHint::GroupedTogether,
        );
      }

      seen_methods.insert(method.clone());
      last_method = Some(method);
    } else {
      last_method = None;
    }
  }
}

fn extract_method_from_stmt(stmt: &Statement) -> Option<(Method, Span)> {
  match stmt {
    Statement::FunctionDeclaration(func) => {
      func.id.as_ref().map(|id| {
        (Method::Method(id.name.to_string()), stmt.span())
      })
    }
    Statement::ExportNamedDeclaration(export_decl) => {
      if let Some(Declaration::FunctionDeclaration(func)) =
        &export_decl.declaration
      {
        func.id.as_ref().map(|id| {
          (Method::Method(id.name.to_string()), stmt.span())
        })
      } else {
        None
      }
    }
    _ => None,
  }
}

fn check_class_body(class_body: &ClassBody, ctx: &mut Context) {
  let mut seen_methods = HashSet::new();
  let mut last_method = None;
  for element in &class_body.body {
    if let Some((method, span)) = extract_method_from_class_element(element) {
      if seen_methods.contains(&method) && last_method.as_ref() != Some(&method)
      {
        ctx.add_diagnostic_with_hint(
          span,
          CODE,
          AdjacentOverloadSignaturesMessage::ShouldBeAdjacent(
            method.to_string(),
          ),
          AdjacentOverloadSignaturesHint::GroupedTogether,
        );
      }

      seen_methods.insert(method.clone());
      last_method = Some(method);
    } else {
      last_method = None;
    }
  }
}

fn extract_method_from_class_element(
  element: &ClassElement,
) -> Option<(Method, Span)> {
  match element {
    ClassElement::MethodDefinition(method_def) => {
      if method_def.kind == MethodDefinitionKind::Constructor {
        return Some((
          Method::Method("constructor".to_string()),
          method_def.span,
        ));
      }
      method_def.key.string_repr().map(|k| {
        let method = if method_def.r#static {
          Method::Static(k)
        } else {
          Method::Method(k)
        };
        (method, method_def.span)
      })
    }
    _ => None,
  }
}

fn check_ts_signatures(members: &[TSSignature], ctx: &mut Context) {
  let mut seen_methods = HashSet::new();
  let mut last_method = None;
  for member in members {
    if let Some((method, span)) = extract_method_from_ts_signature(member) {
      if seen_methods.contains(&method) && last_method.as_ref() != Some(&method)
      {
        ctx.add_diagnostic_with_hint(
          span,
          CODE,
          AdjacentOverloadSignaturesMessage::ShouldBeAdjacent(
            method.to_string(),
          ),
          AdjacentOverloadSignaturesHint::GroupedTogether,
        );
      }

      seen_methods.insert(method.clone());
      last_method = Some(method);
    } else {
      last_method = None;
    }
  }
}

fn extract_method_from_ts_signature(
  member: &TSSignature,
) -> Option<(Method, Span)> {
  match member {
    TSSignature::TSMethodSignature(method_sig) => {
      let key_name = method_sig.key.string_repr()?;
      Some((Method::Method(key_name), method_sig.span))
    }
    TSSignature::TSCallSignatureDeclaration(call_sig) => {
      Some((Method::CallSignature, call_sig.span))
    }
    TSSignature::TSConstructSignatureDeclaration(construct_sig) => {
      Some((Method::ConstructSignature, construct_sig.span))
    }
    _ => None,
  }
}

#[derive(PartialEq, Eq, Hash, Clone, Display)]
#[allow(clippy::enum_variant_names)]
enum Method {
  #[display(fmt = "{}", _0)]
  Method(String),
  #[display(fmt = "{}", _0)]
  Static(String),
  #[display(fmt = "call")]
  CallSignature,
  #[display(fmt = "new")]
  ConstructSignature,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "baz"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "baz"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "call"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "baz"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "new"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "new"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
            },
            {
              line: 7,
              col: 2,
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "new"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "constructor"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "constructor"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "foo"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "bar"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "bar"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
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
              message: variant!(AdjacentOverloadSignaturesMessage, ShouldBeAdjacent, "baz"),
              hint: AdjacentOverloadSignaturesHint::GroupedTogether,
            }
          ]
    };
  }
}
