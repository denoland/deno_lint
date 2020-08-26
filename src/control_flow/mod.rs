use std::{
  collections::{hash_map::Entry, HashMap},
  mem::take,
};
use swc_common::BytePos;
use swc_ecmascript::ast::*;
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

mod util;

pub struct ControlFlow {
  meta: HashMap<BytePos, Metadata>,
}

impl ControlFlow {
  pub fn meta(&self, lo: BytePos) -> Option<&Metadata> {
    self.meta.get(&lo)
  }
}

/// Kind of a basic block.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {
  /// Function's body
  Function,
  Case,
  If,
}

#[derive(Debug, Clone)]
pub struct Metadata {
  unreachable: bool,
  path: Vec<BlockKind>,
}

impl Metadata {
  pub fn path(&self) -> &[BlockKind] {
    &self.path
  }
}

struct Analyzer<'a> {
  scope: Scope<'a>,
  info: HashMap<BytePos, Metadata>,
}

struct Scope<'a> {
  parent: Option<&'a Scope<'a>>,
  path: Vec<BlockKind>,
  /// Unconditionally ends with return, throw, brak or continue
  finished: bool,
}

impl Analyzer<'_> {
  fn with_child_scope(&mut self, kind: BlockKind, op: impl Fn(&mut Analyzer)) {
    self.scope.path.push(kind);
    let info = take(&mut self.info);

    op(self);

    self.info.extend(info);
    self.scope.path.pop();
  }
}

macro_rules! mark_as_finished {
  ($name:ident, $T:ty) => {
    fn $name(&mut self, _: &$T, _: &dyn Node) {
      self.scope.finished = true;
    }
  };
}

impl Visit for Analyzer<'_> {
  noop_visit_type!();

  mark_as_finished!(visit_return_stmt, ReturnStmt);
  mark_as_finished!(visit_throw_stmt, ThrowStmt);
  mark_as_finished!(visit_break_stmt, BreakStmt);
  mark_as_finished!(visit_continue_stmt, ContinueStmt);

  fn visit_fn_decl(&mut self, n: &FnDecl, _: &dyn Node) {
    self.with_child_scope(BlockKind::Function, |a| n.function.visit_with(n, a))
  }

  fn visit_switch_case(&mut self, n: &SwitchCase, _: &dyn Node) {
    self.with_child_scope(BlockKind::Case, |a| n.cons.visit_with(n, a));
  }
}
