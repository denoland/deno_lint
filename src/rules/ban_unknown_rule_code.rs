// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::{Program, ProgramRef};
use std::sync::Arc;

/// This is a dummy struct just for having the docs.
/// The actual implementation resides in [`Context`].
#[derive(Debug)]
pub struct BanUnknownRuleCode;

impl LintRule for BanUnknownRuleCode {
  fn new() -> Arc<Self> {
    Arc::new(BanUnknownRuleCode)
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
    _program: Program<'_>,
  ) {
    // noop
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_unknown_rule_code.md")
  }

  // This rule should be run second to last.
  fn priority(&self) -> u32 {
    u32::MAX - 1
  }
}
