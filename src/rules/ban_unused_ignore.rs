// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::Program;

/// This is a dummy struct just for having the docs.
/// The actual implementation resides in [`Context`].
#[derive(Debug)]
pub struct BanUnusedIgnore;

impl LintRule for BanUnusedIgnore {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    "ban-unused-ignore"
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    _context: &mut Context<'a>,
    _program: &Program<'a>,
  ) {
    // noop
  }

  // This rule should be run last.
  fn priority(&self) -> u32 {
    u32::MAX
  }
}
