// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse, TraverseFlow};
use swc_common::Spanned;

pub struct NoWith;

const CODE: &str = "no-with";
const MESSAGE: &str = "`with` statement is not allowed";

impl LintRule for NoWith {
  fn new() -> Box<Self> {
    Box::new(NoWith)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    NoWithHandler.traverse(program, context);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the usage of `with` statements.

The `with` statement is discouraged as it may be the source of confusing bugs
and compatibility issues. For more details, see [with - JavaScript | MDN].

[with - JavaScript | MDN]: https://developer.mozilla.org/en-US/docs/Web/JavaScript/Reference/Statements/with

### Invalid:

```typescript
with (someVar) {
  console.log("foo");
}
```
"#
  }
}

struct NoWithHandler;

impl Handler for NoWithHandler {
  fn with_stmt(
    &mut self,
    with_stmt: &dprint_swc_ecma_ast_view::WithStmt,
    ctx: &mut Context,
  ) -> TraverseFlow {
    ctx.add_diagnostic(with_stmt.span(), CODE, MESSAGE);
    TraverseFlow::Continue
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_with_invalid() {
    assert_lint_err! {
      NoWith,
      "with (someVar) { console.log('asdf'); }": [{ col: 0, message: MESSAGE }],
    }
  }
}
