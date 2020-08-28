use std::{
  collections::{HashMap, HashSet},
  mem::take,
};
use swc_common::{BytePos, Spanned, DUMMY_SP};
use swc_ecmascript::ast::*;
use swc_ecmascript::{
  utils::{ident::IdentLike, ExprExt, Id, Value},
  visit::{noop_visit_type, Node, Visit, VisitWith},
};

mod util;

pub struct ControlFlow {
  meta: HashMap<BytePos, Metadata>,
}

impl ControlFlow {
  pub fn analyze(m: &Module) -> Self {
    let mut v = Analyzer {
      scope: Scope::new(None, BlockKind::Function),
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
  pub done: bool,
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
  parent: Option<&'a Scope<'a>>,
  /// This field exists to handle code like
  ///
  /// `function foo() { return bar(); function bar() { return 1; } }`
  used_hoistable_ids: HashSet<Id>,

  kind: BlockKind,
  /// Unconditionally ends with return, throw
  done: bool,
  // What should happen when loop ends with a continue
  continue_pos: Option<BytePos>,

  found_break: bool,
  found_continue: bool,
}

impl<'a> Scope<'a> {
  pub fn new(parent: Option<&'a Scope<'a>>, kind: BlockKind) -> Self {
    Self {
      parent,
      kind,
      continue_pos: Default::default(),
      used_hoistable_ids: Default::default(),
      done: false,
      found_break: false,
      found_continue: false,
    }
  }
}

impl Analyzer<'_> {
  /// `lo` is marked as done if child scope is unconditionally finished
  pub(super) fn with_child_scope<F>(
    &mut self,
    kind: BlockKind,
    lo: BytePos,
    op: F,
  ) where
    F: for<'any> FnOnce(&mut Analyzer<'any>),
  {
    let (info, done, hoist) = {
      dbg!(self.scope.parent.is_some());
      dbg!(self.scope.kind);
      let mut child = Analyzer {
        info: take(&mut self.info),
        scope: Scope::new(Some(&self.scope), kind),
      };

      op(&mut child);

      (
        take(&mut child.info),
        child.scope.done,
        child.scope.used_hoistable_ids,
      )
    };

    self.scope.used_hoistable_ids.extend(hoist);

    if done {
      match kind {
        BlockKind::Case | BlockKind::Loop => {
          self.mark_as_done(lo);
        }
        _ => {}
      }
    }

    self.info = info;
  }

  fn is_done(&self, lo: BytePos) -> bool {
    self.info.get(&lo).map(|md| md.done).unwrap_or(false)
  }

  fn mark_as_done(&mut self, lo: BytePos) {
    self.scope.done = true;
    self.info.entry(lo).or_default().done = true;
  }

  /// Visits statement or block. This method handles break and continue
  fn visit_stmt_or_block(&mut self, s: &Stmt) {
    s.visit_with(&Invalid { span: DUMMY_SP }, self);

    // break, continue **may** make execution done
    match s {
      Stmt::Break(..) | Stmt::Continue(..) => self.mark_as_done(s.span().lo),
      _ => {}
    }
  }
}

macro_rules! mark_as_done {
  ($name:ident, $T:ty) => {
    fn $name(&mut self, s: &$T, _: &dyn Node) {
      s.visit_children_with(self);

      self.mark_as_done(s.span().lo);
    }
  };
}

