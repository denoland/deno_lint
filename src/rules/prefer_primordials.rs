// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
#![allow(unused)] // TODO delete
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::{self as ast_view, NodeTrait};
use swc_common::Spanned;

pub struct PreferPrimordials;

const CODE: &str = "prefer-primordials";
const MESSAGE: &str = "TODO";
const HINT: &str = "TODO";

impl LintRule for PreferPrimordials {
  fn new() -> Box<Self> {
    Box::new(PreferPrimordials)
  }

  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: ast_view::Program<'_>,
  ) {
    PreferPrimordialsHandler.traverse(program, context);
  }

  fn docs(&self) -> &'static str {
    r#""#
  }
}

const TARGETS: &[&str] = &[
  "isNaN",
  "decodeURI",
  "decodeURIComponent",
  "encodeURI",
  "encodeURIComponent",
  "JSON",
  "Math",
  "Reflect",
  "AggregateError",
  "Array",
  "ArrayBuffer",
  "BigInt",
  "BigInt64Array",
  "Boolean",
  "DataView",
  "Date",
  "Error",
  "EvalError",
  "Float32Array",
  "Float64Array",
  "Function",
  "Int16Array",
  "Int32Array",
  "Int8Array",
  "Map",
  "Number",
  "Object",
  "queueMicrotask",
  "RangeError",
  "ReferenceError",
  "RegExp",
  "Set",
  "String",
  "Symbol",
  "SyntaxError",
  "TypeError",
  "Uint8Array",
  "URIError",
  "Uint16Array",
  "Uint32Array",
  "Uint8ClampedArray",
  "WeakMap",
  "WeakSet",
  "Promise",
];

// TODO parseInt -> Number.parseInt

struct PreferPrimordialsHandler;

impl Handler for PreferPrimordialsHandler {}

#[cfg(test)]
mod tests {
  use super::*;

  // Test cases are derived from
  // https://github.com/nodejs/node/blob/7919ced0c97e9a5b17e6042e0b57bc911d23583d/test/parallel/test-eslint-prefer-primordials.js
  //
  // Copyright Joyent, Inc. and other Node contributors.
  //
  // Permission is hereby granted, free of charge, to any person obtaining a
  // copy of this software and associated documentation files (the
  // "Software"), to deal in the Software without restriction, including
  // without limitation the rights to use, copy, modify, merge, publish,
  // distribute, sublicense, and/or sell copies of the Software, and to permit
  // persons to whom the Software is furnished to do so, subject to the
  // following conditions:
  //
  // The above copyright notice and this permission notice shall be included
  // in all copies or substantial portions of the Software.
  //
  // THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
  // OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
  // MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
  // NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
  // DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
  // OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
  // USE OR OTHER DEALINGS IN THE SOFTWARE.

  #[test]
  fn prefer_primordials_valid() {
    assert_lint_ok! {
      PreferPrimordials,
      r#"
const { Array } = primordials;
new Array();
      "#,
      r#"
const { JSONStringify } = primordials;
JSONStringify({});
      "#,
      r#"
const { SymbolFor } = primordials;
SymbolFor("foo");
      "#,
      r#"
const { SymbolIterator } = primordials;
class A {
  *[SymbolIterator] () {
    yield "a";
  }
}
      "#,
      r#"
const { SymbolIterator } = primordials;
const a = {
  *[SymbolIterator] () {
    yield "a";
  }
}
      "#,
      r#"
const { ObjectDefineProperty, SymbolToStringTag } = primordials;
ObjectDefineProperty(o, SymbolToStringTag, { value: "o" });
      "#,
      r#"
const { NumberParseInt } = primordials;
NumberParseInt("42");
      "#,
      r#"
const { ReflectOwnKeys } = primordials;
ReflectOwnKeys({});
      "#,
      r#"
const { Map } = primordials;
new Map();
      "#,
      r#"
const { ArrayPrototypeMap } = primordials;
ArrayPrototypeMap([1, 2, 3], (val) => val * 2);
      "#,
    };
  }

  #[test]
  fn prefer_primordials_invalid() {
    assert_lint_err! {
      PreferPrimordials,
      r#"new Array()"#: [],
      r#"JSON.parse("{}")"#: [],
      r#"const { JSON } = primordials; JSON.parse("{}")"#: [],
      r#"Symbol.for("foo")"#: [],
      r#"const { Symbol } = primordials; Symbol.for("foo")"#: [],
      r#"
const { Symbol } = primordials;
class A {
  *[Symbol.iterator] () {
    yield "a";
  }
}
      "#: [],
      r#"
const { Symbol } = primordials;
const a = {
  *[Symbol.iterator] () {
    yield "a";
  }
}
      "#: [],
      r#"
const { ObjectDefineProperty, Symbol } = primordials;
ObjectDefineProperty(o, Symbol.toStringTag, { value: "o" });
      "#: [],
      r#"
const { Number } = primordials;
Number.parseInt('10');
      "#: [],
      r#"parseInt("10")"#: [],
      r#"const { ownKeys } = Reflect;"#: [],
      r#"new Map();"#: [],
      r#"
const { Function } = primordials;
const noop = Function.prototype;
      "#: [],
      r#"[1, 2, 3].map(val => val * 2);"#: [],
    }
  }
}
