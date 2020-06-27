// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::swc_util::DropSpan;
use std::collections::HashSet;
use swc_common::{Span, Spanned};
use swc_ecma_ast::{BinExpr, BinaryOp, Expr, IfStmt, Module, ParenExpr, Stmt};
use swc_ecma_visit::{Node, Visit};

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

    // This check is necessary to avoid outputting the same errors multiple times.
    if !self.checked_span.contains(&span) {
      self.checked_span.insert(span);
      let span_dropped_test = if_stmt.test.clone().drop_span();
      //let conditions_to_check = match &test {
      //Expr::Bin(BinExpr { ref op, .. }) if *op == BinaryOp::LogicalAnd => {
      //let mut v = vec![&test];
      //v.append(&mut split_by_and(&test));
      //v
      //}
      //_ => vec![&test],
      //};

      //let mut list_to_check: Vec<Vec<Vec<&Expr>>> = conditions_to_check
      //.iter()
      //.map(|&c| split_by_or(c).into_iter().map(split_by_and).collect())
      //.collect();
      let mut appeared_conditions: Vec<Vec<Vec<Expr>>> = Vec::new();
      append_test(&mut appeared_conditions, span_dropped_test);

      //let append_test_to_check = |expr: &Expr| {
      //let conditions_to_check = match *expr {
      //Expr::Bin(BinExpr { ref op, .. }) if *op == BinaryOp::LogicalAnd => {
      //let mut v = vec![expr];
      //v.append(&mut split_by_and(expr));
      //v
      //}
      //_ => vec![expr],
      //};
      //list_to_check.extend(
      //conditions_to_check
      //.iter()
      //.map(|&c| split_by_or(c).into_iter().map(split_by_and).collect()),
      //);
      //};

      //append_test_to_check(&span_dropped_test);

      //let mut conditions = BTreeMap::new();
      //conditions.insert(IfElseEqualChecker(test), vec![span]);
      let mut next = if_stmt.alt.as_ref();
      while let Some(cur) = next {
        if let Stmt::If(IfStmt {
          ref test, ref alt, ..
        }) = &**cur
        {
          // preserve the span before dropping
          let span = test.span();
          let span_dropped_test = test.clone().drop_span();
          //let current_or_operands: Vec<Vec<Expr>> =
          //split_by_or(span_dropped_test.clone())
          //.into_iter()
          //.map(split_by_and)
          //.collect();
          let mut current_condition_to_check: Vec<Vec<Vec<Expr>>> =
            mk_condition_to_check(span_dropped_test.clone())
              .into_iter()
              .map(split_by_or_then_and)
              .collect();
          //let mut appeared_conditions_to_check = appeared_conditions.clone();

          for ap_cond in &appeared_conditions {
            current_condition_to_check = current_condition_to_check
              .into_iter()
              .map(|current_or_operands| {
                current_or_operands
                  .into_iter()
                  .filter(|current_or_operand| {
                    !ap_cond.iter().any(|ap_or_operand| {
                      is_subset(ap_or_operand, current_or_operand)
                    })
                  })
                  .collect()
              })
              .collect();

            if current_condition_to_check
              .iter()
              .any(|or_operands| or_operands.len() == 0)
            {
              self
              .context
              .add_diagnostic(span, "no-dupe-else-if", "This branch can never execute. Its condition is a duplicate or covered by previous conditions in the if-else-if chain.");
              break;
            }
          }

          //if current_condition_to_check
          //.into_iter()
          //.map(split_by_or_then_and)
          //.any(|current_or_operands| {
          //current_or_operands
          //.iter()
          //.find(|current_or_operand| {
          //appeared_conditions
          //.iter()
          //.any(|ap_cond| is_subset(current_or_operand, ap_cond))
          //})
          //.is_some()
          //})
          //{
          //self
          //.context
          //.add_diagnostic(span, "no-dupe-else-if", "This branch can never execute. Its condition is a duplicate or covered by previous conditions in the if-else-if chain.");
          //}

          //conditions
          //.entry(IfElseEqualChecker(test))
          //.or_insert_with(Vec::new)
          //.push(span);
          self.checked_span.insert(span);
          append_test(&mut appeared_conditions, span_dropped_test);
          next = alt.as_ref();
        } else {
          break;
        }
      }

      //conditions
      //.values()
      //.map(|c| c.iter().skip(1))
      //.flatten()
      //.for_each(|span| {
      //self
      //.context
      //.add_diagnostic(*span, "no-dupe-else-if", "This branch can never execute. Its condition is a duplicate or covered by previous conditions in the if-else-if chain.")
      //});
    }

    swc_ecma_visit::visit_if_stmt(self, if_stmt, parent);
  }
}

