// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;

use swc_ecmascript::ast::{
  Expr, ExprOrSuper, Lit, TsKeywordType, TsType, TsTypeRef, VarDecl,
};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoInferrableTypes;

impl LintRule for NoInferrableTypes {
  fn new() -> Box<Self> {
    Box::new(NoInferrableTypes)
  }

  fn code(&self) -> &'static str {
    "no-inferrable-types"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoInferrableTypesVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoInferrableTypesVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoInferrableTypesVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic_helper(&self, span: swc_common::Span) {
    self.context.add_diagnostic(
      span,
      "no-inferrable-types",
      "inferrable types are not allowed",
    )
  }

  fn check_callee(
    &self,
    callee: &ExprOrSuper,
    span: swc_common::Span,
    expected_sym: &str,
  ) {
    if let swc_ecmascript::ast::ExprOrSuper::Expr(unboxed) = &callee {
      if let Expr::Ident(value) = &**unboxed {
        if value.sym == *expected_sym {
          self.add_diagnostic_helper(span);
        }
      }
    }
  }

  fn is_nan_or_infinity(&self, ident: &swc_ecmascript::ast::Ident) -> bool {
    ident.sym == *"NaN" || ident.sym == *"Infinity"
  }

  fn check_keyword_type(
    &self,
    value: &Expr,
    ts_type: &TsKeywordType,
    span: swc_common::Span,
  ) {
    use swc_ecmascript::ast::TsKeywordTypeKind::*;
    match ts_type.kind {
      TsBigIntKeyword => match &*value {
        Expr::Lit(Lit::BigInt(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "BigInt");
        }
        Expr::Unary(swc_ecmascript::ast::UnaryExpr { arg, .. }) => match &**arg
        {
          Expr::Lit(Lit::BigInt(_)) => {
            self.add_diagnostic_helper(span);
          }
          Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) => {
            self.check_callee(callee, span, "BigInt");
          }
          Expr::OptChain(swc_ecmascript::ast::OptChainExpr {
            expr, ..
          }) => {
            if let Expr::Call(swc_ecmascript::ast::CallExpr {
              callee, ..
            }) = &**expr
            {
              self.check_callee(callee, span, "BigInt");
            }
          }
          _ => {}
        },
        Expr::OptChain(swc_ecmascript::ast::OptChainExpr { expr, .. }) => {
          if let Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) =
            &**expr
          {
            self.check_callee(callee, span, "BigInt");
          }
        }
        _ => {}
      },
      TsBooleanKeyword => match &*value {
        Expr::Lit(Lit::Bool(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "Boolean");
        }
        Expr::Unary(swc_ecmascript::ast::UnaryExpr { op, .. }) => {
          if op.to_string() == "!" {
            self.add_diagnostic_helper(span);
          }
        }
        Expr::OptChain(swc_ecmascript::ast::OptChainExpr { expr, .. }) => {
          if let Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) =
            &**expr
          {
            self.check_callee(callee, span, "Boolean");
          }
        }
        _ => {}
      },
      TsNumberKeyword => match &*value {
        Expr::Lit(Lit::Num(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "Number");
        }
        Expr::Ident(ident) => {
          if self.is_nan_or_infinity(&ident) {
            self.add_diagnostic_helper(span);
          }
        }
        Expr::Unary(swc_ecmascript::ast::UnaryExpr { arg, .. }) => match &**arg
        {
          Expr::Lit(Lit::Num(_)) => {
            self.add_diagnostic_helper(span);
          }
          Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) => {
            self.check_callee(callee, span, "Number");
          }
          Expr::Ident(ident) => {
            if self.is_nan_or_infinity(&ident) {
              self.add_diagnostic_helper(span);
            }
          }
          Expr::OptChain(swc_ecmascript::ast::OptChainExpr {
            expr, ..
          }) => {
            if let Expr::Call(swc_ecmascript::ast::CallExpr {
              callee, ..
            }) = &**expr
            {
              self.check_callee(callee, span, "Number");
            }
          }
          _ => {}
        },
        Expr::OptChain(swc_ecmascript::ast::OptChainExpr { expr, .. }) => {
          if let Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) =
            &**expr
          {
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
        Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "String");
        }
        Expr::OptChain(swc_ecmascript::ast::OptChainExpr { expr, .. }) => {
          if let Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) =
            &**expr
          {
            self.check_callee(callee, span, "String");
          }
        }
        _ => {}
      },
      TsSymbolKeyword => {
        if let Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) =
          &*value
        {
          self.check_callee(callee, span, "Symbol");
        } else if let Expr::OptChain(swc_ecmascript::ast::OptChainExpr {
          expr,
          ..
        }) = &*value
        {
          if let Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) =
            &**expr
          {
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
        Expr::Unary(swc_ecmascript::ast::UnaryExpr { op, .. }) => {
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
    &self,
    value: &Expr,
    ts_type: &TsTypeRef,
    span: swc_common::Span,
  ) {
    if let swc_ecmascript::ast::TsEntityName::Ident(ident) = &ts_type.type_name
    {
      if ident.sym != *"RegExp" {
        return;
      }
      match &*value {
        Expr::Lit(Lit::Regex(_)) => {
          self.add_diagnostic_helper(span);
        }
        Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) => {
          self.check_callee(callee, span, "RegExp");
        }
        Expr::New(swc_ecmascript::ast::NewExpr { callee, .. }) => {
          if let Expr::Ident(ident) = &**callee {
            if ident.sym == *"RegExp" {
              self.add_diagnostic_helper(span);
            }
          } else if let Expr::OptChain(swc_ecmascript::ast::OptChainExpr {
            expr,
            ..
          }) = &**callee
          {
            if let Expr::Call(swc_ecmascript::ast::CallExpr {
              callee, ..
            }) = &**expr
            {
              self.check_callee(callee, span, "RegExp");
            }
          }
        }
        Expr::OptChain(swc_ecmascript::ast::OptChainExpr { expr, .. }) => {
          if let Expr::Call(swc_ecmascript::ast::CallExpr { callee, .. }) =
            &**expr
          {
            self.check_callee(callee, span, "RegExp");
          }
        }
        _ => {}
      }
    }
  }

  fn check_ts_type(
    &self,
    value: &Expr,
    ts_type: &swc_ecmascript::ast::TsTypeAnn,
    span: swc_common::Span,
  ) {
    if let TsType::TsKeywordType(ts_type) = &*ts_type.type_ann {
      self.check_keyword_type(&value, ts_type, span);
    } else if let TsType::TsTypeRef(ts_type) = &*ts_type.type_ann {
      self.check_ref_type(&value, ts_type, span);
    }
  }
}

