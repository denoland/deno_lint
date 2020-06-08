// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![deny(warnings)]

#[macro_use]
extern crate lazy_static;

pub mod diagnostic;
pub mod linter;
pub mod rules;

mod scopes;
mod swc_util;

#[cfg(test)]
mod test_util;

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ignore_comments_lint() {
    assert_lint_ok::<NoExplicitAny>(
      r#"
// deno-lint-ignore no-explicit-any
function foo(): any {
  // pass
}"#,
      );
    assert_lint_err_on_line::<BanTsIgnore>(
      r#"
// deno-lint-ignore no-explicit-any
function foo(): string {
  return "foo";
}
    "#,
      3,
      2,
    );
  }
}
