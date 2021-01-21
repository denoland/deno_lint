// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;

use derive_more::Display;
use swc_ecmascript::ast::{
  ArrowExpr, CallExpr, ClassProp, Expr, ExprOrSuper, Function, Ident, Lit,
  NewExpr, OptChainExpr, Pat, PrivateProp, Program, TsEntityName,
  TsKeywordType, TsKeywordTypeKind, TsType, TsTypeAnn, TsTypeRef, UnaryExpr,
  VarDecl,
};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoInferrableTypes;

const CODE: &str = "no-inferrable-types";

#[derive(Display)]
enum NoInferrableTypesMessage {
  #[display(fmt = "inferrable types are not allowed")]
  NotAllowed,
}

#[derive(Display)]
enum NoInferrableTypesHint {
  #[display(fmt = "Remove the type, it is easily inferrable")]
  Remove,
}

impl LintRule for NoInferrableTypes {
  fn new() -> Box<Self> {
    Box::new(NoInferrableTypes)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = NoInferrableTypesVisitor::new(context);
    program.visit_all_with(program, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows easily inferrable types

Variable initializations to javascript primitives (and null) are obvious
in their type.  Specifying their type can add additional verbosity to the code.
For example, with `const x: number = 5`, specifying `number` is unnecessary as
it is obvious that `5` is a number.
    
### Invalid:
```typescript
const a: bigint = 10n;
const b: bigint = BigInt(10);
const c: boolean = true;
const d: boolean = !0;
const e: number = 10;
const f: number = Number('1');
const g: number = Infinity;
const h: number = NaN;
const i: null = null;
const j: RegExp = /a/;
const k: RegExp = RegExp('a');
const l: RegExp = new RegExp('a');
const m: string = 'str';
const n: string = `str`;
const o: string = String(1);
const p: symbol = Symbol('a');
const q: undefined = undefined;
const r: undefined = void someValue;

class Foo {
  prop: number = 5;
}

function fn(s: number = 5, t: boolean = true) {}
```

### Valid:
```typescript
const a = 10n;
const b = BigInt(10);
const c = true;
const d = !0;
const e = 10;
const f = Number('1');
const g = Infinity;
const h = NaN;
const i = null;
const j = /a/;
const k = RegExp('a');
const l = new RegExp('a');
const m = 'str';
const n = `str`;
const o = String(1);
const p = Symbol('a');
const q = undefined;
const r = void someValue;

class Foo {
  prop = 5;
}

function fn(s = 5, t = true) {}
```
"#
  }
}

struct NoInferrableTypesVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoInferrableTypesVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic_helper(&mut self, span: swc_common::Span) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoInferrableTypesMessage::NotAllowed,
      NoInferrableTypesHint::Remove,
    )
  }

  fn check_callee(
    &mut self,
    callee: &ExprOrSuper,
    span: swc_common::Span,
    expected_sym: &str,
  ) {
    if let ExprOrSuper::Expr(unboxed) = &callee {
      if let Expr::Ident(value) = &**unboxed {
        if value.sym == *expected_sym {
          self.add_diagnostic_helper(span);
        }
      }
    }
  }

  fn is_nan_or_infinity(&self, ident: &Ident) -> bool {
    ident.sym == *"NaN" || ident.sym == *"Infinity"
  }

  fn check_keyword_type(
    &mut self,
    value: &Expr,
    ts_type: &TsKeywordType,
    span: swc_common::Span,
  ) {
    use TsKeywordTypeKind::*;
    match ts_type.kind {
      TsBigIntKeyword => match &*value {
        Expr::Lit(Lit::BigInt(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "BigInt");
        }
        Expr::Unary(UnaryExpr { arg, .. }) => match &**arg {
          Expr::Lit(Lit::BigInt(_)) => {
            self.add_diagnostic_helper(span);
          }
          Expr::Call(CallExpr { callee, .. }) => {
            self.check_callee(callee, span, "BigInt");
          }
          Expr::OptChain(OptChainExpr { expr, .. }) => {
            if let Expr::Call(CallExpr { callee, .. }) = &**expr {
              self.check_callee(callee, span, "BigInt");
            }
          }
          _ => {}
        },
        Expr::OptChain(OptChainExpr { expr, .. }) => {
          if let Expr::Call(CallExpr { callee, .. }) = &**expr {
            self.check_callee(callee, span, "BigInt");
          }
        }
        _ => {}
      },
      TsBooleanKeyword => match &*value {
        Expr::Lit(Lit::Bool(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "Boolean");
        }
        Expr::Unary(UnaryExpr { op, .. }) => {
          if op.to_string() == "!" {
            self.add_diagnostic_helper(span);
          }
        }
        Expr::OptChain(OptChainExpr { expr, .. }) => {
          if let Expr::Call(CallExpr { callee, .. }) = &**expr {
            self.check_callee(callee, span, "Boolean");
          }
        }
        _ => {}
      },
      TsNumberKeyword => match &*value {
        Expr::Lit(Lit::Num(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "Number");
        }
        Expr::Ident(ident) => {
          if self.is_nan_or_infinity(&ident) {
            self.add_diagnostic_helper(span);
          }
        }
        Expr::Unary(UnaryExpr { arg, .. }) => match &**arg {
          Expr::Lit(Lit::Num(_)) => {
            self.add_diagnostic_helper(span);
          }
          Expr::Call(CallExpr { callee, .. }) => {
            self.check_callee(callee, span, "Number");
          }
          Expr::Ident(ident) => {
            if self.is_nan_or_infinity(&ident) {
              self.add_diagnostic_helper(span);
            }
          }
          Expr::OptChain(OptChainExpr { expr, .. }) => {
            if let Expr::Call(CallExpr { callee, .. }) = &**expr {
              self.check_callee(callee, span, "Number");
            }
          }
          _ => {}
        },
        Expr::OptChain(OptChainExpr { expr, .. }) => {
          if let Expr::Call(CallExpr { callee, .. }) = &**expr {
            self.check_callee(callee, span, "Number");
          }
        }
        _ => {}
      },
      TsNullKeyword => {
        if let Expr::Lit(Lit::Null(_)) = &*value {
          self.add_diagnostic_helper(span);
        }
      }
      TsStringKeyword => match &*value {
        Expr::Lit(Lit::Str(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Tpl(_) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "String");
        }
        Expr::OptChain(OptChainExpr { expr, .. }) => {
          if let Expr::Call(CallExpr { callee, .. }) = &**expr {
            self.check_callee(callee, span, "String");
          }
        }
        _ => {}
      },
      TsSymbolKeyword => {
        if let Expr::Call(CallExpr { callee, .. }) = &*value {
          self.check_callee(callee, span, "Symbol");
        } else if let Expr::OptChain(OptChainExpr { expr, .. }) = &*value {
          if let Expr::Call(CallExpr { callee, .. }) = &**expr {
            self.check_callee(callee, span, "Symbol");
          }
        }
      }
      TsUndefinedKeyword => match &*value {
        Expr::Ident(ident) => {
          if ident.sym == *"undefined" {
            self.add_diagnostic_helper(span);
          }
        }
        Expr::Unary(UnaryExpr { op, .. }) => {
          if op.to_string() == "void" {
            self.add_diagnostic_helper(span);
          }
        }
        _ => {}
      },
      _ => {}
    }
  }

  fn check_ref_type(
    &mut self,
    value: &Expr,
    ts_type: &TsTypeRef,
    span: swc_common::Span,
  ) {
    if let TsEntityName::Ident(ident) = &ts_type.type_name {
      if ident.sym != *"RegExp" {
        return;
      }
      match &*value {
        Expr::Lit(Lit::Regex(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "RegExp");
        }
        Expr::New(NewExpr { callee, .. }) => {
          if let Expr::Ident(ident) = &**callee {
            if ident.sym == *"RegExp" {
              self.add_diagnostic_helper(span);
            }
          } else if let Expr::OptChain(OptChainExpr { expr, .. }) = &**callee {
            if let Expr::Call(CallExpr { callee, .. }) = &**expr {
              self.check_callee(callee, span, "RegExp");
            }
          }
        }
        Expr::OptChain(OptChainExpr { expr, .. }) => {
          if let Expr::Call(CallExpr { callee, .. }) = &**expr {
            self.check_callee(callee, span, "RegExp");
          }
        }
        _ => {}
      }
    }
  }

  fn check_ts_type(
    &mut self,
    value: &Expr,
    ts_type: &TsTypeAnn,
    span: swc_common::Span,
  ) {
    if let TsType::TsKeywordType(ts_type) = &*ts_type.type_ann {
      self.check_keyword_type(&value, ts_type, span);
    } else if let TsType::TsTypeRef(ts_type) = &*ts_type.type_ann {
      self.check_ref_type(&value, ts_type, span);
    }
  }
}

