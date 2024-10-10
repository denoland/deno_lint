// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::Scope;
use deno_ast::{view as ast_view, SourceRanged};
use derive_more::Display;
use if_chain::if_chain;
use std::ptr;

#[derive(Debug)]
pub struct PreferPrimordials;

const CODE: &str = "prefer-primordials";

#[derive(Display)]
enum PreferPrimordialsMessage {
  #[display(fmt = "Don't use the global intrinsic")]
  GlobalIntrinsic,
  #[display(fmt = "Don't use the unsafe intrinsic")]
  UnsafeIntrinsic,
  #[display(fmt = "Use null [[prototype]] object in the define property")]
  DefineProperty,
  #[display(fmt = "Use null [[prototype]] object in the default parameter")]
  ObjectAssignInDefaultParameter,
  #[display(fmt = "Don't use iterator protocol directly")]
  Iterator,
  #[display(fmt = "Don't use RegExp literal directly")]
  RegExp,
  #[display(fmt = "Don't use `instanceof` operator")]
  InstanceOf,
  #[display(fmt = "Don't use `in` operator")]
  In,
}

#[derive(Display)]
enum PreferPrimordialsHint {
  #[display(fmt = "Instead use the equivalent from the `primordials` object")]
  GlobalIntrinsic,
  #[display(
    fmt = "Instead use the safe wrapper from the `primordials` object"
  )]
  UnsafeIntrinsic,
  #[display(fmt = "Add `__proto__: null` to this object literal")]
  NullPrototypeObjectLiteral,
  #[display(fmt = "Wrap a SafeIterator from the `primordials` object")]
  SafeIterator,
  #[display(fmt = "Wrap `SafeRegExp` from the `primordials` object")]
  SafeRegExp,
  #[display(fmt = "Instead use the object pattern destructuring assignment")]
  ObjectPattern,
  #[display(
    fmt = "Instead use `ObjectPrototypeIsPrototypeOf` from the `primordials` object"
  )]
  InstanceOf,
  #[display(
    fmt = "Instead use either `ObjectHasOwn` or `ReflectHas` from the `primordials` object"
  )]
  In,
}

impl LintRule for PreferPrimordials {
  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    PreferPrimordialsHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/prefer_primordials.md")
  }
}

const GLOBAL_TARGETS: &[&str] = &[
  "isFinite",
  "isNaN",
  "decodeURI",
  "decodeURIComponent",
  "encodeURI",
  "encodeURIComponent",
  "eval",
  "parseFloat",
  "parseInt",
  "queueMicrotask",
  "Atomics",
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
  "FinalizationRegistry",
  "Float32Array",
  "Float64Array",
  "Function",
  "Int16Array",
  "Int32Array",
  "Int8Array",
  "Map",
  "Number",
  "Object",
  "Promise",
  "Proxy",
  "RangeError",
  "ReferenceError",
  "RegExp",
  "Set",
  "SharedArrayBuffer",
  "String",
  "Symbol",
  "SyntaxError",
  "TypeError",
  "Uint8Array",
  "Uint16Array",
  "Uint32Array",
  "Uint8ClampedArray",
  "URIError",
  "WeakMap",
  "WeakRef",
  "WeakSet",
];

const UNSAFE_CONSTRUCTOR_TARGETS: &[&str] = &[
  "FinalizationRegistry",
  "Map",
  "RegExp",
  "Set",
  "WeakMap",
  "WeakRef",
  "WeakSet",
];

const UNSAFE_FUNCTION_TARGETS: &[&str] = &[
  "PromiseAll",
  "PromiseAllSettled",
  "PromiseAny",
  "PromiseRace",
  "PromisePrototypeFinally",
];

