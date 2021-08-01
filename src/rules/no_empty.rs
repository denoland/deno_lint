// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use swc_ecmascript::ast::{
  ArrowExpr, BlockStmt, BlockStmtOrExpr, Constructor, Function, SwitchStmt,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

pub struct NoEmpty;

const CODE: &str = "no-empty";

impl LintRule for NoEmpty {
  fn new() -> Box<Self> {
    Box::new(NoEmpty)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoEmptyVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_empty.md")
  }
}

struct NoEmptyVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoEmptyVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoEmptyVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    // Empty functions shouldn't be caught by this rule.
    // Because function's body is a block statement, we're gonna
    // manually visit each member; otherwise rule would produce errors
    // for empty function body.
    if let Some(body) = &function.body {
      body.visit_children_with(self);
    }
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    // Similar to the above, empty arrow expressions shouldn't be caught.
    if let BlockStmtOrExpr::BlockStmt(block_stmt) = &arrow_expr.body {
      block_stmt.visit_children_with(self);
    }
  }

  fn visit_constructor(&mut self, cons: &Constructor, _parent: &dyn Node) {
    // Similar to the above, empty constructors shouldn't be caught.
    if let Some(body) = &cons.body {
      body.visit_children_with(self);
    }
  }

  fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _parent: &dyn Node) {
    if block_stmt.stmts.is_empty() {
      if !block_stmt.contains_comments(self.context) {
        self.context.add_diagnostic_with_hint(
          block_stmt.span,
          CODE,
          "Empty block statement",
          "Add code or comment to the empty block",
        );
      }
    } else {
      block_stmt.visit_children_with(self);
    }
  }

  fn visit_switch_stmt(&mut self, switch: &SwitchStmt, _parent: &dyn Node) {
    if switch.cases.is_empty() {
      self.context.add_diagnostic_with_hint(
        switch.span,
        CODE,
        "Empty switch statement",
        "Add case statement(s) to the empty switch, or remove",
      );
    }
    switch.visit_children_with(self);
  }
}

trait ContainsComments {
  fn contains_comments(&self, context: &Context) -> bool;
}

impl ContainsComments for BlockStmt {
  fn contains_comments(&self, context: &Context) -> bool {
    context
      .all_comments()
      .any(|comment| self.span.contains(comment.span))
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_valid() {
    assert_lint_ok! {
      NoEmpty,
      r#"function foobar() {}"#,
      r#"
class Foo {
  constructor() {}
}
      "#,
      r#"if (foo) { var bar = ""; }"#,
      r#"
if (foo) {
  // This block is not empty
}
    "#,
      r#"
if (foo) {
  /* This block is not empty */
}
    "#,
      r#"
    switch (foo) {
      case bar:
        break;
    }
      "#,
      r#"
if (foo) {
  if (bar) {
    var baz = "";
  }
}
      "#,
      "const testFunction = (): void => {};",
      r#"
      switch (foo) {
        case 1:
        case 2:
          break;
        default:
          return 1;
      }
      "#,

      // https://github.com/denoland/deno_lint/issues/469
      "try { foo(); } catch { /* pass */ }",
      r#"
try {
  foo();
} catch { // pass
}
      "#,
    };
  }

  #[test]
  fn no_empty_invalid() {
    assert_lint_err! {
      NoEmpty,
      "if (foo) { }": [
        {
          col: 9,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      r#"
// This is an empty block
if (foo) { }
      "#: [
        {
          line: 3,
          col: 9,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "while (foo) { }": [
        {
          col: 12,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "do { } while (foo);": [
        {
          col: 3,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "for(;;) { }": [
        {
          col: 8,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "for(var foo in bar) { }": [
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "for(var foo of bar) { }": [
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "switch (foo) { }": [
        {
          col: 0,
          message: "Empty switch statement",
          hint: "Add case statement(s) to the empty switch, or remove",
        }
      ],
      "try { } catch (err) { }": [
        {
          col: 4,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        },
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "try { } catch (err) { } finally { }": [
        {
          col: 4,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        },
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        },
        {
          col: 32,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { if (bar) { } }": [
        {
          col: 20,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { while (bar) { } }": [
        {
          col: 23,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { do { } while (bar); }": [
        {
          col: 14,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { for(;;) { } }": [
        {
          col: 19,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { for(var bar in foo) { } }": [
        {
          col: 31,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { for(var bar of foo) { } }": [
        {
          col: 31,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "if (foo) { switch (foo) { } }": [
        {
          col: 11,
          message: "Empty switch statement",
          hint: "Add case statement(s) to the empty switch, or remove",
        }
      ],
      r#"
switch (
  (() => {
    if (cond) {}
    return 42;
  })()
) {
  case 1:
    foo();
    break;
  default:
    bar();
    break;
}
      "#: [
        {
          line: 4,
          col: 14,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],

      // https://github.com/denoland/deno_lint/issues/469
      "try { foo(); } catch /* outside block */{ }": [
        {
          col: 40,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      "try { foo(); } catch { }/* outside block */": [
        {
          col: 21,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ],
      r#"
try {
  foo();
} catch {
}// pass
      "#: [
        {
          line: 4,
          col: 8,
          message: "Empty block statement",
          hint: "Add code or comment to the empty block",
        }
      ]
    };
  }
}
