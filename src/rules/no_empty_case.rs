// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintFix, LintFixChange};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::NodeTrait;
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoEmptyCase;

const CODE: &str = "no-empty-case";
const MESSAGE: &str = "An empty switch-case statement should be removed.";
const HINT: &str = "Remove empty switch-case";

impl LintRule for NoEmptyCase {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoEmptyCaseHandler.traverse(program, context);
  }
}

struct NoEmptyCaseHandler;

impl Handler for NoEmptyCaseHandler {
  fn switch_case(
    &mut self,
    switch_case: &deno_ast::view::SwitchCase<'_>,
    context: &mut Context,
  ) {
    if let [stmt] = switch_case.cons.iter().as_slice() {
      let text = stmt.text();
      if text[1..text.len() - 1].trim().is_empty() {
        let range = stmt.range();
        let change = LintFixChange {
          new_text: "".into(),
          range,
        };
        context.add_diagnostic_with_fixes(
          range,
          CODE,
          MESSAGE,
          Some(HINT.into()),
          vec![LintFix {
            description: HINT.into(),
            changes: vec![change],
          }],
        );
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid() {
    assert_lint_ok! {
      NoEmptyCase,
      "switch (n) {case 0:case 1:{// todo \n}}",
      "switch (n) {case 0:case 1:default:}",
    };
  }

  #[test]
  fn invalid() {
    assert_lint_err! {
      NoEmptyCase,
      "switch (n) {case 0:case 1:{}}": [{
          col: 26,
          line: 1,
          message: MESSAGE,
          hint: HINT,
          fix: (
            HINT,
            "switch (n) {case 0:case 1:}"
          ),
        }
      ],
      "switch (n) {case 0:case 1:default:{}}": [{
        col: 34,
        line: 1,
        message: MESSAGE,
        hint: HINT,
        fix: (
          HINT,
          "switch (n) {case 0:case 1:default:}"
        ),
      }
    ],
    }
  }
}
