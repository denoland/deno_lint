// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use swc_common::Span;

pub struct BanUntaggedIgnore;

const CODE: &str = "ban-untagged-ignore";

impl LintRule for BanUntaggedIgnore {
  fn new() -> Box<Self> {
    Box::new(BanUntaggedIgnore)
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
    _program: dprint_swc_ecma_ast_view::Program,
  ) {
    let mut violated_spans: Vec<Span> = context
      .file_ignore_directive()
      .iter()
      .filter_map(|d| d.ignore_all().then(|| d.span()))
      .collect();

    violated_spans.extend(
      context
        .line_ignore_directives()
        .values()
        .filter_map(|d| d.ignore_all().then(|| d.span())),
    );

    for span in violated_spans {
      context.add_diagnostic_with_hint(
        span,
        CODE,
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