impl<'c> Visit for NoInferrableTypesVisitor<'c> {
  fn visit_function(
    &mut self,
    function: &swc_ecmascript::ast::Function,
    _parent: &dyn Node,
  ) {
    for param in &function.params {
      if let swc_ecmascript::ast::Pat::Assign(assign_pat) = &param.pat {
        if let swc_ecmascript::ast::Pat::Ident(ident) = &*assign_pat.left {
          if let Some(ident_type_ann) = &ident.type_ann {
            self.check_ts_type(&assign_pat.right, ident_type_ann, param.span);
          }
        }
      }
    }
  }

  fn visit_arrow_expr(
    &mut self,
    arr_expr: &swc_ecmascript::ast::ArrowExpr,
    _parent: &dyn Node,
  ) {
    for param in &arr_expr.params {
      if let swc_ecmascript::ast::Pat::Assign(assign_pat) = &param {
        if let swc_ecmascript::ast::Pat::Ident(ident) = &*assign_pat.left {
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

  fn visit_class_prop(
    &mut self,
    prop: &swc_ecmascript::ast::ClassProp,
    _parent: &dyn Node,
  ) {
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

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    if let Some(init) = &var_decl.decls[0].init {
      if let Expr::Fn(fn_expr) = &**init {
        if !fn_expr.function.params.is_empty() {
          self.visit_function(&fn_expr.function, _parent);
        }
      } else if let Expr::Arrow(arr_expr) = &**init {
        if !arr_expr.params.is_empty() {
          self.visit_arrow_expr(&arr_expr, _parent);
        }
      }
      if let swc_ecmascript::ast::Pat::Ident(ident) = &var_decl.decls[0].name {
        if let Some(ident_type_ann) = &ident.type_ann {
          self.check_ts_type(init, ident_type_ann, var_decl.span);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;
  #[test]
  fn no_inferrable_types_valid() {
    assert_lint_ok::<NoInferrableTypes>("const a = 10n");
    assert_lint_ok::<NoInferrableTypes>("const a = -10n");
    assert_lint_ok::<NoInferrableTypes>("const a = BigInt(10)");
    assert_lint_ok::<NoInferrableTypes>("const a = -BigInt?.(10)");
    assert_lint_ok::<NoInferrableTypes>("const a = +BigInt?.(10)");

    assert_lint_ok::<NoInferrableTypes>("const a = false");
    assert_lint_ok::<NoInferrableTypes>("const a = true");
    assert_lint_ok::<NoInferrableTypes>("const a = Boolean(true)");
    assert_lint_ok::<NoInferrableTypes>("const a = Boolean(null)");
    assert_lint_ok::<NoInferrableTypes>("const a = Boolean?.(null)");
    assert_lint_ok::<NoInferrableTypes>("const a = !0");

    assert_lint_ok::<NoInferrableTypes>("const a = 10");
    assert_lint_ok::<NoInferrableTypes>("const a = +10");
    assert_lint_ok::<NoInferrableTypes>("const a = -10");
    assert_lint_ok::<NoInferrableTypes>("const a = Number('1')");
    assert_lint_ok::<NoInferrableTypes>("const a = +Number('1')");
    assert_lint_ok::<NoInferrableTypes>("const a = -Number('1')");
    assert_lint_ok::<NoInferrableTypes>("const a = Number?.('1')");
    assert_lint_ok::<NoInferrableTypes>("const a = +Number?.('1')");
    assert_lint_ok::<NoInferrableTypes>("const a = -Number?.('1')");
    assert_lint_ok::<NoInferrableTypes>("const a = Infinity");
    assert_lint_ok::<NoInferrableTypes>("const a = +Infinity");
    assert_lint_ok::<NoInferrableTypes>("const a = -Infinity");
    assert_lint_ok::<NoInferrableTypes>("const a = NaN");
    assert_lint_ok::<NoInferrableTypes>("const a = +NaN");
    assert_lint_ok::<NoInferrableTypes>("const a = -NaN");

    assert_lint_ok::<NoInferrableTypes>("const a = null");

    assert_lint_ok::<NoInferrableTypes>("const a = /a/");
    assert_lint_ok::<NoInferrableTypes>("const a = RegExp('a')");
    assert_lint_ok::<NoInferrableTypes>("const a = RegExp?.('a')");
    assert_lint_ok::<NoInferrableTypes>("const a = new RegExp?.('a')");

    assert_lint_ok::<NoInferrableTypes>("const a = 'str'");
    assert_lint_ok::<NoInferrableTypes>(r#"const a = "str""#);
    assert_lint_ok::<NoInferrableTypes>("const a = `str`");
    assert_lint_ok::<NoInferrableTypes>("const a = String(1)");
    assert_lint_ok::<NoInferrableTypes>("const a = String?.(1)");

    assert_lint_ok::<NoInferrableTypes>("const a = Symbol('a')");
    assert_lint_ok::<NoInferrableTypes>("const a = Symbol?.('a')");

    assert_lint_ok::<NoInferrableTypes>("const a = undefined");
    assert_lint_ok::<NoInferrableTypes>("const a = void someValue");

    assert_lint_ok::<NoInferrableTypes>(
      "const fn = (a = 5, b = true, c = 'foo') => {};",
    );
    assert_lint_ok::<NoInferrableTypes>(
      "const fn = function (a = 5, b = true, c = 'foo') {};",
    );
    assert_lint_ok::<NoInferrableTypes>(
      "function fn(a = 5, b = true, c = 'foo') {}",
    );
    assert_lint_ok::<NoInferrableTypes>(
      "function fn(a: number, b: boolean, c: string) {}",
    );

    assert_lint_ok::<NoInferrableTypes>(
      "class Foo {
      a = 5;
      b = true;
      c = 'foo';
    }",
    );

    assert_lint_ok::<NoInferrableTypes>(
      "class Foo {
      readonly a: number = 5;
      }",
    );

    assert_lint_ok::<NoInferrableTypes>(
      "class Foo {
        a?: number = 5;
        b?: boolean = true;
        c?: string = 'foo';
      }",
    );

    assert_lint_ok::<NoInferrableTypes>(
      "const fn = function (a: any = 5, b: any = true, c: any = 'foo') {};",
    );
  }

  #[test]
  fn no_inferrable_types_invalid() {
    assert_lint_err::<NoInferrableTypes>("const a: bigint = 10n", 0);
    assert_lint_err::<NoInferrableTypes>("const a: bigint = -10n", 0);
    assert_lint_err::<NoInferrableTypes>("const a: bigint = BigInt(10)", 0);
    assert_lint_err::<NoInferrableTypes>("const a: bigint = -BigInt?.(10)", 0);
    assert_lint_err::<NoInferrableTypes>("const a: bigint = -BigInt?.(10)", 0);

    assert_lint_err::<NoInferrableTypes>("const a: boolean = false", 0);
    assert_lint_err::<NoInferrableTypes>("const a: boolean = true", 0);
    assert_lint_err::<NoInferrableTypes>("const a: boolean = Boolean(true)", 0);
    assert_lint_err::<NoInferrableTypes>("const a: boolean = Boolean(null)", 0);
    assert_lint_err::<NoInferrableTypes>(
      "const a: boolean = Boolean?.(null)",
      0,
    );
    assert_lint_err::<NoInferrableTypes>("const a: boolean = !0", 0);

    assert_lint_err::<NoInferrableTypes>("const a: number = 10", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = +10", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = -10", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = Number('1')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = +Number('1')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = -Number('1')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = Number?.('1')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = +Number?.('1')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = -Number?.('1')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = Infinity", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = +Infinity", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = -Infinity", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = NaN", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = +NaN", 0);
    assert_lint_err::<NoInferrableTypes>("const a: number = -NaN", 0);

    assert_lint_err::<NoInferrableTypes>("const a: null = null", 0);

    assert_lint_err::<NoInferrableTypes>("const a: RegExp = /a/", 0);
    assert_lint_err::<NoInferrableTypes>("const a: RegExp = RegExp('a')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: RegExp = RegExp?.('a')", 0);
    assert_lint_err::<NoInferrableTypes>(
      "const a: RegExp = new RegExp?.('a')",
      0,
    );

    assert_lint_err::<NoInferrableTypes>("const a: string = 'str'", 0);
    assert_lint_err::<NoInferrableTypes>(r#"const a: string = "str""#, 0);
    assert_lint_err::<NoInferrableTypes>("const a: string = `str`", 0);
    assert_lint_err::<NoInferrableTypes>("const a: string = String(1)", 0);
    assert_lint_err::<NoInferrableTypes>("const a: string = String?.(1)", 0);

    assert_lint_err::<NoInferrableTypes>("const a: symbol = Symbol('a')", 0);
    assert_lint_err::<NoInferrableTypes>("const a: symbol = Symbol?.('a')", 0);

    assert_lint_err::<NoInferrableTypes>("const a: undefined = undefined", 0);
    assert_lint_err::<NoInferrableTypes>(
      "const a: undefined = void someValue",
      0,
    );
    assert_lint_err_n::<NoInferrableTypes>(
      "const fn = (a: number = 5, b: boolean = true, c: string = 'foo') => {};",
      vec![12, 27, 46],
    );

    assert_lint_err_on_line_n::<NoInferrableTypes>(
      "class Foo {
a: number = 5;
b: boolean = true;
c: string = 'foo';
}",
      vec![(2, 0), (3, 0), (4, 0)],
    )
  }
}
