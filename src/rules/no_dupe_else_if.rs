#![allow(unused_imports, dead_code)]
// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use swc_common::{Span, Spanned, DUMMY_SP};
use swc_ecma_ast::{
  ArrayLit, ArrowExpr, AssignExpr, AwaitExpr, BigInt, BinExpr, BlockStmt,
  BlockStmtOrExpr, Bool, CallExpr, ClassExpr, CondExpr, Constructor, Expr,
  FnExpr, Function, Ident, IfStmt, Invalid, JSXElement, JSXEmptyExpr,
  JSXFragment, JSXMemberExpr, JSXNamespacedName, JSXText, Lit, MemberExpr,
  MetaPropExpr, Module, NewExpr, Null, Number, ObjectLit, OptChainExpr,
  ParenExpr, PrivateName, Regex, SeqExpr, Stmt, Str, SwitchStmt, TaggedTpl,
  ThisExpr, Tpl, TsAsExpr, TsConstAssertion, TsNonNullExpr, TsTypeAssertion,
  TsTypeCastExpr, UnaryExpr, UpdateExpr, YieldExpr,
};
use swc_ecma_visit::{Fold, Node, Visit};

pub struct NoDupeElseIf;

impl LintRule for NoDupeElseIf {
  fn new() -> Box<Self> {
    Box::new(NoDupeElseIf)
  }