const METHOD_TARGETS: &[&str] = &[
  // Generic
  "toLocaleString",
  "toString",
  "valueOf",
  // Object
  "hasOwnProperty",
  "isPrototypeOf",
  "propertyIsEnumerable",
  // Function
  "apply",
  "bind",
  "call",
  // Number
  "toExponential",
  "toFixed",
  "toPrecision",
  // Date
  "getDate",
  "getDay",
  "getFullYear",
  "getHours",
  "getMilliseconds",
  "getMinutes",
  "getMonth",
  "getSeconds",
  "getTime",
  "getTimezoneOffset",
  "getUTCDate",
  "getUTCDay",
  "getUTCFullYear",
  "getUTCHours",
  "getUTCMilliseconds",
  "getUTCMinutes",
  "getUTCMonth",
  "getUTCSeconds",
  "getYear",
  "setDate",
  "setFullYear",
  "setHours",
  "setMilliseconds",
  "setMinutes",
  "setMonth",
  "setSeconds",
  "setTime",
  "setUTCDate",
  "setUTCFullYear",
  "setUTCHours",
  "setUTCMilliseconds",
  "setUTCMinutes",
  "setUTCMonth",
  "setUTCSeconds",
  "setYear",
  "toDateString",
  "toISOString",
  "toJSON",
  "toLocaleDateString",
  "toLocaleTimeString",
  "toTimeString",
  "toUTCString",
  // String, Array
  "at",
  "concat",
  "slice",
  "includes",
  "indexOf",
  "lastIndexOf",
  // Array, TypedArray
  "copyWithin",
  "entries",
  "every",
  "fill",
  "filter",
  "find",
  "findIndex",
  "findLast",
  "findLastIndex",
  "flat",
  "flatMap",
  "forEach",
  "join",
  "keys",
  "map",
  "pop",
  "push",
  "reduce",
  "reduceRight",
  "reverse",
  "shift",
  "some",
  "sort",
  "toReversed",
  "toSorted",
  "unshift",
  "values",
  "with",
  // String
  "charAt",
  "charCodeAt",
  "codePointAt",
  "endsWith",
  "localeCompare",
  "match",
  "matchAll",
  "normalize",
  "padEnd",
  "padStart",
  "repeat",
  "replace",
  "replaceAll",
  "search",
  "split",
  "startsWith",
  "substring",
  "toLocaleLowerCase",
  "toLocaleUpperCase",
  "toLowerCase",
  "toUpperCase",
  "trim",
  "trimEnd",
  "trimStart",
  // Array
  "splice",
  "toSpliced",
  // ArrayBuffer
  "resize",
  "transfer",
  "transferToFixedLength",
  // SharedArrayBuffer
  "grow",
  // TypedArray: avoid false positives for Map, Set, WeakMap, and WeakSet
  // "set",
  // DataView
  "getBigInt64",
  "getBigUint64",
  "getFloat32",
  "getFloat64",
  "getInt8",
  "getInt16",
  "getInt32",
  "getUint8",
  "getUint16",
  "getUint32",
  "setBigInt64",
  "setBigUint64",
  "setFloat32",
  "setFloat64",
  "setInt8",
  "setInt16",
  "setInt32",
  "setUint8",
  "setUint16",
  "setUint32",
  // Iterator, Generator
  "next",
  "return",
  "throw",
  // Promise
  "catch",
  "finally",
  "then",
];

const GETTER_TARGETS: &[&str] = &[
  // Symbol
  "description",
  // ArrayBuffer, TypedArray, DataView
  "buffer",
  "byteLength",
  "byteOffset",
  // ArrayBuffer, SharedArrayBuffer
  "maxByteLength",
  // ArrayBuffer
  "detached",
  "resizable",
  // SharedArrayBuffer
  "growable",
  // TypedArray: avoid false positives for Array
  // "length",
];

fn is_null_proto(object_lit: &ast_view::ObjectLit) -> bool {
  for prop_or_spread in object_lit.props {
    if_chain! {
      if let ast_view::PropOrSpread::Prop(prop) = prop_or_spread;
      if let ast_view::Prop::KeyValue(key_value_prop) = prop;
      if matches!(key_value_prop.value, ast_view::Expr::Lit(ast_view::Lit::Null(..)));
      then {
        if let ast_view::PropName::Ident(ident) = key_value_prop.key {
          if ident.sym().as_ref() == "__proto__" {
            return true
          }
        } else if let ast_view::PropName::Str(str) = key_value_prop.key {
          if str.inner.value.as_ref() == "__proto__" {
            return true
          }
        }
      }
    }
  }
  false
}

struct PreferPrimordialsHandler;

