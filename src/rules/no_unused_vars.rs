// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::ast_node::AstNode;
use crate::scopes::LintContext;
use crate::scopes::LintTransform;

pub struct NoUnusedVars {
  context: Context,
}

impl NoUnusedVars {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl LintTransform for NoUnusedVars {
  fn enter(&self, context: &LintContext, node: AstNode) {
    let current_scope = context.get_current_scope();
    eprintln!("current csope in no unused {:#?}", current_scope);
  }
}