impl<'c> VisitAll for NoInferrableTypesVisitor<'c> {
  fn visit_function(&mut self, function: &Function, _: &dyn Node) {
    for param in &function.params {
      if let Pat::Assign(assign_pat) = &param.pat {
        if let Pat::Ident(ident) = &*assign_pat.left {
          if let Some(ident_type_ann) = &ident.type_ann {
            self.check_ts_type(&assign_pat.right, ident_type_ann, param.span);
          }
        }
      }
    }
  }

  fn visit_arrow_expr(&mut self, arr_expr: &ArrowExpr, _: &dyn Node) {
    for param in &arr_expr.params {
      if let Pat::Assign(assign_pat) = &param {
        if let Pat::Ident(ident) = &*assign_pat.left {
          if let Some(ident_type_ann) = &ident.type_ann {
            self.check_ts_type(
              &assign_pat.right,
              ident_type_ann,
              assign_pat.span,
            );
          }
        }
      }
    }
  }

  fn visit_class_prop(&mut self, prop: &ClassProp, _: &dyn Node) {
    if prop.readonly || prop.is_optional {
      return;
    }
    if let Some(init) = &prop.value {
      if let Expr::Ident(_) = &*prop.key {
        if let Some(ident_type_ann) = &prop.type_ann {
          self.check_ts_type(init, ident_type_ann, prop.span);
        }
      }
    }
  }

