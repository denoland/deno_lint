// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use once_cell::sync::Lazy;
use regex::Regex;
use swc_ecmascript::ast::Number;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoOctal;

const CODE: &str = "no-octal";
const MESSAGE: &str = "Numeric literals beginning with `0` are not allowed";
const HINT: &str = "To express octal numbers, use `0o` as a prefix instead";

impl LintRule for NoOctal {
  fn new() -> Box<Self> {
    Box::new(NoOctal)
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
    let mut visitor = NoOctalVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows expressing octal numbers via numeric literals beginning with `0`

Octal numbers can be expressed via numeric literals with leading `0` like `042`,
but this expression often confuses programmers. That's why ECMAScript's strict
mode throws `SyntaxError` for the expression.

Since ES2015, the other prefix `0o` has been introduced as an alternative. This
new one is always encouraged to use in today's code.

### Invalid:

```typescript
const a = 042;
const b = 7 + 042;
```

### Valid:

```typescript
const a = 0o42;
const b = 7 + 0o42;
const c = "042";
```
"#
  }
}

struct NoOctalVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoOctalVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoOctalVisitor<'c, 'view> {
  fn visit_number(&mut self, literal_num: &Number, _parent: &dyn Node) {
    static OCTAL: Lazy<Regex> = Lazy::new(|| Regex::new(r"^0[0-9]").unwrap());

    let raw_number = self
      .context
      .source_map()
      .span_to_snippet(literal_num.span)
      .expect("error in loading snippet");

    if OCTAL.is_match(&raw_number) {
      self.context.add_diagnostic_with_hint(
        literal_num.span,
        CODE,
        MESSAGE,
        HINT,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_octal_valid() {
    assert_lint_ok! {
      NoOctal,
      "7",
      "\"07\"",
      "0x08",
      "-0.01",
    };
  }

  #[test]
  fn no_octal_invalid() {
    assert_lint_err! {
      NoOctal,
      "07": [{col: 0, message: MESSAGE, hint: HINT}],
      "let x = 7 + 07": [{col: 12, message: MESSAGE, hint: HINT}],

      // https://github.com/denoland/deno/issues/10954
      // Make sure it doesn't panic
      "020000000000000000000;": [{col: 0, message: MESSAGE, hint: HINT}],
    }
  }
}
