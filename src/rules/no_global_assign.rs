// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::ProgramRef;
use crate::{globals::GLOBALS, swc_util::find_lhs_ids};
use deno_ast::swc::ast::Id;
use deno_ast::swc::{
  ast::*,
  visit::{noop_visit_type, Visit, VisitWith},
};
use deno_ast::SourceRange;
use deno_ast::SwcSourceRanged;
use derive_more::Display;
use std::sync::Arc;

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
  fn new() -> Arc<Self> {
    Arc::new(NoGlobalAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoGlobalAssignVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_global_assign.md")
  }
}

struct NoGlobalAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoGlobalAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check(&mut self, range: SourceRange, id: Id) {
    if id.1 != self.context.unresolved_ctxt() {
      return;
    }

    if self.context.scope().var(&id).is_some() {
      return;
    }

    // We only care about globals.
    let maybe_global = GLOBALS.iter().find(|(name, _)| name == &&*id.0);

    if let Some(global) = maybe_global {
      // If global can be overwritten then don't need to report anything
      if !global.1 {
        self.context.add_diagnostic_with_hint(
          range,
          CODE,
          NoGlobalAssignMessage::NotAllowed,
          NoGlobalAssignHint::Remove,
        );
      }
    }
  }
}

impl<'c, 'view> Visit for NoGlobalAssignVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, e: &AssignExpr) {
    let idents: Vec<Ident> = find_lhs_ids(&e.left);

    for ident in idents {
      self.check(ident.range(), ident.to_id());
    }
  }

  fn visit_update_expr(&mut self, e: &UpdateExpr) {
    if let Expr::Ident(i) = &*e.arg {
      self.check(e.range(), i.to_id());
    } else {
      e.visit_children_with(self);
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