impl Handler for PreferPrimordialsHandler {
  fn ident(&mut self, ident: &ast_view::Ident, ctx: &mut Context) {
    fn inside_var_decl_lhs_or_member_expr_or_prop_or_type_ref(
      orig: ast_view::Node,
      node: ast_view::Node,
    ) -> bool {
      if node.is::<ast_view::MemberExpr>() {
        return true;
      }
      if let Some(decl) = node.to::<ast_view::VarDeclarator>() {
        return decl.name.range().contains(&orig.range());
      }
      if let Some(kv) = node.to::<ast_view::KeyValueProp>() {
        return kv.key.range().contains(&orig.range());
      }
      if let Some(type_ref) = node.to::<ast_view::TsTypeRef>() {
        return type_ref.type_name.range().contains(&orig.range());
      }

      match node.parent() {
        None => false,
        Some(parent) => {
          inside_var_decl_lhs_or_member_expr_or_prop_or_type_ref(orig, parent)
        }
      }
    }

    fn is_shadowed(ident: &ast_view::Ident, scope: &Scope) -> bool {
      scope.var(&ident.inner.to_id()).is_some()
    }

    if inside_var_decl_lhs_or_member_expr_or_prop_or_type_ref(
      ident.as_node(),
      ident.as_node(),
    ) {
      return;
    }

    if GLOBAL_TARGETS.contains(&ident.sym().as_ref())
      && !is_shadowed(ident, ctx.scope())
    {
      ctx.add_diagnostic_with_hint(
        ident.range(),
        CODE,
        PreferPrimordialsMessage::GlobalIntrinsic,
        PreferPrimordialsHint::GlobalIntrinsic,
      );
    }

    if UNSAFE_CONSTRUCTOR_TARGETS.contains(&ident.sym().as_ref())
      && matches!(ident.parent(), ast_view::Node::NewExpr(_))
    {
      ctx.add_diagnostic_with_hint(
        ident.range(),
        CODE,
        PreferPrimordialsMessage::UnsafeIntrinsic,
        PreferPrimordialsHint::UnsafeIntrinsic,
      );
    }

    if UNSAFE_FUNCTION_TARGETS.contains(&ident.sym().as_ref())
      && matches!(ident.parent(), ast_view::Node::CallExpr(_))
    {
      ctx.add_diagnostic_with_hint(
        ident.range(),
        CODE,
        PreferPrimordialsMessage::UnsafeIntrinsic,
        PreferPrimordialsHint::UnsafeIntrinsic,
      );
    }

    match &ident.sym().as_ref() {
      &"ObjectDefineProperty" | &"ReflectDefineProperty" => {
        if_chain! {
          if let ast_view::Node::CallExpr(call_expr) = ident.parent();
          if let Some(expr_or_spread) = call_expr.args.get(2);
          if let ast_view::Expr::Object(object_lit) = expr_or_spread.expr;
          if !is_null_proto(object_lit);
          then {
            ctx.add_diagnostic_with_hint(
              object_lit.range(),
              CODE,
              PreferPrimordialsMessage::DefineProperty,
              PreferPrimordialsHint::NullPrototypeObjectLiteral,
            );
          }
        }
      }
      &"ObjectDefineProperties" => {
        if_chain! {
          if let ast_view::Node::CallExpr(call_expr) = ident.parent();
          if let Some(expr_or_spread) = call_expr.args.get(1);
          if let ast_view::Expr::Object(object_lit) = expr_or_spread.expr;
          then {
            for prop_or_spread in object_lit.props {
              if_chain! {
                if let ast_view::PropOrSpread::Prop(prop) = prop_or_spread;
                if let ast_view::Prop::KeyValue(key_value_prop) = prop;
                if let ast_view::Expr::Object(object_lit) = key_value_prop.value;
                if !is_null_proto(object_lit);
                then {
                  ctx.add_diagnostic_with_hint(
                    object_lit.range(),
                    CODE,
                    PreferPrimordialsMessage::DefineProperty,
                    PreferPrimordialsHint::NullPrototypeObjectLiteral,
                  );
                }
              }
            }
          }
        }
      }
      _ => (),
    }
  }

