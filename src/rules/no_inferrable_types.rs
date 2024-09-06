// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::{
  ArrowExpr, CallExpr, ClassProp, Expr, Function, Ident, Lit, NewExpr, OptCall,
  OptChainBase, OptChainExpr, Pat, PrivateProp, TsEntityName, TsKeywordType,
  TsKeywordTypeKind, TsType, TsTypeAnn, TsTypeRef, UnaryExpr, VarDecl,
};
use deno_ast::swc::ast::{Callee, PropName};
use deno_ast::swc::visit::{Visit, VisitWith};
use deno_ast::SourceRange;
use deno_ast::SourceRangedForSpanned;
use derive_more::Display;

#[derive(Debug)]
pub struct NoInferrableTypes;

const CODE: &str = "no-inferrable-types";

#[derive(Display)]
enum NoInferrableTypesMessage {
  #[display("inferrable types are not allowed")]
  NotAllowed,
}

#[derive(Display)]
enum NoInferrableTypesHint {
  #[display("Remove the type, it is easily inferrable")]
  Remove,
}

impl LintRule for NoInferrableTypes {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = NoInferrableTypesVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_inferrable_types.md")
  }
}

struct NoInferrableTypesVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoInferrableTypesVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn add_diagnostic_helper(&mut self, range: SourceRange) {
    self.context.add_diagnostic_with_hint(
      range,
      CODE,
      NoInferrableTypesMessage::NotAllowed,
      NoInferrableTypesHint::Remove,
    )
  }

  fn check_callee(
    &mut self,
    callee: &Callee,
    range: SourceRange,
    expected_sym: &str,
  ) {
    if let Callee::Expr(expr) = &callee {
      self.check_callee_expr(expr, range, expected_sym);
    }
  }

  fn check_callee_expr(
    &mut self,
    expr: &Expr,
    range: SourceRange,
    expected_sym: &str,
  ) {
    if let Expr::Ident(value) = expr {
      if value.sym == *expected_sym {
        self.add_diagnostic_helper(range);
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
    range: SourceRange,
  ) {
    use TsKeywordTypeKind::*;
    match ts_type.kind {
      TsBigIntKeyword => match value {
        Expr::Lit(Lit::BigInt(_)) => {
          self.add_diagnostic_helper(range);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, range, "BigInt");
        }
        Expr::Unary(UnaryExpr { arg, .. }) => match &**arg {
          Expr::Lit(Lit::BigInt(_)) => {
            self.add_diagnostic_helper(range);
          }
          Expr::Call(CallExpr { callee, .. }) => {
            self.check_callee(callee, range, "BigInt");
          }
          Expr::OptChain(OptChainExpr { base, .. }) => {
            if let OptChainBase::Call(OptCall { callee, .. }) = &**base {
              self.check_callee_expr(callee, range, "BigInt");
            }
          }
          _ => {}
        },
        Expr::OptChain(OptChainExpr { base, .. }) => {
          if let OptChainBase::Call(OptCall { callee, .. }) = &**base {
            self.check_callee_expr(callee, range, "BigInt");
          }
        }
        _ => {}
      },
      TsBooleanKeyword => match value {
        Expr::Lit(Lit::Bool(_)) => {
          self.add_diagnostic_helper(range);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, range, "Boolean");
        }
        Expr::Unary(UnaryExpr { op, .. }) => {
          if op.to_string() == "!" {
            self.add_diagnostic_helper(range);
          }
        }
        Expr::OptChain(OptChainExpr { base, .. }) => {
          if let OptChainBase::Call(OptCall { callee, .. }) = &**base {
            self.check_callee_expr(callee, range, "Boolean");
          }
        }
        _ => {}
      },
      TsNumberKeyword => match value {
        Expr::Lit(Lit::Num(_)) => {
          self.add_diagnostic_helper(range);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, range, "Number");
        }
        Expr::Ident(ident) => {
          if self.is_nan_or_infinity(ident) {
            self.add_diagnostic_helper(range);
          }
        }
        Expr::Unary(UnaryExpr { arg, .. }) => match &**arg {
          Expr::Lit(Lit::Num(_)) => {
            self.add_diagnostic_helper(range);
          }
          Expr::Call(CallExpr { callee, .. }) => {
            self.check_callee(callee, range, "Number");
          }
          Expr::Ident(ident) => {
            if self.is_nan_or_infinity(ident) {
              self.add_diagnostic_helper(range);
            }
          }
          Expr::OptChain(OptChainExpr { base, .. }) => {
            if let OptChainBase::Call(OptCall { callee, .. }) = &**base {
              self.check_callee_expr(callee, range, "Number");
            }
          }
          _ => {}
        },
        Expr::OptChain(OptChainExpr { base, .. }) => {
          if let OptChainBase::Call(OptCall { callee, .. }) = &**base {
            self.check_callee_expr(callee, range, "Number");
          }
        }
        _ => {}
      },
      TsNullKeyword => {
        if let Expr::Lit(Lit::Null(_)) = value {
          self.add_diagnostic_helper(range);
        }
      }
      TsStringKeyword => match value {
        Expr::Lit(Lit::Str(_)) => {
          self.add_diagnostic_helper(range);
        }
        Expr::Tpl(_) => {
          self.add_diagnostic_helper(range);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, range, "String");
        }
        Expr::OptChain(OptChainExpr { base, .. }) => {
          if let OptChainBase::Call(OptCall { callee, .. }) = &**base {
            self.check_callee_expr(callee, range, "String");
          }
        }
        _ => {}
      },
      TsSymbolKeyword => {
        if let Expr::Call(CallExpr { callee, .. }) = value {
          self.check_callee(callee, range, "Symbol");
        } else if let Expr::OptChain(OptChainExpr { base, .. }) = value {
          if let OptChainBase::Call(OptCall { callee, .. }) = &**base {
            self.check_callee_expr(callee, range, "Symbol");
          }
        }
      }
      TsUndefinedKeyword => match value {
        Expr::Ident(ident) => {
          if ident.sym == *"undefined" {
            self.add_diagnostic_helper(range);
          }
        }
        Expr::Unary(UnaryExpr { op, .. }) => {
          if op.to_string() == "void" {
            self.add_diagnostic_helper(range);
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
    range: SourceRange,
  ) {
    if let TsEntityName::Ident(ident) = &ts_type.type_name {
      if ident.sym != *"RegExp" {
        return;
      }
      match value {
        Expr::Lit(Lit::Regex(_)) => {
          self.add_diagnostic_helper(range);
        }
        Expr::Call(CallExpr { callee, .. }) => {
          self.check_callee(callee, range, "RegExp");
        }
        Expr::New(NewExpr { callee, .. }) => {
          if let Expr::Ident(ident) = &**callee {
            if ident.sym == *"RegExp" {
              self.add_diagnostic_helper(range);
            }
          } else if let Expr::OptChain(opt_chain) = &**callee {
            if let OptChainBase::Call(OptCall { callee, .. }) = &*opt_chain.base
            {
              self.check_callee_expr(callee, range, "RegExp");
            }
          }
        }
        Expr::OptChain(opt_chain) => {
          if let OptChainBase::Call(OptCall { callee, .. }) = &*opt_chain.base {
            self.check_callee_expr(callee, range, "RegExp");
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
    range: SourceRange,
  ) {
    if let TsType::TsKeywordType(ts_type) = &*ts_type.type_ann {
      self.check_keyword_type(value, ts_type, range);
    } else if let TsType::TsTypeRef(ts_type) = &*ts_type.type_ann {
      self.check_ref_type(value, ts_type, range);
    }
  }
}

impl<'c, 'view> Visit for NoInferrableTypesVisitor<'c, 'view> {
  fn visit_function(&mut self, function: &Function) {
    for param in &function.params {
      if let Pat::Assign(assign_pat) = &param.pat {
        if let Pat::Ident(ident) = &*assign_pat.left {
          if let Some(ident_type_ann) = &ident.type_ann {
            self.check_ts_type(
              &assign_pat.right,
              ident_type_ann,
              param.range(),
            );
          }
        }
      }
    }
    function.visit_children_with(self);
  }

  fn visit_arrow_expr(&mut self, arr_expr: &ArrowExpr) {
    for param in &arr_expr.params {
      if let Pat::Assign(assign_pat) = &param {
        if let Pat::Ident(ident) = &*assign_pat.left {
          if let Some(ident_type_ann) = &ident.type_ann {
            self.check_ts_type(
              &assign_pat.right,
              ident_type_ann,
              assign_pat.range(),
            );
          }
        }
      }
    }
    arr_expr.visit_children_with(self);
  }

  fn visit_class_prop(&mut self, prop: &ClassProp) {
    if prop.readonly || prop.is_optional {
      return;
    }
    if let Some(init) = &prop.value {
      if let PropName::Ident(_) = &prop.key {
        if let Some(ident_type_ann) = &prop.type_ann {
          self.check_ts_type(init, ident_type_ann, prop.range());
        }
      }
    }
    prop.visit_children_with(self);
  }

  fn visit_private_prop(&mut self, prop: &PrivateProp) {
    if prop.readonly || prop.is_optional {
      return;
    }
    if let Some(init) = &prop.value {
      if let Some(ident_type_ann) = &prop.type_ann {
        self.check_ts_type(init, ident_type_ann, prop.range());
      }
    }
    prop.visit_children_with(self);
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl) {
    for decl in &var_decl.decls {
      if let Some(init) = &decl.init {
        if let Pat::Ident(ident) = &decl.name {
          if let Some(ident_type_ann) = &ident.type_ann {
            self.check_ts_type(init, ident_type_ann, decl.range());
          }
        }
      }
    }
    var_decl.visit_children_with(self);
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
