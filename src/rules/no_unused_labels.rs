// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use swc_ecmascript::ast::BreakStmt;
use swc_ecmascript::ast::ContinueStmt;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::ast::LabeledStmt;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoUnusedLabels;

const CODE: &str = "no-unused-labels";

#[derive(Display)]
enum NoUnusedLabelsMessage {
  #[display(fmt = "`{}` label is never used", _0)]
  Unused(String),
}

impl LintRule for NoUnusedLabels {
  fn new() -> Box<Self> {
    Box::new(NoUnusedLabels)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: ProgramRef<'_>) {
    let mut visitor = NoUnusedLabelsVisitor::new(context);
    match program {
      ProgramRef::Module(ref m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(ref s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }
}

struct LabelScope {
  used: bool,
  name: String,
}

struct NoUnusedLabelsVisitor<'c> {
  context: &'c mut Context,
  label_scopes: Vec<LabelScope>,
}

impl<'c> NoUnusedLabelsVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self {
      context,
      label_scopes: vec![],
    }
  }

  fn maybe_check_label(&mut self, maybe_label: Option<&Ident>) {
    if let Some(label) = maybe_label {
      let label_name = label.sym.as_ref();

      for label_scope in self.label_scopes.iter_mut().rev() {
        if label_scope.name == label_name {
          label_scope.used = true;
          break;
        }
      }
    }
  }
}

impl<'c> Visit for NoUnusedLabelsVisitor<'c> {
  noop_visit_type!();

  fn visit_labeled_stmt(
    &mut self,
    labeled_stmt: &LabeledStmt,
    parent: &dyn Node,
  ) {
    let name = labeled_stmt.label.sym.as_ref();
    let label_scope = LabelScope {
      name: name.to_owned(),
      used: false,
    };
    self.label_scopes.push(label_scope);
    swc_ecmascript::visit::visit_labeled_stmt(self, labeled_stmt, parent);
    let scope = self.label_scopes.pop().expect("self.label_scopes is empty");
    if !scope.used {
      self.context.add_diagnostic(
        labeled_stmt.span,
        CODE,
        NoUnusedLabelsMessage::Unused(name.to_string()),
      );
    }
  }

  fn visit_continue_stmt(
    &mut self,
    continue_stmt: &ContinueStmt,
    _parent: &dyn Node,
  ) {
    self.maybe_check_label(continue_stmt.label.as_ref());
  }

  fn visit_break_stmt(&mut self, break_stmt: &BreakStmt, _parent: &dyn Node) {
    self.maybe_check_label(break_stmt.label.as_ref());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unused_label_valid() {
    assert_lint_ok! {
      NoUnusedLabels,
      "LABEL: for (let i = 0; i < 5; i++) { a(); break LABEL; }",
      "LABEL: for (let i = 0; i < 5; i++) { a(); if (i < 3) { continue LABEL; } b(); if (i > 3) { break LABEL; } }",
      "LABEL: { a(); b(); break LABEL; c(); }",
      "A: { B: break B; C: for (var i = 0; i < 10; ++i) { foo(); if (a) break A; if (c) continue C; bar(); } }",
      "LABEL: while(true) { break LABEL; }",
      "LABEL: break LABEL;",
    };
  }

  #[test]
  fn no_unused_label_invalid() {
    assert_lint_err! {
      NoUnusedLabels,
      "LABEL: var a = 0;": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "LABEL"),
        }
      ],
      "LABEL: if (something) { a(); }": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "LABEL"),
        }
      ],
      "LABEL: for (let i = 0; i < 5; i++) { a(); b(); }": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "LABEL"),
        }
      ],
      "A: for (var i = 0; i < 10; ++i) { B: break A; }": [
        {
          col: 34,
          message: variant!(NoUnusedLabelsMessage, Unused, "B"),
        }
      ],
      "A: { let A = 0; console.log(A); }": [
        {
          col: 0,
          message: variant!(NoUnusedLabelsMessage, Unused, "A"),
        }
      ],
    };
  }
}
