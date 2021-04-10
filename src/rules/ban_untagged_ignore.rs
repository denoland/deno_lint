// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use swc_common::Span;

pub struct BanUntaggedIgnore;

impl LintRule for BanUntaggedIgnore {
  fn new() -> Box<Self> {
    Box::new(BanUntaggedIgnore)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "ban-untagged-ignore"
  }

  fn lint_program(&self, context: &mut Context, _program: ProgramRef<'_>) {
    let violated_spans: Vec<Span> = context
      .ignore_directives()
      .iter()
      .filter_map(|d| {
        if d.codes.is_empty() {
          Some(d.span)
        } else {
          None
        }
      })
      .collect();

    for span in violated_spans {
      context.add_diagnostic_with_hint(
        span,
        "ban-untagged-ignore",
        "Ignore directive requires lint rule name(s)",
        "Add one or more lint rule names.  E.g. // deno-lint-ignore adjacent-overload-signatures",
      )
    }
  }

  fn docs(&self) -> &'static str {
    r#"Requires `deno-lint-ignore` to be annotated with one or more rule names.

Ignoring all rules can mask unexpected or future problems. Therefore you need to explicitly specify which rule(s) are to be ignored.

### Invalid:
```typescript
// deno-lint-ignore
export function duplicateArgumentsFn(a, b, a) { }
```

### Valid:
```typescript
// deno-lint-ignore no-dupe-args
export function duplicateArgumentsFn(a, b, a) { }
```
"#
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ban_ts_ignore_valid() {
    assert_lint_ok! {
      BanUntaggedIgnore,
      r#"
// deno-lint-ignore some-code some-code-2
function bar() {
  // pass
}
    "#,
    };
  }

  #[test]
  fn ban_ts_ignore_invalid() {
    assert_lint_err! {
      BanUntaggedIgnore,
      r#"
// deno-lint-ignore
function foo() {
  // pass
}
      "#: [
        {
          line: 2,
          col: 0,
          message: "Ignore directive requires lint rule name(s)",
          hint: "Add one or more lint rule names.  E.g. // deno-lint-ignore adjacent-overload-signatures",
        }
      ]
    };
  }
}