  fn code(&self) -> &'static str {
    "no-dupe-else-if"
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = NoDupeElseIfVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct NoDupeElseIfVisitor {
  context: Context,
  checked_span: HashSet<Span>,
}

impl NoDupeElseIfVisitor {
  pub fn new(context: Context) -> Self {
    Self {
      context,
      checked_span: HashSet::new(),
    }
  }
}

impl Visit for NoDupeElseIfVisitor {
  fn visit_if_stmt(&mut self, if_stmt: &IfStmt, parent: &dyn Node) {
    let mut dropper = ExprSpanDropper;
    let span = if_stmt.test.span();
    if !self.checked_span.contains(&span) {
      self.checked_span.insert(span);
      let mut conditions = HashMap::new();
      let test = dropper.fold_expr((&*if_stmt.test).clone());
      conditions.insert(ConditionToCheck::new(test), vec![if_stmt.test.span()]);
      let mut next = if_stmt.alt.as_ref();
      while let Some(alt) = next {
        if let Stmt::If(IfStmt {
          ref test, ref alt, ..
        }) = &**alt
        {
          // preserve the span before dropping
          let span = test.span();
          let test = dropper.fold_expr((&**test).clone());
          conditions
            .entry(ConditionToCheck::new(test))
            .or_insert_with(Vec::new)
            .push(span);
          self.checked_span.insert(span);
          next = alt.as_ref();
        } else {
          break;
        }
      }
      dbg!(&conditions);

      conditions
        .values()
        .filter_map(|c| if c.len() >= 2 {
          Some(c.iter().skip(1))
        } else {
          None
        })
        .flatten()
        .for_each(|span| {
          self
            .context
            .add_diagnostic(*span, "no-dupe-else-if", "This branch can never execute. Its condition is a duplicate or covered by previous conditions in the if-else-if chain.")
        });
    }

    swc_ecma_visit::visit_if_stmt(self, if_stmt, parent);
  }
}

struct ExprSpanDropper;

impl Fold for ExprSpanDropper {
  fn fold_expr(&mut self, expr: Expr) -> Expr {
    let dropped = match expr {
      Expr::This(_) => Expr::This(ThisExpr { span: DUMMY_SP }),
      Expr::Array(ArrayLit { elems, .. }) => Expr::Array(ArrayLit {
        span: DUMMY_SP,
        elems,
      }),
      Expr::Object(ObjectLit { props, .. }) => Expr::Object(ObjectLit {
        span: DUMMY_SP,
        props,
      }),
      Expr::Fn(FnExpr {
        mut ident,
        mut function,
      }) => {
        ident.as_mut().map(|i| i.span = DUMMY_SP);
        function.span = DUMMY_SP;
        Expr::Fn(FnExpr { ident, function })
      }
      Expr::Unary(UnaryExpr { op, arg, .. }) => Expr::Unary(UnaryExpr {
        span: DUMMY_SP,
        op,
        arg,
      }),
      Expr::Update(UpdateExpr {
        op, prefix, arg, ..
      }) => Expr::Update(UpdateExpr {
        span: DUMMY_SP,
        op,
        prefix,
        arg,
      }),
      Expr::Bin(BinExpr {
        op, left, right, ..
      }) => Expr::Bin(BinExpr {
        span: DUMMY_SP,
        op,
        left,
        right,
      }),
      Expr::Assign(AssignExpr {
        op, left, right, ..
      }) => Expr::Assign(AssignExpr {
        span: DUMMY_SP,
        op,
        left,
        right,
      }),
      Expr::Member(MemberExpr {
        obj,
        prop,
        computed,
        ..
      }) => Expr::Member(MemberExpr {
        span: DUMMY_SP,
        obj,
        prop,
        computed,
      }),
      Expr::Cond(CondExpr {
        test, cons, alt, ..
      }) => Expr::Cond(CondExpr {
        span: DUMMY_SP,
        test,
        cons,
        alt,
      }),
      Expr::Call(CallExpr {
        callee,
        args,
        type_args,
        ..
      }) => Expr::Call(CallExpr {
        span: DUMMY_SP,
        callee,
        args,
        type_args,
      }),
      Expr::New(NewExpr {
        callee,
        args,
        type_args,
        ..
      }) => Expr::New(NewExpr {
        span: DUMMY_SP,
        callee,
        args,
        type_args,
      }),
      Expr::Seq(SeqExpr { exprs, .. }) => Expr::Seq(SeqExpr {
        span: DUMMY_SP,
        exprs,
      }),
      Expr::Ident(Ident {
        sym,
        type_ann,
        optional,
        ..
      }) => Expr::Ident(Ident {
        span: DUMMY_SP,
        sym,
        type_ann,
        optional,
      }),
      Expr::Lit(lit) => {
        let l = match lit {
          Lit::Str(Str {
            value, has_escape, ..
          }) => Lit::Str(Str {
            span: DUMMY_SP,
            value,
            has_escape,
          }),
          Lit::Bool(Bool { value, .. }) => Lit::Bool(Bool {
            span: DUMMY_SP,
            value,
          }),
          Lit::Null(_) => Lit::Null(Null { span: DUMMY_SP }),
          Lit::Num(Number { value, .. }) => Lit::Num(Number {
            span: DUMMY_SP,
            value,
          }),
          Lit::BigInt(BigInt { value, .. }) => Lit::BigInt(BigInt {
            span: DUMMY_SP,
            value,
          }),
          Lit::Regex(Regex { exp, flags, .. }) => Lit::Regex(Regex {
            span: DUMMY_SP,
            exp,
            flags,
          }),
          Lit::JSXText(JSXText { value, raw, .. }) => Lit::JSXText(JSXText {
            span: DUMMY_SP,
            value,
            raw,
          }),
        };
        Expr::Lit(l)
      }
      Expr::Tpl(Tpl { exprs, quasis, .. }) => Expr::Tpl(Tpl {
        span: DUMMY_SP,
        exprs,
        quasis,
      }),
      Expr::TaggedTpl(TaggedTpl {
        tag,
        exprs,
        quasis,
        type_params,
        ..
      }) => Expr::TaggedTpl(TaggedTpl {
        span: DUMMY_SP,
        tag,
        exprs,
        quasis,
        type_params,
      }),
      Expr::Arrow(ArrowExpr {
        params,
        body,
        is_async,
        is_generator,
        type_params,
        return_type,
        ..
      }) => Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        params,
        body,
        is_async,
        is_generator,
        type_params,
        return_type,
      }),
      // TODO(magurotuna) from here. next is ClassExpr
      _ => Expr::Invalid(Invalid { span: DUMMY_SP }),
    };

    swc_ecma_visit::fold_expr(self, dropped)
  }
}

#[derive(Debug, Eq, PartialEq, Hash)]
struct ConditionToCheck {
  condition: Expr,
}