  fn member_expr(
    &mut self,
    member_expr: &ast_view::MemberExpr,
    ctx: &mut Context,
  ) {
    use ast_view::{Expr, Node};

    // If `member_expr.obj` is an array literal, access to its properties or
    // methods should be replaced with the one from `primordials`.
    // For example:
    //
    // ```js
    // [1, 2, 3].filter((val) => val % 2 === 0)
    // ```
    //
    // should be turned into:
    //
    // ```js
    // primordials.ArrayPrototypeFilter([1, 2, 3], (val) => val % 2 === 0)
    // ```
    if let Expr::Array(_) = &member_expr.obj {
      ctx.add_diagnostic_with_hint(
        member_expr.range(),
        CODE,
        PreferPrimordialsMessage::GlobalIntrinsic,
        PreferPrimordialsHint::GlobalIntrinsic,
      );
      return;
    }

    if_chain! {
      // Don't check non-root elements in chained member expressions
      // e.g. `bar.baz` in `foo.bar.baz`
      if !member_expr.parent().is::<ast_view::MemberExpr>();
      if let Expr::Ident(ident) = &member_expr.obj;
      if GLOBAL_TARGETS.contains(&ident.sym().as_ref());
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.range(),
          CODE,
          PreferPrimordialsMessage::GlobalIntrinsic,
          PreferPrimordialsHint::GlobalIntrinsic,
        );
        return;
      }
    }

    if_chain! {
      if member_expr.parent().is::<ast_view::CallExpr>();
      if let ast_view::MemberProp::Ident(ident) = &member_expr.prop;
      if METHOD_TARGETS.contains(&ident.sym().as_ref());
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.range(),
          CODE,
          PreferPrimordialsMessage::GlobalIntrinsic,
          PreferPrimordialsHint::GlobalIntrinsic,
        );
      }
    }

    if_chain! {
      // Don't check left side of assignment expressions
      // e.g. `foo.bar = 1`
      if !matches!(member_expr.parent(), Node::AssignExpr(assign_expr)
        if assign_expr.left.to::<ast_view::MemberExpr>().map_or(false, |expr| ptr::eq(expr, member_expr))
      );
      // Don't check call expressions
      // e.g. `foo.bar()`
      if !member_expr.parent().is::<ast_view::CallExpr>();
      if let ast_view::MemberProp::Ident(ident) = &member_expr.prop;
      if GETTER_TARGETS.contains(&ident.sym().as_ref());
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.range(),
          CODE,
          PreferPrimordialsMessage::GlobalIntrinsic,
          PreferPrimordialsHint::GlobalIntrinsic,
        );
      }
    }
  }

  fn object_lit(
    &mut self,
    object_lit: &ast_view::ObjectLit,
    ctx: &mut Context,
  ) {
    fn inside_param(orig: ast_view::Node, node: ast_view::Node) -> bool {
      if let Some(param) = node.to::<ast_view::Param>() {
        return param.range().contains(&orig.range());
      }

      match node.parent() {
        None => false,
        Some(parent) => inside_param(orig, parent),
      }
    }

    if_chain! {
      if !is_null_proto(object_lit);
      if matches!(object_lit.parent(), ast_view::Node::AssignPat(_) | ast_view::Node::AssignPatProp(_));
      if inside_param(object_lit.as_node(), object_lit.as_node());
      then {
        ctx.add_diagnostic_with_hint(
          object_lit.range(),
          CODE,
          PreferPrimordialsMessage::ObjectAssignInDefaultParameter,
          PreferPrimordialsHint::NullPrototypeObjectLiteral,
        );
      }
    }
  }

  fn expr_or_spread(
    &mut self,
    expr_or_spread: &ast_view::ExprOrSpread,
    ctx: &mut Context,
  ) {
    if_chain! {
      if expr_or_spread.inner.spread.is_some();
      if !expr_or_spread.inner.expr.is_new();
      then {
        ctx.add_diagnostic_with_hint(
          expr_or_spread.range(),
          CODE,
          PreferPrimordialsMessage::Iterator,
          PreferPrimordialsHint::SafeIterator,
        );
      }
    }
  }

  fn for_of_stmt(
    &mut self,
    for_of_stmt: &ast_view::ForOfStmt,
    ctx: &mut Context,
  ) {
    if !for_of_stmt.right.is::<ast_view::NewExpr>() {
      ctx.add_diagnostic_with_hint(
        for_of_stmt.right.range(),
        CODE,
        PreferPrimordialsMessage::Iterator,
        PreferPrimordialsHint::SafeIterator,
      );
    }
  }

  fn yield_expr(
    &mut self,
    yield_expr: &ast_view::YieldExpr,
    ctx: &mut Context,
  ) {
    if yield_expr.delegate()
      && !matches!(yield_expr.arg, Some(ast_view::Expr::New(_)))
    {
      ctx.add_diagnostic_with_hint(
        yield_expr.range(),
        CODE,
        PreferPrimordialsMessage::Iterator,
        PreferPrimordialsHint::SafeIterator,
      );
    }
  }

  fn array_pat(&mut self, array_pat: &ast_view::ArrayPat, ctx: &mut Context) {
    use ast_view::{Expr, Node, Pat};

    // If array_pat.elems don't include rest pattern, should be used object pattern instead
    // For example:
    //
    // ```js
    // const [a, b] = [1, 2];
    // ```
    //
    // should be turned into:
    //
    // ```js
    // const { 0: a, 1: b } = [1, 2];
    // ```
    if !matches!(array_pat.elems.last(), Some(Some(Pat::Rest(_)))) {
      ctx.add_diagnostic_with_hint(
        array_pat.range(),
        CODE,
        PreferPrimordialsMessage::Iterator,
        PreferPrimordialsHint::ObjectPattern,
      );
      return;
    }

    match array_pat.parent() {
      Node::VarDeclarator(var_declarator) => {
        if !matches!(var_declarator.init, Some(Expr::New(_)) | None) {
          ctx.add_diagnostic_with_hint(
            var_declarator.range(),
            CODE,
            PreferPrimordialsMessage::Iterator,
            PreferPrimordialsHint::SafeIterator,
          );
        }
      }
      Node::AssignExpr(asssign_expr) => {
        if !matches!(asssign_expr.right, Expr::New(_)) {
          ctx.add_diagnostic_with_hint(
            asssign_expr.range(),
            CODE,
            PreferPrimordialsMessage::Iterator,
            PreferPrimordialsHint::SafeIterator,
          );
        }
      }
      // TODO(petamoriken): Support for deeply nested assignments
      _ => (),
    }
  }

  fn regex(&mut self, regex: &ast_view::Regex, ctx: &mut Context) {
    if !matches!(regex.parent(), ast_view::Node::ExprOrSpread(expr_or_spread)
      if expr_or_spread.parent().is::<ast_view::NewExpr>()
    ) {
      ctx.add_diagnostic_with_hint(
        regex.range(),
        CODE,
        PreferPrimordialsMessage::RegExp,
        PreferPrimordialsHint::SafeRegExp,
      );
    }
  }

  fn bin_expr(&mut self, bin_expr: &ast_view::BinExpr, ctx: &mut Context) {
    use ast_view::BinaryOp;

    if matches!(bin_expr.op(), BinaryOp::InstanceOf) {
      ctx.add_diagnostic_with_hint(
        bin_expr.range(),
        CODE,
        PreferPrimordialsMessage::InstanceOf,
        PreferPrimordialsHint::InstanceOf,
      );
    } else if matches!(bin_expr.op(), BinaryOp::In) {
      ctx.add_diagnostic_with_hint(
        bin_expr.range(),
        CODE,
        PreferPrimordialsMessage::In,
        PreferPrimordialsHint::In,
      );
    }
  }
}

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
ObjectDefineProperty(o, SymbolToStringTag, { __proto__: null, value: "o" });
      "#,
      r#"
