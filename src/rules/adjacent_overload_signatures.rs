// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::comments::Comment;
use crate::swc_common::comments::CommentKind;
use crate::swc_ecma_ast;

pub struct AdjacentOverloadSignatures;

impl AdjacentOverloadSignatures {
  fn lint_comment(&self, context: &Context, comment: &Comment) {
    if comment.kind != CommentKind::Line {
      return;
    }

    lazy_static! {
      static ref BTC_REGEX: regex::Regex =
        regex::Regex::new(r#"^/*\s*@ts-(expect-error|ignore|nocheck)$"#)
          .unwrap();
    }

    if BTC_REGEX.is_match(&comment.text) {
      context.add_diagnostic(
        comment.span,
        "ban-ts-comment",
        "ts directives are not allowed",
      );
    }
  }
}

impl LintRule for AdjacentOverloadSignatures {
  fn new() -> Box<Self> {
    Box::new(AdjacentOverloadSignatures)
  }

  fn code(&self) -> &'static str {
    "ban-ts-comment"
  }

  fn lint_module(&self, context: Context, _module: swc_ecma_ast::Module) {
    context.leading_comments.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
        self.lint_comment(&context, comment);
      }
    });
    context.trailing_comments.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
        self.lint_comment(&context, comment);
      }
    });
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
      0,
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
      0,
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
      0,
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
      0,
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
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
function foo(s: string) {}
function foo(n: number) {}
class Bar {}
function foo(sn: string | number) {}
      "#,
      0,
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
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare function foo(s: string);
declare function foo(n: number);
declare function bar(): void;
declare function baz(): void;
declare function foo(sn: string | number);
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<AdjacentOverloadSignatures>(
      r#"
declare function foo(s: string);
declare function foo(n: number);
const a = '';
const b = '';
declare function foo(sn: string | number);
      "#,
      0,
      0,
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
      0,
      0,
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
      0,
      0,
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
      0,
      0,
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
      0,
      0,
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
