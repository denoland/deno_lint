// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use regex::Regex;
use swc_common::{BytePos, Span, Spanned};
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::collections::HashMap;
use std::sync::Arc;

lazy_static! {
  static ref MSG_MAP: HashMap<&'static str, &'static str> = {
    let mut map = HashMap::new();
    map.insert(
      "call",
      "Unexpected newline between function and ( of function call",
    );
    map.insert(
      "member",
      "Unexpected newline between object and [ of property access",
    );
    map.insert(
      "div",
      "Unexpected newline between numerator and division operator",
    );
    map.insert(
      "template",
      "Unexpected newline between template tag and template literal",
    );
    map
  };
  static ref SLASH_AND_FLAGS: regex::Regex =
    Regex::new(r"^/[gimsuy]+(?:[\W].*)?$").unwrap();
}

pub struct NoUnexpectedMultiline;

impl LintRule for NoUnexpectedMultiline {
  fn new() -> Box<Self> {
    Box::new(NoUnexpectedMultiline)
  }

  fn code(&self) -> &'static str {
    "no-unexpected-multiline"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoUnexpectedMultilineVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoUnexpectedMultilineVisitor {
  context: Arc<Context>,
  current_node_is_optional: bool,
}

impl NoUnexpectedMultilineVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self {
      context,
      current_node_is_optional: false,
    }
  }

  fn check_for_break_after(&self, outer: Span, after: Span, msg: &str) {
    let source_map = &self.context.source_map;
    let before_paren = outer.trim_start(after).unwrap();
    let temp_span = source_map
      .span_take_while(before_paren, |c| *c == ')' || c.is_whitespace());
    let left_paren = before_paren.trim_start(temp_span).unwrap();
    let line1 = source_map.lookup_char_pos(after.hi()).line;
    let line2 = source_map.lookup_char_pos(left_paren.lo()).line;
    if line1 != line2 {
      self.context.add_diagnostic(
        left_paren.with_lo(left_paren.lo() + BytePos(1)),
        "no-unexpected-multiline",
        MSG_MAP[msg],
      );
    }
  }
}

impl Visit for NoUnexpectedMultilineVisitor {
  fn visit_opt_chain_expr(
    &mut self,
    opt_chain_expr: &swc_ecmascript::ast::OptChainExpr,
    parent: &dyn Node,
  ) {
    self.current_node_is_optional = true;
    self.visit_expr(&opt_chain_expr.expr, parent);
  }

  fn visit_bin_expr(
    &mut self,
    bin_expr: &swc_ecmascript::ast::BinExpr,
    _parent: &dyn Node,
  ) {
    self.visit_expr(&bin_expr.left, bin_expr);
    self.visit_expr(&bin_expr.right, bin_expr);

    if let swc_ecmascript::ast::BinaryOp::Div = bin_expr.op {
      if let swc_ecmascript::ast::Expr::Bin(inner_bin_expr) = &*bin_expr.left {
        let temp_span = bin_expr.span.trim_start(bin_expr.left.span()).span();
        let source_map = &self.context.source_map;
        let slash_and_flags = source_map
          .span_to_snippet(
            temp_span
              .trim_start(source_map.span_take_while(temp_span, |c| *c != '/'))
              .unwrap(),
          )
          .unwrap();
        if !matches!(inner_bin_expr.op, swc_ecmascript::ast::BinaryOp::Div)
          || !SLASH_AND_FLAGS.is_match(&slash_and_flags)
        {
          return;
        }
        self.check_for_break_after(
          bin_expr.span,
          inner_bin_expr.left.span(),
          "div",
        );
      }
    }
  }

  fn visit_call_expr(
    &mut self,
    call_expr: &swc_ecmascript::ast::CallExpr,
    _parent: &dyn Node,
  ) {
    let optional = self.current_node_is_optional;
    self.current_node_is_optional = false;

    if let swc_ecmascript::ast::ExprOrSuper::Expr(expr) = &call_expr.callee {
      self.visit_expr(expr, call_expr);
    }
    for arg in &call_expr.args {
      self.visit_expr(&arg.expr, call_expr);
    }
    if call_expr.args.is_empty() || optional {
      return;
    }

    self.check_for_break_after(call_expr.span, call_expr.callee.span(), "call");
  }

  fn visit_member_expr(
    &mut self,
    member_expr: &swc_ecmascript::ast::MemberExpr,
    _parent: &dyn Node,
  ) {
    let optional = self.current_node_is_optional;
    self.current_node_is_optional = false;

    if let swc_ecmascript::ast::ExprOrSuper::Expr(expr) = &member_expr.obj {
      self.visit_expr(expr, member_expr);
    }
    if !member_expr.computed || optional {
      return;
    }
    self.check_for_break_after(
      member_expr.span(),
      member_expr.obj.span(),
      "member",
    );
  }

  fn visit_tagged_tpl(
    &mut self,
    tagged_tpl: &swc_ecmascript::ast::TaggedTpl,
    _parent: &dyn Node,
  ) {
    if tagged_tpl.quasis.is_empty() {
      return;
    }

    let tag = &tagged_tpl.tag;
    let tag_end_loc =
      self.context.source_map.lookup_char_pos((&*tag).span().hi());

    let quasi = &tagged_tpl.quasis[0];
    let quasi_start_loc =
      self.context.source_map.lookup_char_pos(quasi.span().lo());
    if tag_end_loc.line != quasi_start_loc.line {
      self.context.add_diagnostic(
        quasi.span(),
        "no-unexpected-multiline",
        MSG_MAP["template"],
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_unexpected_multiline_valid() {
    assert_lint_ok::<NoUnexpectedMultiline>(
      r#"var foo = bar;
(1 || 2).baz();

var foo = bar
;(1 || 2).baz()

foo<string>
("").length;

(foo
)(bar);

(foo).callback
?.
(bar)

var hello = 'world';
[1, 2, 3].forEach(addNumber);

(array
)[1]

var hello = 'world'
void [1, 2, 3].forEach(addNumber);

var a = b
?.[a, b, c].forEach(doSomething);

var a = b?.
[a, b, c].forEach(doSomething);

function foo() { return ""; }
foo
().length

function foo<T>(bar: T): T { return bar; }
`hello`

let x = function() {};
`hello`

let tag = function() {}
tag `hello`

let a = b/
abc/g;

let x = a
/foo/ g"#,
    );
  }

  #[test]
  fn no_unexpected_multiline_invalid() {
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"var foo = bar
(1 || 2).baz();"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"foo
<string>
("")"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"(foo)
(abc).length"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"foo(bar
(x), baz)"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"(foo).callback
(bar)?.baz"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"foo.bar?.baz.bay
[boo];"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"var hello = 'world'
  [1, 2, 3].forEach(addNumber);"#,
      2,
      3,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"var a = b
/
abc/g-a"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"let x = foo
/regex/g.foo(bar)"#,
      2,
      1,
    );
  }

  #[test]
  fn no_unexpected_multiline_invalid_tagged_tpl() {
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"let x = function() {}
`hello`"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"let x = function() {}
x
`hello`"#,
      3,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"(foo)
`hello`"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"bar<string>
`${x} hello`"#,
      2,
      1,
    );
    assert_lint_err_on_line::<NoUnexpectedMultiline>(
      r#"const x = aaaa<
  test
>/*
test
*/`foo`"#,
      5,
      3,
    );
  }
}