const { ReflectDefineProperty, SymbolToStringTag } = primordials;
ReflectDefineProperty(o, SymbolToStringTag, { __proto__: null, value: "o" });
      "#,
      r#"
const { ObjectDefineProperties } = primordials;
ObjectDefineProperties(o, {
  foo: { __proto__: null, value: "o" },
  bar: { "__proto__": null, value: "o" },
});
      "#,
      r#"
function foo(o = { __proto__: null }) {}
function bar({ o = { __proto__: null } }) {}
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
const { SafeRegExp } = primordials;
new SafeRegExp("aaaa");
      "#,
      r#"
const { SafeMap } = primordials;
new SafeMap();
      "#,
      r#"
const { SafePromiseAll, PromiseResolve } = primordials;
SafePromiseAll([
  PromiseResolve(1),
  PromiseResolve(2),
]);
      "#,
      r#"
const { ArrayPrototypeMap } = primordials;
ArrayPrototypeMap([1, 2, 3], (val) => val * 2);
      "#,
      r#"
const parseInt = () => {};
parseInt();
      "#,
      r#"const foo = { Error: 1 };"#,
      r#"foo.description = 1"#,
      r#"foo.description()"#,
      r#"
const { SafeRegExp } = primordials;
const pattern = new SafeRegExp(/aaaa/u);
pattern.source;
      "#,
      r#"
const { SafeSet } = primordials;
const set = new SafeSet();
set.add(1);
set.add(2);
set.size;
      "#,
      r#"
const foo = { size: 100 };
foo.size;
      "#,
      r#"