fn mk_condition_to_check(cond: Expr) -> Vec<Expr> {
  match cond {
    Expr::Bin(BinExpr { op, .. }) if op == BinaryOp::LogicalAnd => {
      let mut c = vec![cond.clone()];
      c.append(&mut split_by_and(cond));
      c
    }
    Expr::Paren(ParenExpr { expr, .. }) => mk_condition_to_check(*expr),
    _ => vec![cond],
  }
}

fn split_by_bin_op(op_to_split: BinaryOp, expr: Expr) -> Vec<Expr> {
  match expr {
    Expr::Bin(BinExpr {
      op, left, right, ..
    }) if op == op_to_split => {
      let mut ret = split_by_bin_op(op_to_split, *left);
      ret.append(&mut split_by_bin_op(op_to_split, *right));
      ret
    }
    Expr::Paren(ParenExpr { expr, .. }) => split_by_bin_op(op_to_split, *expr),
    _ => vec![expr],
  }
}

fn split_by_or(expr: Expr) -> Vec<Expr> {
  split_by_bin_op(BinaryOp::LogicalOr, expr)
}

fn split_by_and(expr: Expr) -> Vec<Expr> {
  split_by_bin_op(BinaryOp::LogicalAnd, expr)
}

fn is_subset(arr_a: &Vec<Expr>, arr_b: &Vec<Expr>) -> bool {
  arr_a
    .iter()
    .all(|a| arr_b.iter().any(|b| equal_in_if_else(a, b)))
}

/// Determines whether the two given `Expr`s are considered to be equal in if-else condition
/// context. Note that `expr1` and `expr2` must be span-dropped to be compared properly.
fn equal_in_if_else(expr1: &Expr, expr2: &Expr) -> bool {
  use swc_ecma_ast::Expr::*;
  match (expr1, expr2) {
    (Bin(ref bin1), Bin(ref bin2))
      if matches!(bin1.op, BinaryOp::LogicalOr | BinaryOp::LogicalAnd)
        && bin1.op == bin2.op =>
    {
      equal_in_if_else(&*bin1.left, &*bin2.left)
        && equal_in_if_else(&*bin1.right, &*bin2.right)
        || equal_in_if_else(&*bin1.left, &*bin2.right)
          && equal_in_if_else(&*bin1.right, &*bin2.left)
    }
    (Paren(ParenExpr { ref expr, .. }), _) => equal_in_if_else(&**expr, expr2),
    (_, Paren(ParenExpr { ref expr, .. })) => equal_in_if_else(expr1, &**expr),
    (This(_), This(_))
    | (Array(_), Array(_))
    | (Object(_), Object(_))
    | (Fn(_), Fn(_))
    | (Unary(_), Unary(_))
    | (Update(_), Update(_))
    | (Bin(_), Bin(_))
    | (Assign(_), Member(_))
    | (Cond(_), Cond(_))
    | (Call(_), Call(_))
    | (New(_), New(_))
    | (Seq(_), Seq(_))
    | (Ident(_), Ident(_))
    | (Lit(_), Lit(_))
    | (Tpl(_), Tpl(_))
    | (TaggedTpl(_), TaggedTpl(_))
    | (Arrow(_), Arrow(_))
    | (Class(_), Class(_))
    | (Yield(_), Yield(_))
    | (MetaProp(_), MetaProp(_))
    | (Await(_), Await(_))
    | (JSXMember(_), JSXMember(_))
    | (JSXNamespacedName(_), JSXNamespacedName(_))
    | (JSXEmpty(_), JSXEmpty(_))
    | (JSXElement(_), JSXElement(_))
    | (JSXFragment(_), JSXFragment(_))
    | (TsTypeAssertion(_), TsTypeAssertion(_))
    | (TsConstAssertion(_), TsConstAssertion(_))
    | (TsNonNull(_), TsNonNull(_))
    | (TsTypeCast(_), TsTypeCast(_))
    | (TsAs(_), TsAs(_))
    | (PrivateName(_), PrivateName(_))
    | (OptChain(_), OptChain(_))
    | (Invalid(_), Invalid(_)) => expr1 == expr2,
    _ => false,
  }
}

