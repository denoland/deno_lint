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

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/ban_unused_ignore.md")
  }
}