const { SafeArrayIterator } = primordials;
[1, 2, ...new SafeArrayIterator(arr)];
foo(1, 2, ...new SafeArrayIterator(arr));
new Foo(1, 2, ...new SafeArrayIterator(arr));
      "#,
      r#"
const { SafeArrayIterator } = primordials;
[1, 2, ...new SafeArrayIterator([1, 2, 3])];
foo(1, 2, ...new SafeArrayIterator([1, 2, 3]));
new Foo(1, 2, ...new SafeArrayIterator([1, 2, 3]));
      "#,
      r#"
({ ...{} });
      "#,
      r#"
const { SafeArrayIterator } = primordials;
for (const val of new SafeArrayIterator(arr)) {}
for (const val of new SafeArrayIterator([1, 2, 3])) {}
      "#,
      r#"
const { SafeArrayIterator } = primordials;
function* foo() { yield* new SafeArrayIterator([1, 2, 3]); }
      "#,
      r#"
const { 0: a, 1: b } = [1, 2];
      "#,
      r#"
let a, b;
({ 0: a, 1: b } = [1, 2]);
      "#,
      r#"
const { SafeArrayIterator } = primordials;
const [a, b, ...c] = new SafeArrayIterator([1, 2, 3]);
      "#,
      r#"
const { SafeArrayIterator } = primordials;
let a, b, c;
[a, b, ...c] = new SafeArrayIterator([1, 2, 3]);
      "#,
      r#"
const { indirectEval } = primordials;
indirectEval("console.log('This test should pass.');");
      "#,
      r#"
function foo(a: Array<any>) {}
      "#,
      r#"
function foo(): Array<any> {}
      "#,
      r#"