impl Visit for Analyzer<'_> {
  noop_visit_type!();

  mark_as_done!(visit_return_stmt, ReturnStmt);
  mark_as_done!(visit_throw_stmt, ThrowStmt);

  fn visit_break_stmt(&mut self, _: &BreakStmt, _: &dyn Node) {
    self.scope.found_break = true;
  }

  fn visit_continue_stmt(&mut self, _: &ContinueStmt, _: &dyn Node) {
    self.scope.found_continue = true;
  }

  fn visit_block_stmt(&mut self, s: &BlockStmt, _: &dyn Node) {
    s.visit_children_with(self);

    if self.scope.done {
      self.mark_as_done(s.span.lo);
    }
  }

  fn visit_stmts(&mut self, stmts: &[Stmt], _: &dyn Node) {
    for stmt in stmts {
      self.visit_stmt_or_block(stmt);
    }
  }

  fn visit_expr(&mut self, n: &Expr, _: &dyn Node) {
    n.visit_children_with(self);

    if !self.scope.done {
      match n {
        Expr::Ident(i) => {
          self.scope.used_hoistable_ids.insert(i.to_id());
        }
        _ => {}
      }
    }
  }

  fn visit_member_expr(&mut self, n: &MemberExpr, _: &dyn Node) {
    n.obj.visit_with(n, self);
    if n.computed {
      n.prop.visit_with(n, self);
    }
  }

  fn visit_fn_decl(&mut self, n: &FnDecl, _: &dyn Node) {
    self.with_child_scope(BlockKind::Function, n.span().lo, |a| {
      n.function.visit_with(n, a)
    })
  }

  fn visit_switch_stmt(&mut self, n: &SwitchStmt, _: &dyn Node) {
    n.visit_children_with(self);

    // SwitchStmt finishes execution if all cases finishes execution
    let is_done = n
      .cases
      .iter()
      .map(|case| case.span.lo)
      .all(|lo| self.is_done(lo));

    if is_done {
      self.mark_as_done(n.span.lo);
    }
  }

  fn visit_switch_case(&mut self, n: &SwitchCase, _: &dyn Node) {
    self.with_child_scope(BlockKind::Case, n.span.lo, |a| {
      n.cons.visit_with(n, a)
    });
  }

  fn visit_if_stmt(&mut self, n: &IfStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::If, n.cons.span().lo, |a| {
      a.visit_stmt_or_block(&n.cons);
    });

    let is_cons_finished = self.is_done(n.cons.span().lo);

    match &n.alt {
      Some(alt) => {
        self.with_child_scope(BlockKind::If, alt.span().lo, |a| {
          //
          a.visit_stmt_or_block(&alt);
        });
        let is_alt_finished = self.is_done(alt.span().lo);

        if is_cons_finished && is_alt_finished {
          self.scope.done = true;
          self.info.entry(n.span.lo).or_default().done = true;
        }
      }
      None => {}
    }
  }

  fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
    let unreachable = if self.scope.done {
      // Although execution is done, we should handle hoisting.
      match n {
        Stmt::Decl(Decl::Fn(FnDecl { ident, .. }))
          if self.scope.used_hoistable_ids.contains(&ident.to_id()) =>
        {
          false
        }
        Stmt::Decl(Decl::Var(VarDecl {
          kind: VarDeclKind::Var,
          decls,
          ..
        }))
          if decls.iter().all(|decl| decl.init.is_none()) =>
        {
          false
        }
        // It's unreachable
        _ => true,
      }
    } else {
      false
    };

    if unreachable {
      self.info.entry(n.span().lo).or_default().unreachable = true;
    }

    n.visit_children_with(self);
  }

  // loops

  fn visit_for_stmt(&mut self, n: &ForStmt, _: &dyn Node) {
    n.init.visit_with(n, self);
    n.update.visit_with(n, self);
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      a.scope.continue_pos = Some(n.span.lo);

      n.body.visit_with(n, a);

      if !a.scope.found_break {
        if n.test.is_none() {
          a.scope.done = true;
        } else if let (_, Value::Known(true)) =
          n.test.as_ref().unwrap().as_bool()
        {
          a.scope.done = true;
        }
      }
    });
  }

  fn visit_for_of_stmt(&mut self, n: &ForOfStmt, _: &dyn Node) {
    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      a.scope.continue_pos = Some(n.span.lo);

      n.body.visit_with(n, a);
    });
  }

  fn visit_for_in_stmt(&mut self, n: &ForInStmt, _: &dyn Node) {
    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      n.body.visit_with(n, a);
    });
  }

  fn visit_while_stmt(&mut self, n: &WhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    let done = self.scope.done;

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      a.scope.continue_pos = Some(n.span.lo);

      n.body.visit_with(n, a);

      dbg!(a.scope.found_break);
      if !a.scope.found_break {
        if let (_, Value::Known(true)) = n.test.as_bool() {
          a.scope.done = true;
          a.info.entry(n.span.lo).or_default().done = true;
        }
      }
    });

    self.scope.done |= done;
  }

  fn visit_do_while_stmt(&mut self, n: &DoWhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      a.scope.continue_pos = Some(n.span.lo);

      n.body.visit_with(n, a);
    });
  }
}
