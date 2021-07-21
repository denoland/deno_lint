use super::{Context, LintRule, ProgramRef};

/// This is a dummy struct just for having the docs.
/// The actual implementation resides in [`Context`].
pub struct BanUnusedIgnore;

impl LintRule for BanUnusedIgnore {
  fn new() -> Box<Self> {
    Box::new(BanUnusedIgnore)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "ban-unused-ignore"
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
    r#"Warns unused ignore directives

We sometimes have to suppress and ignore lint errors for some reasons and we can
do so using [ignore directives](https://lint.deno.land/ignoring-rules).

In some cases, however, like after refactoring, we may end up having ignore
directives that are no longer necessary. Such superfluous ignore directives are
likely to confuse future code readers, and to make matters worse, might hide
future lint errors unintentionally. To prevent such situations, this rule
detects unused, superfluous ignore directives.

### Invalid:

```typescript
// Actually this line is valid since `export` means "used",
// so this directive is superfluous
// deno-lint-ignore no-unused-vars
export const foo = 42;
```

### Valid:

```typescript
export const foo = 42;
```
"#
  }
}