type p = Promise<void>;
      "#,
    };
  }

  #[test]
  fn prefer_primordials_invalid() {
    assert_lint_err! {
      PreferPrimordials,
      r#"const foo = Symbol("foo");"#: [
        {
          col: 12,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"const foo = Symbol.for("foo");"#: [
        {
          col: 12,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"const arr = new Array();"#: [
        {
          col: 16,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { RegExp } = primordials;
new RegExp("aaaa");
      "#: [
        {
          line: 3,
          col: 4,
          message: PreferPrimordialsMessage::UnsafeIntrinsic,
          hint: PreferPrimordialsHint::UnsafeIntrinsic,
        },
      ],
      r#"
const { Map } = primordials;
new Map();
      "#: [
        {
          line: 3,
          col: 4,
          message: PreferPrimordialsMessage::UnsafeIntrinsic,
          hint: PreferPrimordialsHint::UnsafeIntrinsic,
        },
      ],
      r#"
const { PromiseAll, PromiseResolve } = primordials;
PromiseAll([
  PromiseResolve(1),
  PromiseResolve(2),
]);
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::UnsafeIntrinsic,
          hint: PreferPrimordialsHint::UnsafeIntrinsic,
        },
      ],
      r#"JSON.parse("{}")"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { JSON } = primordials;
JSON.parse("{}");
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"Symbol.for("foo")"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { Symbol } = primordials;
Symbol.for("foo");
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { Symbol } = primordials;
class A {
  *[Symbol.iterator] () {
    yield "a";
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { Symbol } = primordials;
const a = {
  *[Symbol.iterator] () {
    yield "a";
  }
}
      "#: [
        {
          line: 4,
          col: 4,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { ObjectDefineProperty, Symbol } = primordials;
ObjectDefineProperty(o, Symbol.toStringTag, { value: "o" });
      "#: [
        {
          line: 3,
          col: 24,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
        {
          line: 3,
          col: 44,
          message: PreferPrimordialsMessage::DefineProperty,
          hint: PreferPrimordialsHint::NullPrototypeObjectLiteral,
        },
      ],
      r#"
const { ObjectDefineProperty, SymbolToStringTag } = primordials;
ObjectDefineProperty(o, SymbolToStringTag, { value: "o" });
      "#: [
        {
          line: 3,
          col: 43,
          message: PreferPrimordialsMessage::DefineProperty,
          hint: PreferPrimordialsHint::NullPrototypeObjectLiteral,
        },
      ],
      r#"
const { ObjectDefineProperties } = primordials;
ObjectDefineProperties(o, {
  foo: { value: "o" },
  bar: { __proto__: {}, value: "o" },
  baz: { ["__proto__"]: null, value: "o" },
});
      "#: [
        {
          line: 4,
          col: 7,
          message: PreferPrimordialsMessage::DefineProperty,
          hint: PreferPrimordialsHint::NullPrototypeObjectLiteral,
        },
        {
          line: 5,
          col: 7,
          message: PreferPrimordialsMessage::DefineProperty,
          hint: PreferPrimordialsHint::NullPrototypeObjectLiteral,
        },
        {
          line: 6,
          col: 7,
          message: PreferPrimordialsMessage::DefineProperty,
          hint: PreferPrimordialsHint::NullPrototypeObjectLiteral,
        },
      ],
      r#"
function foo(o = {}) {}
function bar({ o = {} }) {}
      "#: [
        {
          line: 2,
          col: 17,
          message: PreferPrimordialsMessage::ObjectAssignInDefaultParameter,
          hint: PreferPrimordialsHint::NullPrototypeObjectLiteral,
        },
        {
          line: 3,
          col: 19,
          message: PreferPrimordialsMessage::ObjectAssignInDefaultParameter,
          hint: PreferPrimordialsHint::NullPrototypeObjectLiteral,
        }
      ],
      r#"
const { Number } = primordials;
Number.parseInt("10");
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"parseInt("10")"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"const { ownKeys } = Reflect;"#: [
        {
          col: 20,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"new Map();"#: [
        {
          col: 4,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
        {
          col: 4,
          message: PreferPrimordialsMessage::UnsafeIntrinsic,
          hint: PreferPrimordialsHint::UnsafeIntrinsic,
        },
      ],
      r#"
const { Function } = primordials;
const noop = Function.prototype;
      "#: [
        {
          line: 3,
          col: 13,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"[1, 2, 3].map(val => val * 2);"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const obj = { foo: 1 };
obj.hasOwnProperty("foo");
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const fn = () => 1;
fn.call(null);
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const num = 123.456;
num.toFixed(2);
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { Date } = primordials;
new Date().toISOString();
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const arr = [1, 2, 3, 4];
arr.filter((val) => val % 2 === 0);
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const str = "foo bar baz";
str.split(" ");
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const thenable = { then() {} };
thenable.then(() => {});
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { Uint8Array } = primordials;
new Uint8Array(10).buffer;
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { ArrayBuffer } = primordials;
new ArrayBuffer(10).byteLength;
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"
const { ArrayBuffer, DataView } = primordials;
new DataView(new ArrayBuffer(10)).byteOffset;
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#"foo = bar.description;"#: [
        {
          col: 6,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
      r#""a" in A"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::In,
          hint: PreferPrimordialsHint::In,
        },
      ],
      r#"a instanceof A"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::InstanceOf,
          hint: PreferPrimordialsHint::InstanceOf,
        },
      ],
      r#"[1, 2, ...arr];"#: [
        {
          col: 7,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"foo(1, 2, ...arr);"#: [
        {
          col: 10,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"new Foo(1, 2, ...arr);"#: [
        {
          col: 14,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"[1, 2, ...[3]];"#: [
        {
          col: 7,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"foo(1, 2, ...[3]);"#: [
        {
          col: 10,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"new Foo(1, 2, ...[3]);"#: [
        {
          col: 14,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"for (const val of arr) {}"#: [
        {
          col: 18,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"for (const val of [1, 2, 3]) {}"#: [
        {
          col: 18,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"function* foo() { yield* [1, 2, 3]; }"#: [
        {
          col: 18,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        }
      ],
      r#"const [a, b] = [1, 2];"#: [
        {
          col: 6,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::ObjectPattern,
        },
      ],
      r#"
let a, b;
[a, b] = [1, 2];
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::ObjectPattern,
        },
      ],
      r#"const [a, b, ...c] = [1, 2, 3];"#: [
        {
          col: 6,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"
let a, b, c;
[a, b, ...c] = [1, 2, 3];
      "#: [
        {
          line: 3,
          col: 0,
          message: PreferPrimordialsMessage::Iterator,
          hint: PreferPrimordialsHint::SafeIterator,
        },
      ],
      r#"/aaa/u"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::RegExp,
          hint: PreferPrimordialsHint::SafeRegExp,
        }
      ],
      r#"eval("console.log('This test should fail!');");"#: [
        {
          col: 0,
          message: PreferPrimordialsMessage::GlobalIntrinsic,
          hint: PreferPrimordialsHint::GlobalIntrinsic,
        },
      ],
    }
  }
}