impl ConditionToCheck {
  fn new(condition: Expr) -> Self {
    Self { condition }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_dupe_else_if_valid() {
    assert_lint_ok::<NoDupeElseIf>("if (a) {} else if (b) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a); else if (b); else if (c);");
    assert_lint_ok::<NoDupeElseIf>("if (true) {} else if (false) {} else {}");
    assert_lint_ok::<NoDupeElseIf>("if (1) {} else if (2) {}");
    assert_lint_ok::<NoDupeElseIf>("if (f) {} else if (f()) {}");
    assert_lint_ok::<NoDupeElseIf>("if (f(a)) {} else if (g(a)) {}");
    assert_lint_ok::<NoDupeElseIf>("if (f(a)) {} else if (f(b)) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a === 1) {} else if (a === 2) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a === 1) {} else if (b === 1) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a);");
    assert_lint_ok::<NoDupeElseIf>("if (a) {} else {}");
    assert_lint_ok::<NoDupeElseIf>("if (a) if (a) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a) if (a);");
    assert_lint_ok::<NoDupeElseIf>("if (a) { if (a) {} }");
    assert_lint_ok::<NoDupeElseIf>("if (a) {} else { if (a) {} }");
    assert_lint_ok::<NoDupeElseIf>("if (a) {} if (a) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a); if (a);");
    assert_lint_ok::<NoDupeElseIf>("while (a) if (a);");
    assert_lint_ok::<NoDupeElseIf>("if (a); else a ? a : a;");
    assert_lint_ok::<NoDupeElseIf>("if (a) { if (b) {} } else if (b) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a) if (b); else if (a);");
    assert_lint_ok::<NoDupeElseIf>("if (a) {} else if (!!a) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a === 1) {} else if (a === (1)) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a || b) {} else if (c || d) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a || b) {} else if (a || c) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a) {} else if (a || b) {}");
    assert_lint_ok::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a || b || c) {}",
    );
    assert_lint_ok::<NoDupeElseIf>(
      "if (a && b) {} else if (a) {} else if (b) {}",
    );
    assert_lint_ok::<NoDupeElseIf>(
      "if (a && b) {} else if (b && c) {} else if (a && c) {}",
    );
    assert_lint_ok::<NoDupeElseIf>("if (a && b) {} else if (b || c) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a) {} else if (b && (a || c)) {}");
    assert_lint_ok::<NoDupeElseIf>("if (a) {} else if (b && (c || d && a)) {}");
    assert_lint_ok::<NoDupeElseIf>(
      "if (a && b && c) {} else if (a && b && (c || d)) {}",
    );
  }

  #[test]
  fn hogepiyo() {
    assert_lint_err::<NoDupeElseIf>(
      "if (a>1) {} else if (a >             1) {} else if (b) {}",
      21,
    );
  }

  #[test]
  fn no_dupe_else_if_invalid() {
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (a) {} else if (b) {}",
      19,
    );
    assert_lint_err::<NoDupeElseIf>("if (a); else if (a);", 17);
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a) {} else {}", 0); // TODO(magurotuna) from here
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a) {} else if (c) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c) {} else if (a) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (b) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (b) {} else {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c) {} else if (b) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a); else if (b); else if (c); else if (b); else if (d); else;",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a); else if (b); else if (c); else if (d); else if (b); else if (e);", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (a) {} else if (a) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a) {} else if (b) {} else if (a) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a) { if (b) {} } else if (a) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a === 1) {} else if (a === 1) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (1 < a) {} else if (1 < a) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (true) {} else if (true) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a && b) {} else if (a && b) {}", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a && b || c)  {} else if (a && b || c) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (f(a)) {} else if (f(a)) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a === 1) {} else if (a===1) {}", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a === 1) {} else if (a === /* comment */ 1) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a === 1) {} else if ((a === 1)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (a) {}", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b) {} else if (a) {} else if (b) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (b || a) {}", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a || b) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b) {} else if (c || d) {} else if (a || d) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if ((a === b && fn(c)) || d) {} else if (fn(c) && a === b) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a && b) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a && b) {} else if (b && a) {}", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a && b) {} else if (a && b && c) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || c) {} else if (a && b || c) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c && a || b) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c && (a || b)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b && c) {} else if (d && (a || e && c && b)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b && c) {} else if (b && c && d) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (b && c) {}", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if ((a || b) && c) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if ((a && (b || c)) || d) {} else if ((c || b) && e && a) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a && b || b && c) {} else if (a && b && c) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b && c) {} else if (d && (c && e && b || a)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || (b && (c || d))) {} else if ((d || c) && b) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b) {} else if ((b || a) && c) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b) {} else if (c) {} else if (d) {} else if (b && (a || c)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b || c) {} else if (a || (b && d) || (c && e)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || (b || c)) {} else if (a || (b && c)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (c) {} else if (d) {} else if ((a || c) && (b || d)) {}", 0);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c && (a || d && b)) {}",
      0,
    );
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a || a) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a || a) {} else if (a || a) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a || a) {} else if (a) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a && a) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a && a) {} else if (a && a) {}", 0);
    assert_lint_err::<NoDupeElseIf>("if (a && a) {} else if (a) {}", 0);
  }
}
