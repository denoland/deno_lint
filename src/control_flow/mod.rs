use std::{collections::HashMap, mem::take};
use swc_common::{BytePos, Spanned, DUMMY_SP};
use swc_ecmascript::ast::*;
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

mod util;

pub struct ControlFlow {
  meta: HashMap<BytePos, Metadata>,
}

impl ControlFlow {
  pub fn analyze(m: &Module) -> Self {
    let mut v = Analyzer {
      scope: Scope {
        _parent: None,
        path: vec![],
        finished: false,
      },
      info: Default::default(),
    };
    m.visit_with(&Invalid { span: DUMMY_SP }, &mut v);
    ControlFlow { meta: v.info }
  }

  /// lo can be extracted from span of
  ///
  /// - All statements (including stmt.span())
  /// - [SwitchCase]
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
  /// Body of a loop
  Loop,
}

#[derive(Debug, Default, Clone)]
pub struct Metadata {
  pub unreachable: bool,
  pub finished: bool,
  // path: Vec<BlockKind>,
}

impl Metadata {
  // pub fn path(&self) -> &[BlockKind] {
  //   &self.path
  // }
}

struct Analyzer<'a> {
  scope: Scope<'a>,
  info: HashMap<BytePos, Metadata>,
}

struct Scope<'a> {
  _parent: Option<&'a Scope<'a>>,
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

  fn is_finished(&self, lo: BytePos) -> bool {
    self.info.get(&lo).map(|md| md.finished).unwrap_or(false)
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

  fn visit_switch_stmt(&mut self, n: &SwitchStmt, _: &dyn Node) {
    n.visit_children_with(self);

    // SwitchStmt finishes execution if all cases finishes execution
    let is_finished = n
      .cases
      .iter()
      .map(|case| case.span.lo)
      .all(|lo| self.is_finished(lo));

    if is_finished {
      self.info.entry(n.span.lo).or_default().finished = true;
    }
  }

  fn visit_switch_case(&mut self, n: &SwitchCase, _: &dyn Node) {
    self.with_child_scope(BlockKind::Case, |a| n.cons.visit_with(n, a));
  }

  fn visit_if_stmt(&mut self, n: &IfStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::If, |a| {
      n.cons.visit_with(n, a);
    });

    let is_cons_finished = self.is_finished(n.cons.span().lo);

    match &n.alt {
      Some(alt) => {
        self.with_child_scope(BlockKind::If, |a| {
          //
          alt.visit_with(n, a);
        });
        let is_alt_finished = self.is_finished(alt.span().lo);

        if is_cons_finished && is_alt_finished {
          self.info.entry(n.span.lo).or_default().finished = true;
        }
      }
      None => {}
    }
  }

  fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
    if self.scope.finished {
      // It's unreachable
      self.info.entry(n.span().lo).or_default().unreachable = true;
    }

    n.visit_children_with(self);
  }

  // loops

  fn visit_for_stmt(&mut self, n: &ForStmt, _: &dyn Node) {
    n.init.visit_with(n, self);
    n.update.visit_with(n, self);
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, |a| {
      n.body.visit_with(n, a);
    });
  }

  fn visit_for_of_stmt(&mut self, n: &ForOfStmt, _: &dyn Node) {
    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, |a| {
      n.body.visit_with(n, a);
    });
  }

  fn visit_for_in_stmt(&mut self, n: &ForInStmt, _: &dyn Node) {
    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, |a| {
      n.body.visit_with(n, a);
    });
  }

  fn visit_while_stmt(&mut self, n: &WhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, |a| {
      n.body.visit_with(n, a);
    });
  }

  fn visit_do_while_stmt(&mut self, n: &DoWhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, |a| {
      n.body.visit_with(n, a);
    });
  }
}
