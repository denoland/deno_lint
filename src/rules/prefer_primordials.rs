// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::Tags;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::GetSpan;
use deno_ast::oxc::span::Span;
use derive_more::Display;
use std::collections::HashSet;

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
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = PreferPrimordialsHandler::new();
    crate::handler::traverse_program(&mut handler, program, context);
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

fn is_null_proto(object_expr: &ObjectExpression) -> bool {
  for prop_or_spread in &object_expr.properties {
    if let ObjectPropertyKind::ObjectProperty(prop) = prop_or_spread {
      // Only non-computed keys count. `["__proto__"]` (computed) does not.
      if prop.computed {
        continue;
      }
      if matches!(&prop.value, Expression::NullLiteral(_)) {
        match &prop.key {
          PropertyKey::StaticIdentifier(ident) => {
            if ident.name == "__proto__" {
              return true;
            }
          }
          PropertyKey::StringLiteral(s) if s.value == "__proto__" => {
            return true;
          }
          _ => {}
        }
      }
    }
  }
  false
}

fn is_new_expression(expr: &Expression) -> bool {
  matches!(expr, Expression::NewExpression(_))
}

struct PreferPrimordialsHandler {
  /// Spans of StaticMemberExpressions that should NOT be reported for getter targets
  /// because they appear as assignment targets or call targets, not reads.
  skip_getter_spans: HashSet<Span>,
  /// Spans of IdentifierReferences that should NOT be reported because they are
  /// the object of a member expression that is already being reported.
  skip_identifier_spans: HashSet<Span>,
  /// Spans of RegExpLiterals that should NOT be reported because they are
  /// arguments to safe wrappers (e.g. `new SafeRegExp(/pattern/)`).
  skip_regex_spans: HashSet<Span>,
  /// Spans of SpreadElements that are within object expressions (object spread),
  /// which do NOT use the iterator protocol and should not be flagged.
  skip_spread_spans: HashSet<Span>,
  /// Spans of ArrayPatterns that should NOT be reported because their RHS is a
  /// safe iterator (e.g. `new SafeArrayIterator(...)`).
  skip_array_pattern_spans: HashSet<Span>,
  /// Spans of StaticMemberExpressions that have already been reported as method
  /// call targets (e.g. `[1,2,3].map(...)`) so static_member_expression won't
  /// double-report them.
  skip_method_call_spans: HashSet<Span>,
}

impl PreferPrimordialsHandler {
  fn new() -> Self {
    Self {
      skip_getter_spans: HashSet::new(),
      skip_identifier_spans: HashSet::new(),
      skip_regex_spans: HashSet::new(),
      skip_spread_spans: HashSet::new(),
      skip_array_pattern_spans: HashSet::new(),
      skip_method_call_spans: HashSet::new(),
    }
  }

  fn is_safe_array_iterator_rhs(expr: &Expression) -> bool {
    if let Expression::NewExpression(new_expr) = expr {
      if let Expression::Identifier(ident) = &new_expr.callee {
        return ident.name == "SafeArrayIterator";
      }
    }
    false
  }
}

