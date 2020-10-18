// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use swc_ecmascript::ast::{
  ArrowExpr, BlockStmt, BlockStmtOrExpr, Constructor, Function, Module,
  SwitchStmt,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

pub struct NoEmpty;

impl LintRule for NoEmpty {
  fn new() -> Box<Self> {
    Box::new(NoEmpty)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-empty"
  }

  fn lint_module(&self, context: &mut Context, module: &Module) {
    let mut visitor = NoEmptyVisitor::new(context);
    visitor.visit_module(module, module);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of empty block statements.

Empty block statements are legal but often represent that something was missed and can make code less readable. This rule ignores block statements that only contain comments. This rule also ignores empty constructors and function bodies (including arrow functions), which are covered by the `no-empty-function` rule.

### Valid:
```typescript
if (foo) {
  // empty
}
```
```typescript
while (foo) {
  /* empty */
}
```
```typescript
try {
  doSomething();
} catch (ex) {
  // continue regardless of error
}
```
```typescript
try {
  doSomething();
} finally {
  /* continue regardless of error */
}
```

### Invalid:
```typescript
if (foo) {
}
```
```typescript
while (foo) {
}
```
```typescript
switch(foo) {
}
```
```typescript
try {
  doSomething();
} catch(ex) {

} finally {

}
```"#
  }
}

struct NoEmptyVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoEmptyVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoEmptyVisitor<'c> {
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
      if !block_stmt.contains_comments(&self.context) {
        self.context.add_diagnostic(
          block_stmt.span,
          "no-empty",
          "Empty block statement",
        );
      }
    } else {
      block_stmt.visit_children_with(self);
    }
  }

  fn visit_switch_stmt(&mut self, switch: &SwitchStmt, _parent: &dyn Node) {
    if switch.cases.is_empty() {
      self.context.add_diagnostic(
        switch.span,
        "no-empty",
        "Empty switch statement",
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
      .leading_comments
      .values()
      .flatten()
      .any(|comment| self.span.contains(comment.span))
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_empty_valid() {
    assert_lint_ok_macro! {
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
    };
  }

  #[test]
  fn it_fails_for_an_empty_if_block() {
    assert_lint_err::<NoEmpty>("if (foo) { }", 9);
  }

  #[test]
  fn it_fails_for_an_empty_block_with_preceding_comments() {
    assert_lint_err_on_line::<NoEmpty>(
      r#"
// This is an empty block
if (foo) { }
      "#,
      3,
      9,
    );
  }

  #[test]
  fn it_fails_for_an_empty_while_block() {
    assert_lint_err::<NoEmpty>("while (foo) { }", 12);
  }

  #[test]
  fn it_fails_for_an_empty_do_while_block() {
    assert_lint_err::<NoEmpty>("do { } while (foo);", 3);
  }

  #[test]
  fn it_fails_for_an_empty_for_block() {
    assert_lint_err::<NoEmpty>("for(;;) { }", 8);
  }

  #[test]
  fn it_fails_for_an_empty_for_in_block() {
    assert_lint_err::<NoEmpty>("for(var foo in bar) { }", 20);
  }

  #[test]
  fn it_fails_for_an_empty_for_of_block() {
    assert_lint_err::<NoEmpty>("for(var foo of bar) { }", 20);
  }

  #[test]
  fn it_fails_for_an_empty_switch_block() {
    assert_lint_err::<NoEmpty>("switch (foo) { }", 0);
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_block() {
    assert_lint_err_n::<NoEmpty>("try { } catch (err) { }", vec![4, 20]);
  }

  #[test]
  fn it_fails_for_an_empty_try_catch_finally_block() {
    assert_lint_err_n::<NoEmpty>(
      "try { } catch (err) { } finally { }",
      vec![4, 20, 32],
    );
  }

  #[test]
  fn it_fails_for_a_nested_empty_if_block() {
    assert_lint_err::<NoEmpty>("if (foo) { if (bar) { } }", 20);
  }

  #[test]
  fn it_fails_for_a_nested_empty_while_block() {
    assert_lint_err::<NoEmpty>("if (foo) { while (bar) { } }", 23);
  }

  #[test]
  fn it_fails_for_a_nested_empty_do_while_block() {
    assert_lint_err::<NoEmpty>("if (foo) { do { } while (bar); }", 14);
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_block() {
    assert_lint_err::<NoEmpty>("if (foo) { for(;;) { } }", 19);
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_in_block() {
    assert_lint_err::<NoEmpty>("if (foo) { for(var bar in foo) { } }", 31);
  }

  #[test]
  fn it_fails_for_a_nested_empty_for_of_block() {
    assert_lint_err::<NoEmpty>("if (foo) { for(var bar of foo) { } }", 31);
  }

  #[test]
  fn it_fails_for_a_nested_empty_switch() {
    assert_lint_err::<NoEmpty>("if (foo) { switch (foo) { } }", 11);
  }

  #[test]
  fn it_fails_for_a_nested_empty_if_block_in_switch_discriminant() {
    assert_lint_err_on_line::<NoEmpty>(
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
      "#,
      4,
      14,
    );
  }
}
