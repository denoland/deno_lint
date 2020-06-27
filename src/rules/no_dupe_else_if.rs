#![allow(unused_imports, dead_code)]
// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_util::{equal_in_if_else, DropSpan, SpanDropped};
use std::cmp::Ordering;
use std::collections::{BTreeMap, HashSet};
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
    let span = if_stmt.test.span();
    if !self.checked_span.contains(&span) {
      self.checked_span.insert(span);
      let mut conditions = BTreeMap::new();
      let test = (&*if_stmt.test).clone().drop_span();
      conditions.insert(IfElseEqualChecker(test), vec![span]);
      let mut next = if_stmt.alt.as_ref();
      while let Some(alt) = next {
        if let Stmt::If(IfStmt {
          ref test, ref alt, ..
        }) = &**alt
        {
          // preserve the span before dropping
          let span = test.span();
          let test = (&**test).clone().drop_span();
          conditions
            .entry(IfElseEqualChecker(test))
            .or_insert_with(Vec::new)
            .push(span);
          self.checked_span.insert(span);
          next = alt.as_ref();
        } else {
          break;
        }
      }
      //dbg!(&conditions);

      conditions
        .values()
        .map(|c| c.iter().skip(1))
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

#[derive(Debug)]
struct IfElseEqualChecker(SpanDropped<Expr>);

impl PartialEq for IfElseEqualChecker {
  fn eq(&self, other: &Self) -> bool {
    let IfElseEqualChecker(ref self_sd) = self;
    let IfElseEqualChecker(ref other_sd) = other;

    equal_in_if_else(&self_sd.as_ref(), &other_sd.as_ref())
  }
}

impl Eq for IfElseEqualChecker {}

impl PartialOrd for IfElseEqualChecker {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for IfElseEqualChecker {
  fn cmp(&self, other: &Self) -> Ordering {
    if self == other {
      Ordering::Equal
    } else {
      let IfElseEqualChecker(ref self_sd) = self;
      let IfElseEqualChecker(ref other_sd) = other;
      self_sd.cmp(other_sd)
    }
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
      "if (a === 1) {} else if ((a === 1)) {}",
      0,
    );
  }

  #[test]
  fn no_dupe_else_if_invalid() {
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (a) {} else if (b) {}",
      19,
    );
    assert_lint_err::<NoDupeElseIf>("if (a); else if (a);", 17);
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a) {} else {}", 19);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a) {} else if (c) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c) {} else if (a) {}",
      49,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (b) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (b) {} else {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c) {} else if (b) {}",
      49,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a); else if (b); else if (c); else if (b); else if (d); else;",
      43,
    );
    assert_lint_err::<NoDupeElseIf>("if (a); else if (b); else if (c); else if (d); else if (b); else if (e);", 56);
    assert_lint_err_n::<NoDupeElseIf>(
      "if (a) {} else if (a) {} else if (a) {}",
      vec![19, 34],
    );
    assert_lint_err_n::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a) {} else if (b) {} else if (a) {}",
      vec![34, 64, 49],
    );
    assert_lint_err::<NoDupeElseIf>("if (a) { if (b) {} } else if (a) {}", 30);
    assert_lint_err::<NoDupeElseIf>("if (a === 1) {} else if (a === 1) {}", 25);
    assert_lint_err::<NoDupeElseIf>("if (1 < a) {} else if (1 < a) {}", 23);
    assert_lint_err::<NoDupeElseIf>("if (true) {} else if (true) {}", 22);
    assert_lint_err::<NoDupeElseIf>("if (a && b) {} else if (a && b) {}", 24);
    assert_lint_err::<NoDupeElseIf>(
      "if (a && b || c)  {} else if (a && b || c) {}",
      30,
    );
    assert_lint_err::<NoDupeElseIf>("if (f(a)) {} else if (f(a)) {}", 22);
    assert_lint_err::<NoDupeElseIf>("if (a === 1) {} else if (a===1) {}", 25);
    assert_lint_err::<NoDupeElseIf>(
      "if (a === 1) {} else if (a === /* comment */ 1) {}",
      25,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a === 1) {} else if ((a === 1)) {}",
      25,
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
