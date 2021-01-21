// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use derive_more::Display;
use swc_ecmascript::ast::DebuggerStmt;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoDebugger;

const CODE: &str = "no-debugger";

#[derive(Display)]
enum NoDebuggerMessage {
  #[display(fmt = "`debugger` statement is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoDebuggerHint {
  #[display(fmt = "Remove the `debugger` statement")]
  Remove,
}

impl LintRule for NoDebugger {
  fn new() -> Box<Self> {
    Box::new(NoDebugger)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoDebuggerVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the use of the `debugger` statement

`debugger` is a statement which is meant for stopping the javascript execution
environment and start the debugger at the statement.  Modern debuggers and tooling
no longer need this statement and leaving it in can cause the execution of your
code to stop in production.
    
### Invalid:
```typescript
function isLongString(x: string) {
  debugger;
  return x.length > 100;
}
```

### Valid:
```typescript
function isLongString(x: string) {
  return x.length > 100;  // set breakpoint here instead
}
```
"#
  }
}
struct NoDebuggerVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoDebuggerVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoDebuggerVisitor<'c> {
  noop_visit_type!();

  fn visit_debugger_stmt(
    &mut self,
    debugger_stmt: &DebuggerStmt,
    _parent: &dyn Node,
  ) {
    self.context.add_diagnostic_with_hint(
      debugger_stmt.span,
      CODE,
      NoDebuggerMessage::Unexpected,
      NoDebuggerHint::Remove,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_debugger_invalid() {
    assert_lint_err! {
      NoDebugger,
      r#"function asdf(): number { console.log("asdf"); debugger; return 1; }"#: [
        {
          col: 47,
          message: NoDebuggerMessage::Unexpected,
          hint: NoDebuggerHint::Remove,
        }
      ]
    };
  }
}
