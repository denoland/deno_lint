// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use crate::{globals::GLOBALS, swc_util::find_lhs_ids};
use deno_ast::swc::ast::Id;
use deno_ast::SourceRange;
use deno_ast::SourceRangedForSpanned;
use deno_ast::{view::*, SourceRanged};
use derive_more::Display;

#[derive(Debug)]
pub struct NoGlobalAssign;

const CODE: &str = "no-global-assign";

#[derive(Display)]
enum NoGlobalAssignMessage {
  #[display(fmt = "Assignment to global is not allowed")]
  NotAllowed,
}

#[derive(Display)]
enum NoGlobalAssignHint {
  #[display(fmt = "Remove the assignment to the global variable")]
  Remove,
}

impl LintRule for NoGlobalAssign {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    NoGlobalAssignVisitor.traverse(program, context)
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_global_assign.md")
  }
}

struct NoGlobalAssignVisitor;

impl NoGlobalAssignVisitor {
  fn check(&mut self, range: SourceRange, id: Id, ctx: &mut Context) {
    if id.1 != ctx.unresolved_ctxt() {
      return;
    }

    if ctx.scope().var(&id).is_some() {
      return;
    }

    // We only care about globals.
    let maybe_global = GLOBALS.iter().find(|(name, _)| name == &&*id.0);

    if let Some(global) = maybe_global {
      // If global can be overwritten then don't need to report anything
      if !global.1 {
        ctx.add_diagnostic_with_hint(
          range,
          CODE,
          NoGlobalAssignMessage::NotAllowed,
          NoGlobalAssignHint::Remove,
        );
      }
    }
  }
}

impl Handler for NoGlobalAssignVisitor {
  fn assign_expr(&mut self, e: &AssignExpr, ctx: &mut Context) {
    let idents: Vec<deno_ast::swc::ast::Ident> = find_lhs_ids(&e.left);

    for ident in idents {
      self.check(ident.range(), ident.to_id(), ctx);
    }
  }

  fn update_expr(&mut self, e: &UpdateExpr, ctx: &mut Context) {
    if let Expr::Ident(i) = e.arg {
      self.check(e.range(), i.to_id(), ctx);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_global_assign_valid() {
    assert_lint_ok! {
      NoGlobalAssign,
      "string = 'hello world';",
      "var string;",
      "top = 0;",
      "require = 0;",
      "onmessage = function () {};",
      "let Array = 0; Array = 42;",
      r#"
let Boolean = true;
function foo() {
  Boolean = false;
}
      "#,
    };
  }

  #[test]
  fn no_global_assign_invalid() {
    assert_lint_err! {
      NoGlobalAssign,
      "String = 'hello world';": [
        {
          col: 0,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      "String++;": [
        {
          col: 0,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      "({Object = 0, String = 0} = {});": [
        {
          col: 2,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        },
        {
          col: 14,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      "Array = 1;": [
        {
          col: 0,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        }
      ],
      r#"
function foo() {
  let Boolean = false;
  Boolean = true;
}
Boolean = true;
      "#: [
        {
          col: 0,
          line: 6,
          message: NoGlobalAssignMessage::NotAllowed,
          hint: NoGlobalAssignHint::Remove,
        },
      ],
    };
  }
}
