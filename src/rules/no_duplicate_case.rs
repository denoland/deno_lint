// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use derive_more::Display;
use std::collections::HashSet;
use swc_common::Spanned;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

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

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoDuplicateCaseVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows using the same case clause in a switch statement more than once

When you reuse a case test expression in a `switch` statement, the duplicate case will
never be reached meaning this is almost always a bug.
    
### Invalid:
```typescript
const someText = "a";
switch (someText) {
  case "a":
    break;
  case "b":
    break;
  case "a": // duplicate test expression
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

struct NoDuplicateCaseVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoDuplicateCaseVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoDuplicateCaseVisitor<'c> {
  noop_visit_type!();

  fn visit_switch_stmt(
    &mut self,
    switch_stmt: &swc_ecmascript::ast::SwitchStmt,
    _parent: &dyn Node,
  ) {
    // Works like in ESLint - by comparing text repr of case statement
    let mut seen: HashSet<String> = HashSet::new();

    for case in &switch_stmt.cases {
      if let Some(test) = &case.test {
        let span = test.span();
        let test_txt = self
          .context
          .source_map
          .span_to_snippet(span)
          .unwrap()
          .replace(|c: char| c.is_whitespace(), "");
        if !seen.insert(test_txt) {
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

      // TODO(magurotuna): to pass the following tests, we have to remove comments and whitespaces somehow.
      // "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p.p.p1: break; case p. p // comment\n .p1: break; default: break;}": [
      //   {
      //     col: 68,
      //     message: NoDuplicateCaseMessage::Unexpected,
      //     hint: NoDuplicateCaseHint::RemoveOrRename,
      //   }
      // ],
      // "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p .p\n/* comment */\n.p1: break; case p.p.p1: break; default: break;}": [
      //   {
      //     line: 3,
      //     col: 12,
      //     message: NoDuplicateCaseMessage::Unexpected,
      //     hint: NoDuplicateCaseHint::RemoveOrRename,
      //   }
      // ],
      // "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p .p\n/* comment */\n.p1: break; case p. p // comment\n .p1: break; default: break;}": [
      //   {
      //     line: 3,
      //     col: 12,
      //     message: NoDuplicateCaseMessage::Unexpected,
      //     hint: NoDuplicateCaseHint::RemoveOrRename,
      //   }
      // ],
      // "var a = 1, p = {p: {p1: 1, p2: 1}}; switch (a) {case p.p.p1: break; case p. p // comment\n .p1: break; case p .p\n/* comment */\n.p1: break; default: break;}": [
      //   {
      //     line: 1,
      //     col: 68,
      //     message: NoDuplicateCaseMessage::Unexpected,
      //     hint: NoDuplicateCaseHint::RemoveOrRename,
      //   },
      //   {
      //     line: 2,
      //     col: 13,
      //     message: NoDuplicateCaseMessage::Unexpected,
      //     hint: NoDuplicateCaseHint::RemoveOrRename,
      //   }
      // ],
      // "var a = 1, f = function(s) { return { p1: s } }; switch (a) {case f(\na + 1 // comment\n).p1: break; case f(a+1)\n.p1: break; default: break;}": [
      //   {
      //     line: 3,
      //     col: 13,
      //     message: NoDuplicateCaseMessage::Unexpected,
      //     hint: NoDuplicateCaseHint::RemoveOrRename,
      //   }
      // ],
    };
  }
}
