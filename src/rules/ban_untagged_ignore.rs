// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;

use std::sync::Arc;

pub struct BanUntaggedIgnore;

impl LintRule for BanUntaggedIgnore {
  fn new() -> Box<Self> {
    Box::new(BanUntaggedIgnore)
  }

  fn code(&self) -> &'static str {
    "ban-untagged-ignore"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    _module: &swc_ecmascript::ast::Module,
  ) {
    for ignore_directive in &context.ignore_directives {
      if ignore_directive.codes.is_empty() {
        context.add_diagnostic(
          ignore_directive.span,
          "ban-untagged-ignore",
          "Ignore directive requires lint rule code",
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ban_ts_ignore() {
    assert_lint_err_on_line::<BanUntaggedIgnore>(
      r#"
// deno-lint-ignore
function foo() {
  // pass
}
    "#,
      2,
      0,
    );
    assert_lint_ok::<BanUntaggedIgnore>(
      r#"
// deno-lint-ignore some-code some-code-2
function bar() {
  // pass
}
    "#,
    );
  }
}
