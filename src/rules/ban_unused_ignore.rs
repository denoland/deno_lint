// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::{tags::{self, Tags}, Program};

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

  fn lint_program_with_ast_view(
    &self,
    _context: &mut Context,
    _program: Program<'_>,
  ) {
    // noop
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_unused_ignore.md")
  }

  // This rule should be run last.
  fn priority(&self) -> u32 {
    u32::MAX
  }
}
