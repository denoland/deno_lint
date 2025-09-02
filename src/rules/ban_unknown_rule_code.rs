// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::{
  tags::{self, Tags},
  Program,
};

/// This is a dummy struct just for having the docs.
/// The actual implementation resides in [`Context`].
#[derive(Debug)]
pub struct BanUnknownRuleCode;

pub(crate) const CODE: &str = "ban-unknown-rule-code";

impl LintRule for BanUnknownRuleCode {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    _context: &mut Context,
    _program: Program<'_>,
  ) {
    // noop
  }

  // This rule should be run second to last.
  fn priority(&self) -> u32 {
    u32::MAX - 1
  }
}