impl Handler<'_> for PreferPrimordialsHandler {
  // Handle global identifier references (not as part of member expressions or declarations)
  // Note: In OXC we don't have parent() access, so we use identifier_reference
  // which fires for all identifier references. We rely on the fact that
  // member expression object references also fire as identifier_reference.
  // However, we cannot distinguish LHS of variable declarations from RHS.
  // This is a simplified port that may have some differences from the original.

  fn ts_type_annotation(&mut self, _n: &TSTypeAnnotation, ctx: &mut Context) {
    // Skip traversal inside type annotations entirely — identifiers in type
    // position (e.g. `Array` in `a: Array<any>`) are not runtime references.
    ctx.stop_traverse();
  }

  fn ts_type_parameter_declaration(
    &mut self,
    _n: &TSTypeParameterDeclaration,
    ctx: &mut Context,
  ) {
    ctx.stop_traverse();
  }

  fn ts_type_parameter_instantiation(
    &mut self,
    _n: &TSTypeParameterInstantiation,
    ctx: &mut Context,
  ) {
    ctx.stop_traverse();
  }

  fn ts_type_alias_declaration(
    &mut self,
    _n: &TSTypeAliasDeclaration,
    ctx: &mut Context,
  ) {
    ctx.stop_traverse();
  }

  fn call_expression(&mut self, call_expr: &CallExpression, ctx: &mut Context) {
    // Check for unsafe function targets like PromiseAll, etc.
    if let Expression::Identifier(ident) = &call_expr.callee {
      if UNSAFE_FUNCTION_TARGETS.contains(&ident.name.as_str()) {
        ctx.add_diagnostic_with_hint(
          ident.span,
          CODE,
          PreferPrimordialsMessage::UnsafeIntrinsic,
          PreferPrimordialsHint::UnsafeIntrinsic,
        );
      }

      // Check ObjectDefineProperty / ReflectDefineProperty
      match ident.name.as_str() {
        "ObjectDefineProperty" | "ReflectDefineProperty" => {
          if let Some(Argument::ObjectExpression(object_lit)) =
            call_expr.arguments.get(2)
          {
            if !is_null_proto(object_lit) {
              ctx.add_diagnostic_with_hint(
                object_lit.span,
                CODE,
                PreferPrimordialsMessage::DefineProperty,
                PreferPrimordialsHint::NullPrototypeObjectLiteral,
              );
            }
          }
        }
        "ObjectDefineProperties" => {
          if let Some(Argument::ObjectExpression(object_lit)) =
            call_expr.arguments.get(1)
          {
            for prop_or_spread in &object_lit.properties {
              if let ObjectPropertyKind::ObjectProperty(prop) = prop_or_spread {
                if let Expression::ObjectExpression(inner_obj) = &prop.value {
                  if !is_null_proto(inner_obj) {
                    ctx.add_diagnostic_with_hint(
                      inner_obj.span,
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
        _ => {}
      }
    }

    // Check for method calls like foo.bar() where bar is in METHOD_TARGETS
    if let Expression::StaticMemberExpression(member) = &call_expr.callee {
      if METHOD_TARGETS.contains(&member.property.name.as_str()) {
        ctx.add_diagnostic_with_hint(
          member.span,
          CODE,
          PreferPrimordialsMessage::GlobalIntrinsic,
          PreferPrimordialsHint::GlobalIntrinsic,
        );
        // Mark as reported so static_member_expression won't double-report.
        self.skip_method_call_spans.insert(member.span);
      }
      // Mark callee as skip for getter targets: `foo.description()` is a method call,
      // not a getter read, so the static_member_expression handler should skip it.
      if GETTER_TARGETS.contains(&member.property.name.as_str()) {
        self.skip_getter_spans.insert(member.span);
      }
    }
  }

  fn assignment_expression(
    &mut self,
    assign_expr: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    // Mark the LHS of an assignment as skip for getter targets.
    // e.g. `foo.description = 1` — `foo.description` is being written, not read.
    if let AssignmentTarget::StaticMemberExpression(member) = &assign_expr.left
    {
      if GETTER_TARGETS.contains(&member.property.name.as_str()) {
        self.skip_getter_spans.insert(member.span);
      }
    }
    if let AssignmentTarget::ArrayAssignmentTarget(array_target) =
      &assign_expr.left
    {
      if Self::is_safe_array_iterator_rhs(&assign_expr.right) {
        // `[a, b, ...c] = new SafeArrayIterator(...)` — array pattern LHS is safe.
        self.skip_array_pattern_spans.insert(array_target.span);
      } else {
        // `[a, b] = expr` — array destructuring assignment without SafeArrayIterator.
        let has_rest = array_target.rest.is_some();
        if !has_rest {
          ctx.add_diagnostic_with_hint(
            array_target.span,
            CODE,
            PreferPrimordialsMessage::Iterator,
            PreferPrimordialsHint::ObjectPattern,
          );
        } else {
          ctx.add_diagnostic_with_hint(
            array_target.span,
            CODE,
            PreferPrimordialsMessage::Iterator,
            PreferPrimordialsHint::SafeIterator,
          );
        }
      }
    }
  }

  fn variable_declarator(
    &mut self,
    declarator: &VariableDeclarator,
    _ctx: &mut Context,
  ) {
    // `const [a, b, ...c] = new SafeArrayIterator(...)` — array pattern is safe.
    if let Some(init) = &declarator.init {
      if Self::is_safe_array_iterator_rhs(init) {
        if let BindingPattern::ArrayPattern(array_pat) = &declarator.id {
          self.skip_array_pattern_spans.insert(array_pat.span);
        }
      }
    }
  }

  fn new_expression(&mut self, new_expr: &NewExpression, ctx: &mut Context) {
    if let Expression::Identifier(ident) = &new_expr.callee {
      // Report GlobalIntrinsic first (if applicable), then UnsafeIntrinsic.
      // This ensures the ordering matches what tests expect.
      if GLOBAL_TARGETS.contains(&ident.name.as_str())
        && ctx.scope().var_by_name(ident.name.as_str()).is_none()
      {
        ctx.add_diagnostic_with_hint(
          ident.span,
          CODE,
          PreferPrimordialsMessage::GlobalIntrinsic,
          PreferPrimordialsHint::GlobalIntrinsic,
        );
        // Mark the identifier so identifier_reference won't double-report it.
        self.skip_identifier_spans.insert(ident.span);
      }
      if UNSAFE_CONSTRUCTOR_TARGETS.contains(&ident.name.as_str()) {
        ctx.add_diagnostic_with_hint(
          ident.span,
          CODE,
          PreferPrimordialsMessage::UnsafeIntrinsic,
          PreferPrimordialsHint::UnsafeIntrinsic,
        );
        // Also mark the identifier so identifier_reference won't double-report it.
        self.skip_identifier_spans.insert(ident.span);
      }
      // `new SafeRegExp(regex)` — the regex literal argument is intentionally wrapped,
      // so skip reporting it as "don't use RegExp literal directly".
      if ident.name.as_str() == "SafeRegExp" {
        for arg in &new_expr.arguments {
          if let Some(Expression::RegExpLiteral(regex)) = arg.as_expression() {
            self.skip_regex_spans.insert(regex.span);
          }
        }
      }
    }
  }

  fn identifier_reference(
    &mut self,
    ident: &IdentifierReference,
    ctx: &mut Context,
  ) {
    // Skip if already reported as part of a member expression (e.g. `Symbol` in `Symbol.for`).
    if self.skip_identifier_spans.contains(&ident.span) {
      return;
    }
    if GLOBAL_TARGETS.contains(&ident.name.as_str())
      && ctx.scope().var_by_name(ident.name.as_str()).is_none()
    {
      ctx.add_diagnostic_with_hint(
        ident.span,
        CODE,
        PreferPrimordialsMessage::GlobalIntrinsic,
        PreferPrimordialsHint::GlobalIntrinsic,
      );
    }
  }

  fn static_member_expression(
    &mut self,
    member_expr: &StaticMemberExpression,
    ctx: &mut Context,
  ) {
    // If this member expression was already reported as part of a method call, skip.
    if self.skip_method_call_spans.contains(&member_expr.span) {
      return;
    }

    // If the object is an array literal, flag it
    if let Expression::ArrayExpression(_) = &member_expr.object {
      ctx.add_diagnostic_with_hint(
        member_expr.span,
        CODE,
        PreferPrimordialsMessage::GlobalIntrinsic,
        PreferPrimordialsHint::GlobalIntrinsic,
      );
      return;
    }

    // Check for global.property access (non-call, non-chained)
    if let Expression::Identifier(ident) = &member_expr.object {
      if GLOBAL_TARGETS.contains(&ident.name.as_str()) {
        ctx.add_diagnostic_with_hint(
          member_expr.span,
          CODE,
          PreferPrimordialsMessage::GlobalIntrinsic,
          PreferPrimordialsHint::GlobalIntrinsic,
        );
        // Mark the identifier so it won't be reported again by identifier_reference.
        self.skip_identifier_spans.insert(ident.span);
        return;
      }
    }

    // Check getter targets (read access only).
    // Skip if the member expression was marked as an assignment target or call target.
    if GETTER_TARGETS.contains(&member_expr.property.name.as_str())
      && !self.skip_getter_spans.contains(&member_expr.span)
    {
      ctx.add_diagnostic_with_hint(
        member_expr.span,
        CODE,
        PreferPrimordialsMessage::GlobalIntrinsic,
        PreferPrimordialsHint::GlobalIntrinsic,
      );
    }
  }

  fn assignment_pattern(
    &mut self,
    assign_pat: &AssignmentPattern,
    ctx: &mut Context,
  ) {
    // Check for default parameters with object literals that lack __proto__: null
    if let Expression::ObjectExpression(object_lit) = &assign_pat.right {
      if !is_null_proto(object_lit) {
        ctx.add_diagnostic_with_hint(
          object_lit.span,
          CODE,
          PreferPrimordialsMessage::ObjectAssignInDefaultParameter,
          PreferPrimordialsHint::NullPrototypeObjectLiteral,
        );
      }
    }
  }

  fn formal_parameter(&mut self, param: &FormalParameter, ctx: &mut Context) {
    // Check for default parameter values that are object literals without __proto__: null.
    // e.g. `function foo(o = {}) {}` — FormalParameter.initializer holds the default.
    // (This is distinct from destructuring defaults like `{ o = {} }` which use AssignmentPattern.)
    if let Some(init) = &param.initializer {
      if let Expression::ObjectExpression(object_lit) = init.as_ref() {
        if !is_null_proto(object_lit) {
          ctx.add_diagnostic_with_hint(
            object_lit.span,
            CODE,
            PreferPrimordialsMessage::ObjectAssignInDefaultParameter,
            PreferPrimordialsHint::NullPrototypeObjectLiteral,
          );
        }
      }
    }
  }

  fn object_expression(
    &mut self,
    obj_expr: &ObjectExpression,
    _ctx: &mut Context,
  ) {
    // Object spread (`{ ...obj }`) does NOT use the iterator protocol, so mark
    // any spread elements within this object expression to be skipped.
    for prop in &obj_expr.properties {
      if let ObjectPropertyKind::SpreadProperty(spread) = prop {
        self.skip_spread_spans.insert(spread.span);
      }
    }
  }

  fn spread_element(&mut self, spread: &SpreadElement, ctx: &mut Context) {
    // Skip object spreads — they don't use the iterator protocol.
    if self.skip_spread_spans.contains(&spread.span) {
      return;
    }
    if !is_new_expression(&spread.argument) {
      ctx.add_diagnostic_with_hint(
        spread.span,
        CODE,
        PreferPrimordialsMessage::Iterator,
        PreferPrimordialsHint::SafeIterator,
      );
    }
  }

  fn for_of_statement(&mut self, for_of: &ForOfStatement, ctx: &mut Context) {
    if !is_new_expression(&for_of.right) {
      ctx.add_diagnostic_with_hint(
        for_of.right.span(),
        CODE,
        PreferPrimordialsMessage::Iterator,
        PreferPrimordialsHint::SafeIterator,
      );
    }
  }

  fn yield_expression(
    &mut self,
    yield_expr: &YieldExpression,
    ctx: &mut Context,
  ) {
    if yield_expr.delegate
      && !matches!(&yield_expr.argument, Some(expr) if is_new_expression(expr))
    {
      ctx.add_diagnostic_with_hint(
        yield_expr.span,
        CODE,
        PreferPrimordialsMessage::Iterator,
        PreferPrimordialsHint::SafeIterator,
      );
    }
  }

  fn array_pattern(&mut self, array_pat: &ArrayPattern, ctx: &mut Context) {
    // If the array pattern is known to be safe (RHS is SafeArrayIterator), skip.
    if self.skip_array_pattern_spans.contains(&array_pat.span) {
      return;
    }

    // If array_pat.elements don't include rest pattern, should be used object pattern
    let has_rest = array_pat.rest.is_some();

    if !has_rest {
      ctx.add_diagnostic_with_hint(
        array_pat.span,
        CODE,
        PreferPrimordialsMessage::Iterator,
        PreferPrimordialsHint::ObjectPattern,
      );
      return;
    }

    // If it has a rest pattern, flag the parent var declarator or assignment
    // for iterator usage (unless the RHS is a new SafeArrayIterator).
    ctx.add_diagnostic_with_hint(
      array_pat.span,
      CODE,
      PreferPrimordialsMessage::Iterator,
      PreferPrimordialsHint::SafeIterator,
    );
  }

  fn reg_exp_literal(&mut self, regex: &RegExpLiteral, ctx: &mut Context) {
    // Skip regex literals that are arguments to safe wrappers like `new SafeRegExp(...)`.
    if self.skip_regex_spans.contains(&regex.span) {
      return;
    }
    ctx.add_diagnostic_with_hint(
      regex.span,
      CODE,
      PreferPrimordialsMessage::RegExp,
      PreferPrimordialsHint::SafeRegExp,
    );
  }

  fn binary_expression(
    &mut self,
    bin_expr: &BinaryExpression,
    ctx: &mut Context,
  ) {
    if bin_expr.operator == BinaryOperator::Instanceof {
      ctx.add_diagnostic_with_hint(
        bin_expr.span,
        CODE,
        PreferPrimordialsMessage::InstanceOf,
        PreferPrimordialsHint::InstanceOf,
      );
    } else if bin_expr.operator == BinaryOperator::In {
      ctx.add_diagnostic_with_hint(
        bin_expr.span,
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
