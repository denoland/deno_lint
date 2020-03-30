// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::ast_node::AstNode;
use crate::scopes::LintContext;
use crate::scopes::LintTransform;
use crate::traverse::AstTraverser;
use swc_ecma_ast::DebuggerStmt;

pub struct NoDebugger {
  context: Context,
}

impl NoDebugger {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoDebugger {
  fn walk_debugger_stmt(&self, debugger_stmt: DebuggerStmt) {
    self.context.add_diagnostic(
      &debugger_stmt.span,
      "noDebugger",
      "`debugger` statement is not allowed",
    );
  }
}

impl LintTransform for NoDebugger {
  fn enter(&self, _context: &LintContext, node: AstNode) {
    if let AstNode::DebuggerStmt(debugger_stmt) = node {
      self.context.add_diagnostic(
        &debugger_stmt.span,
        "noDebugger",
        "`debugger` statement is not allowed",
      );
    }
  }
}