  fn visit_private_prop(&mut self, prop: &PrivateProp, _: &dyn Node) {
    if prop.readonly || prop.is_optional {
      return;
    }
    if let Some(init) = &prop.value {
      if let Some(ident_type_ann) = &prop.type_ann {
        self.check_ts_type(init, ident_type_ann, prop.span);
      }
    }
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _: &dyn Node) {
    for decl in &var_decl.decls {
      if let Some(init) = &decl.init {
        if let Pat::Ident(ident) = &decl.name {
          if let Some(ident_type_ann) = &ident.type_ann {
            self.check_ts_type(init, ident_type_ann, decl.span);
          }
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_inferrable_types_valid() {
    assert_lint_ok! {
      NoInferrableTypes,
      "const a = 10n",
      "const a = -10n",
      "const a = BigInt(10)",
      "const a = -BigInt?.(10)",
      "const a = +BigInt?.(10)",
      "const a = false",
      "const a = true",
      "const a = Boolean(true)",
      "const a = Boolean(null)",
      "const a = Boolean?.(null)",
      "const a = !0",
      "const a = 10",
      "const a = +10",
      "const a = -10",
      "const a = Number('1')",
      "const a = +Number('1')",
      "const a = -Number('1')",
      "const a = Number?.('1')",
      "const a = +Number?.('1')",
      "const a = -Number?.('1')",
      "const a = Infinity",
      "const a = +Infinity",
      "const a = -Infinity",
      "const a = NaN",
      "const a = +NaN",
      "const a = -NaN",
      "const a = null",
      "const a = /a/",
      "const a = RegExp('a')",
      "const a = RegExp?.('a')",
      "const a = new RegExp?.('a')",
      "const a = 'str'",
      r#"const a = "str""#,
      "const a = `str`",
      "const a = String(1)",
      "const a = String?.(1)",
      "const a = Symbol('a')",
      "const a = Symbol?.('a')",
      "const a = undefined",
      "const a = void someValue",
      "const fn = (a = 5, b = true, c = 'foo') => {};",
      "const fn = function (a = 5, b = true, c = 'foo') {};",
      "function fn(a = 5, b = true, c = 'foo') {}",
      "function fn(a: number, b: boolean, c: string) {}",
      "class Foo {
      a = 5;
      b = true;
      c = 'foo';
    }",
      "class Foo {
      readonly a: number = 5;
      }",
      "class Foo {
        a?: number = 5;
        b?: boolean = true;
        c?: string = 'foo';
      }",
      "const fn = function (a: any = 5, b: any = true, c: any = 'foo') {};",
    };
  }

  #[test]
  fn no_inferrable_types_invalid() {
    assert_lint_err! {
      NoInferrableTypes,
      "const a: bigint = 10n": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = -10n": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = BigInt(10)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = -BigInt?.(10)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: bigint = -BigInt?.(10)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = false": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = true": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = Boolean(true)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = Boolean(null)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = Boolean?.(null)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: boolean = !0": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = 10": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +10": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -10": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = Number('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +Number('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -Number('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = Number?.('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +Number?.('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -Number?.('1')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = Infinity": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +Infinity": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -Infinity": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = NaN": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = +NaN": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = -NaN": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: null = null": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: RegExp = /a/": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: RegExp = RegExp('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: RegExp = RegExp?.('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: RegExp = new RegExp?.('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = 'str'": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      r#"const a: string = "str""#: [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = `str`": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = String(1)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: string = String?.(1)": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: symbol = Symbol('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: symbol = Symbol?.('a')": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: undefined = undefined": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: undefined = void someValue": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a: number = 0, b: string = 'foo';": [
        {
          col: 6,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        },
        {
          col: 21,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "function f(a: number = 5) {};": [
        {
          col: 11,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const fn = (a: number = 5, b: boolean = true, c: string = 'foo') => {};": [
        {
          col: 12,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        },
        {
          col: 27,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        },
        {
          col: 46,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { a: number = 42; }": [
        {
          col: 10,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { a(x: number = 42) {} }": [
        {
          col: 12,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],

      // https://github.com/denoland/deno_lint/issues/558
      "class A { #foo: string = '' }": [
        {
          col: 10,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { static #foo: string = '' }": [
        {
          col: 10,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { #foo(x: number = 42) {} }": [
        {
          col: 15,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { static #foo(x: number = 42) {} }": [
        {
          col: 22,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],

      // nested
      "function a() { const x: number = 5; }": [
        {
          col: 21,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a = () => { const b = (x: number = 42) => {}; };": [
        {
          col: 29,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "class A { a = class { b: number = 42; }; }": [
        {
          col: 22,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
      "const a = function () { let x: number = 42; };": [
        {
          col: 28,
          message: NoInferrableTypesMessage::NotAllowed,
          hint: NoInferrableTypesHint::Remove,
        }
      ],
    };
  }
}
