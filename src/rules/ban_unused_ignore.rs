// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::ignore_directives::CodeStatus;
use crate::ignore_directives::LineIgnoreDirective;
use crate::{Program, ProgramRef};
use std::sync::Arc;
use std::collections::HashSet;

#[derive(Debug)]
pub struct BanUnusedIgnore;

const CODE: &str = "ban-unused-ignore";

impl LintRule for BanUnusedIgnore {
  fn new() -> Arc<Self> {
    Arc::new(BanUnusedIgnore)
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
    _program: Program<'_>,
  ) {
    // If there's a file-level ignore directive containing `ban-unused-ignore`,
    // exit without running this rule.
    if context
      .file_ignore_directive()
      .map_or(false, |file_ignore| file_ignore.has_code(CODE))
    {
      return;
    }

    let executed_builtin_codes = context.rule_codes().to_owned();
    let plugin_codes = context.plugin_codes().to_owned();

    let is_unused_code = |&(code, status): &(&String, &CodeStatus)| {
      let is_unknown = !executed_builtin_codes.contains(code.as_str())
        && !plugin_codes.contains(code.as_str());
      !status.used && !is_unknown
    };
    
    if let Some(file_ignore) = context.file_ignore_directive() {
      let span = file_ignore.span();
      let unused_codes: Vec<String> = file_ignore.codes().iter().filter(is_unused_code).map(|u| u.0.to_string()).collect();
      for unused_code in unused_codes {
        context.add_diagnostic(
          span,
          CODE,
          format!("Ignore for code \"{}\" was not used.", unused_code),
        );
      }
    }

    let line_ignore_directives: Vec<LineIgnoreDirective> = context
      .line_ignore_directives()
      .clone()
      .into_values()
      .map(|v| v.clone())
      .collect();

    for line_ignore in line_ignore_directives {
      // We do nothing special even if the line-level ignore directive contains
      // `ban-unused-ignore`. `ban-unused-ignore` can be ignored only via the
      // file-level directive.

      let span = line_ignore.span();
      let unused_codes: Vec<String> = line_ignore.codes().iter().filter(is_unused_code).map(|u| u.0.to_string()).collect();

      for unused_code in unused_codes {
        context.add_diagnostic(
          span,
          CODE,
          format!("Ignore for code \"{}\" was not used.", unused_code),
        );
      }
    }
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
