use super::{Context, LintRule, ProgramRef};

/// This is a dummy struct just for having the docs.
/// The actual implementation resides in [`Context`].
pub struct BanUnknownRuleCode;

impl LintRule for BanUnknownRuleCode {
  fn new() -> Box<Self> {
    Box::new(BanUnknownRuleCode)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "ban-unknown-rule-code"
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
  }

  fn lint_program_with_ast_view(
    &self,
    _context: &mut Context,
    _program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    // noop
  }

  fn docs(&self) -> &'static str {
    r#"Warns the usage of unknown rule codes in ignore directives

We sometimes have to suppress and ignore lint errors for some reasons. We can do
so using [ignore directives](https://lint.deno.land/ignoring-rules) with rule
names that should be ignored like so:

```typescript
// deno-lint-ignore no-explicit-any no-unused-vars
const foo: any = 42;
```

This rule checks for the validity of the specified rule names (i.e. whether
`deno_lint` provides the rule or not).

### Invalid:

```typescript
// typo
// deno-lint-ignore no-extra-sem
export const a = 42;;

// unknown rule name
// deno-lint-ignore UNKNOWN_RULE_NAME
const b = "b";
```

### Valid:

```typescript
// deno-lint-ignore no-extra-semi
export const a = 42;;

// deno-lint-ignore no-unused-vars
const b = "b";
```
"#
  }
}
