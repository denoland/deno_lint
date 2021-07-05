// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use std::collections::HashSet;
use swc_ecmascript::ast::{Expr, SwitchStmt};
use swc_ecmascript::utils::drop_span;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoDuplicateCase;

const CODE: &str = "no-duplicate-case";

#[derive(Display)]
enum NoDuplicateCaseMessage {
  #[display(fmt = "Duplicate values in `case` are not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoDuplicateCaseHint {
  #[display(fmt = "Remove or rename the duplicate case clause")]
  RemoveOrRename,
}

impl LintRule for NoDuplicateCase {
  fn new() -> Box<Self> {
    Box::new(NoDuplicateCase)
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
    let mut visitor = NoDuplicateCaseVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows using the same case clause in a switch statement more than once

When you reuse a case test expression in a `switch` statement, the duplicate case will
never be reached meaning this is almost always a bug.

### Invalid:

```typescript
const someText = "a";
switch (someText) {
  case "a": // (1)
    break;
  case "b":
    break;
  case "a": // duplicate of (1)
    break;
  default:
    break;
}
```

### Valid:

```typescript
const someText = "a";
switch (someText) {
  case "a":
    break;
  case "b":
    break;
  case "c":
    break;
  default:
    break;
}
```
"#
  }
}

struct NoDuplicateCaseVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoDuplicateCaseVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoDuplicateCaseVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_switch_stmt(&mut self, switch_stmt: &SwitchStmt, _: &dyn Node) {
    // Check if there are duplicates by comparing span dropped expressions
    let mut seen: HashSet<Box<Expr>> = HashSet::new();

    for case in &switch_stmt.cases {
      if let Some(test) = &case.test {
        let span_dropped_test = drop_span(test.clone());
        if !seen.insert(span_dropped_test) {
          self.context.add_diagnostic_with_hint(
            case.span,
            CODE,
            NoDuplicateCaseMessage::Unexpected,
            NoDuplicateCaseHint::RemoveOrRename,
          );
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.13.0/tests/lib/rules/no-duplicate-case.js
  // MIT Licensed.

  #[test]
  fn no_duplicate_case_valid() {
    assert_lint_ok! {
      NoDuplicateCase,
      "var a = 1; switch (a) {case 1: break; case 2: break; default: break;}",
      "var a = 1; switch (a) {case 1: break; case '1': break; default: break;}",
      "var a = 1; switch (a) {case 1: break; case true: break; default: break;}",
      "var a = 1; switch (a) {default: break;}",
      "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p.p.p1: break; case p.p.p2: break; default: break;}",
      "var a = 1, f = function(b) { return b ? { p1: 1 } : { p1: 2 }; }; switch (a) {case f(true).p1: break; case f(true, false).p1: break; default: break;}",
      "var a = 1, f = function(s) { return { p1: s } }; switch (a) {case f(a + 1).p1: break; case f(a + 2).p1: break; default: break;}",
      "var a = 1, f = function(s) { return { p1: s } }; switch (a) {case f(a == 1 ? 2 : 3).p1: break; case f(a === 1 ? 2 : 3).p1: break; default: break;}",
      "var a = 1, f1 = function() { return { p1: 1 } }, f2 = function() { return { p1: 2 } }; switch (a) {case f1().p1: break; case f2().p1: break; default: break;}",
      "var a = [1,2]; switch(a.toString()){case ([1,2]).toString():break; case ([1]).toString():break; default:break;}",
      "switch(a) { case a: break; } switch(a) { case a: break; }",
      "switch(a) { case toString: break; }",

      // nested
      r#"
switch (a) {
  case 1:
    switch (b) {
      case 2:
        foo();
        break;
      default:
        bar();
        break;
    }
  default:
    break;
}
      "#,
    };
  }

  #[test]
  fn no_duplicate_case_invalid() {
    assert_lint_err! {
      NoDuplicateCase,
      r#"
const someText = "some text";
switch (someText) {
    case "a":
        break;
    case "b":
        break;
    case "a":
        break;
    default:
        break;
}
      "#: [
        {
          col: 4,
          line: 8,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1; switch (a) {case 1: break; case 1: break; case 2: break; default: break;}": [
        {
          col: 38,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = '1'; switch (a) {case '1': break; case '1': break; case '2': break; default: break;}": [
        {
          col: 42,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, one = 1; switch (a) {case one: break; case one: break; case 2: break; default: break;}": [
        {
          col: 49,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p.p.p1: break; case p.p.p1: break; default: break;}": [
        {
          col: 68,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, f = function(b) { return b ? { p1: 1 } : { p1: 2 }; }; switch (a) {case f(true).p1: break; case f(true).p1: break; default: break;}": [
        {
          col: 102,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, f = function(s) { return { p1: s } }; switch (a) {case f(a + 1).p1: break; case f(a + 1).p1: break; default: break;}": [
        {
          col: 86,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, f = function(s) { return { p1: s } }; switch (a) {case f(a === 1 ? 2 : 3).p1: break; case f(a === 1 ? 2 : 3).p1: break; default: break;}": [
        {
          col: 96,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, f1 = function() { return { p1: 1 } }; switch (a) {case f1().p1: break; case f1().p1: break; default: break;}": [
        {
          col: 82,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = [1, 2]; switch(a.toString()){case ([1, 2]).toString():break; case ([1, 2]).toString():break; default:break;}": [
        {
          col: 69,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "switch (a) { case a: case a: }": [
        {
          col: 21,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "switch (a) { case a: break; case b: break; case a: break; case c: break; case a: break; }": [
        {
          col: 43,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        },
        {
          col: 73,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, f = function(s) { return { p1: s } }; switch (a) {case f(a + 1).p1: break; case f(a+1).p1: break; default: break;}": [
        {
          col: 86,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],

      // nested
      r#"
switch (a) {
  case 1:
    switch (b) {
      case 1:
        break;
      case 1:
        break;
    }
  default:
    break;
}
      "#: [
        {
          line: 7,
          col: 6,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p.p.p1: break; case p. p // comment\n .p1: break; default: break;}": [
        {
          col: 68,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p .p\n/* comment */\n.p1: break; case p.p.p1: break; default: break;}": [
        {
          line: 3,
          col: 12,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p .p\n/* comment */\n.p1: break; case p. p // comment\n .p1: break; default: break;}": [
        {
          line: 3,
          col: 12,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p.p.p1: break; case p. p // comment\n .p1: break; case p .p\n/* comment */\n.p1: break; default: break;}": [
        {
          line: 1,
          col: 68,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        },
        {
          line: 2,
          col: 13,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
      "var a = 1, f = function(s) { return { p1: s } }; switch (a) {case f(\na + 1 // comment\n).p1: break; case f(a+1)\n.p1: break; default: break;}": [
        {
          line: 3,
          col: 13,
          message: NoDuplicateCaseMessage::Unexpected,
          hint: NoDuplicateCaseHint::RemoveOrRename,
        }
      ],
    };
  }
}