fn append_test(appeared_conditions: &mut Vec<Vec<Vec<Expr>>>, expr: Expr) {
  //let conditions_to_check = match expr {
  //Expr::Bin(BinExpr { ref op, .. }) if *op == BinaryOp::LogicalAnd => {
  //let mut v = vec![expr.clone()];
  //v.append(&mut split_by_and(expr));
  //v
  //}
  //_ => vec![expr],
  //};
  appeared_conditions.push(split_by_or_then_and(expr));
  //appeared_conditions.extend(conditions_to_check.iter().map(|c| {
  //split_by_or(c.clone())
  //.into_iter()
  //.map(split_by_and)
  //.collect()
  //}));
}

fn split_by_or_then_and(expr: Expr) -> Vec<Vec<Expr>> {
  split_by_or(expr).into_iter().map(split_by_and).collect()
}

//#[derive(Debug)]
//struct IfElseEqualChecker(SpanDropped<Expr>);

//impl PartialEq for IfElseEqualChecker {
//fn eq(&self, other: &Self) -> bool {
//let IfElseEqualChecker(ref self_sd) = self;
//let IfElseEqualChecker(ref other_sd) = other;

//equal_in_if_else(&self_sd.as_ref(), &other_sd.as_ref())
//}
//}

//impl Eq for IfElseEqualChecker {}

//impl PartialOrd for IfElseEqualChecker {
//fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
//Some(self.cmp(other))
//}
//}

//impl Ord for IfElseEqualChecker {
//fn cmp(&self, other: &Self) -> Ordering {
//if self == other {
//Ordering::Equal
//} else {
//let IfElseEqualChecker(ref self_sd) = self;
//let IfElseEqualChecker(ref other_sd) = other;
//self_sd.cmp(other_sd)
//}
//}
//}

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
    assert_lint_ok::<NoDupeElseIf>("if (a || b) {} else if (a || c) {}");
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
      vec![34, 49, 64],
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
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (a) {}", 24);
    assert_lint_err_n::<NoDupeElseIf>(
      "if (a || b) {} else if (a) {} else if (b) {}",
      vec![24, 39],
    );
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (b || a) {}", 24);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (a || b) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b) {} else if (c || d) {} else if (a || d) {}",
      44,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if ((a === b && fn(c)) || d) {} else if (fn(c) && a === b) {}",
      41,
    );
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a && b) {}", 19);
    assert_lint_err::<NoDupeElseIf>("if (a && b) {} else if (b && a) {}", 24);
    assert_lint_err::<NoDupeElseIf>(
      "if (a && b) {} else if (a && b && c) {}",
      24,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || c) {} else if (a && b || c) {}",
      24,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c && a || b) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c && (a || b)) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b && c) {} else if (d && (a || e && c && b)) {}",
      39,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b && c) {} else if (b && c && d) {}",
      29,
    );
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (b && c) {}", 24);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if ((a || b) && c) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if ((a && (b || c)) || d) {} else if ((c || b) && e && a) {}",
      38,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a && b || b && c) {} else if (a && b && c) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b && c) {} else if (d && (c && e && b || a)) {}",
      39,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || (b && (c || d))) {} else if ((d || c) && b) {}",
      38,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b) {} else if ((b || a) && c) {}",
      24,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b) {} else if (c) {} else if (d) {} else if (b && (a || c)) {}",
      54,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || b || c) {} else if (a || (b && d) || (c && e)) {}",
      29,
    );
    assert_lint_err::<NoDupeElseIf>(
      "if (a || (b || c)) {} else if (a || (b && c)) {}",
      31,
    );
    assert_lint_err::<NoDupeElseIf>("if (a || b) {} else if (c) {} else if (d) {} else if ((a || c) && (b || d)) {}", 54);
    assert_lint_err::<NoDupeElseIf>(
      "if (a) {} else if (b) {} else if (c && (a || d && b)) {}",
      34,
    );
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a || a) {}", 19);
    assert_lint_err::<NoDupeElseIf>("if (a || a) {} else if (a || a) {}", 24);
    assert_lint_err::<NoDupeElseIf>("if (a || a) {} else if (a) {}", 24);
    assert_lint_err::<NoDupeElseIf>("if (a) {} else if (a && a) {}", 19);
    assert_lint_err::<NoDupeElseIf>("if (a && a) {} else if (a && a) {}", 24);
    assert_lint_err::<NoDupeElseIf>("if (a && a) {} else if (a) {}", 24);
  }
}
