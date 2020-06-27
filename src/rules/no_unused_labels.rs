// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::BreakStmt;
use swc_ecma_ast::ContinueStmt;
use swc_ecma_ast::Ident;
use swc_ecma_ast::LabeledStmt;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoUnusedLabels;

impl LintRule for NoUnusedLabels {
  fn new() -> Box<Self> {
    Box::new(NoUnusedLabels)
  }

  fn code(&self) -> &'static str {
    "no-unused-labels"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoUnusedLabelsVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

struct LabelScope {
  used: bool,
  name: String,
}

struct NoUnusedLabelsVisitor {
  context: Context,
  label_scopes: Vec<LabelScope>,
}

impl NoUnusedLabelsVisitor {
  pub fn new(context: Context) -> Self {
    Self {
      context,
      label_scopes: vec![],
    }
  }

  fn maybe_check_label(&mut self, maybe_label: Option<&Ident>) {
    if let Some(label) = maybe_label {
      let label_name = label.sym.to_string();

      for label_scope in self.label_scopes.iter_mut().rev() {
        if label_scope.name == label_name {
          label_scope.used = true;
          break;
        }
      }
    }
  }
}

impl Visit for NoUnusedLabelsVisitor {
  fn visit_labeled_stmt(
    &mut self,
    labeled_stmt: &LabeledStmt,
    parent: &dyn Node,
  ) {
    let name = labeled_stmt.label.sym.to_string();
    let label_scope = LabelScope {
      name: name.clone(),
      used: false,
    };
    self.label_scopes.push(label_scope);
    swc_ecma_visit::visit_labeled_stmt(self, labeled_stmt, parent);
    let scope = self.label_scopes.pop().expect("self.label_scopes is empty");
    if !scope.used {
      self.context.add_diagnostic(
        labeled_stmt.span,
        "no-unused-labels",
        &format!("\"{}\" label is never used", name),
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
  use crate::test_util::*;

  #[test]
  fn no_unused_label_ok() {
    assert_lint_ok::<NoUnusedLabels>(
      "LABEL: for (let i = 0; i < 5; i++) { a(); break LABEL; }",
    );
    assert_lint_ok::<NoUnusedLabels>("LABEL: for (let i = 0; i < 5; i++) { a(); if (i < 3) { continue LABEL; } b(); if (i > 3) { break LABEL; } }");
    assert_lint_ok::<NoUnusedLabels>("LABEL: { a(); b(); break LABEL; c(); }");
    assert_lint_ok::<NoUnusedLabels>("A: { B: break B; C: for (var i = 0; i < 10; ++i) { foo(); if (a) break A; if (c) continue C; bar(); } }");
    assert_lint_ok::<NoUnusedLabels>("LABEL: while(true) { break LABEL; }");
    assert_lint_ok::<NoUnusedLabels>("LABEL: break LABEL;");
  }

  #[test]
  fn no_unused_label_err() {
    assert_lint_err::<NoUnusedLabels>("LABEL: var a = 0;", 0);
    assert_lint_err::<NoUnusedLabels>("LABEL: if (something) { a(); }", 0);
    assert_lint_err::<NoUnusedLabels>(
      "LABEL: for (let i = 0; i < 5; i++) { a(); b(); }",
      0,
    );
    assert_lint_err::<NoUnusedLabels>(
      "A: for (var i = 0; i < 10; ++i) { B: break A; }",
      34,
    );
  }
}
