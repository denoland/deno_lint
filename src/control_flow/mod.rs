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
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BlockKind {
  /// Function's body
  Function,
  Block,
  /// Switch case
  Case,
  If,
  /// Body of a loop
  Loop,
  Label(Id),
}

#[derive(Debug, Default, Clone)]
pub struct Metadata {
  pub unreachable: bool,
  done: Option<Done>,
}

impl Metadata {
  /// Returns true if a node prevents further execution.
  pub fn stops_execution(&self) -> bool {
    self.done.is_some()
  }
}

struct Analyzer<'a> {
  scope: Scope<'a>,
  info: HashMap<BytePos, Metadata>,
}

#[derive(Debug)]
struct Scope<'a> {
  _parent: Option<&'a Scope<'a>>,
  /// This field exists to handle code like
  ///
  /// `function foo() { return bar(); function bar() { return 1; } }`
  used_hoistable_ids: HashSet<Id>,

  _kind: BlockKind,

  /// Unconditionally ends with return, throw
  done: Option<Done>,

  may_throw: bool,

  ///
  /// - None: Not found
  /// - Some(None): Stopped at a break statement without label
  /// - Some(Somd(id)): Stopped at a break statement with label id
  found_break: Option<Option<Id>>,
  found_continue: bool,
}
#[derive(Debug, Copy, Clone)]
enum Done {
  /// Return, Throw, or infinite loop
  Forced,
  // Break or continue
  Break,
}

impl<'a> Scope<'a> {
  pub fn new(parent: Option<&'a Scope<'a>>, kind: BlockKind) -> Self {
    Self {
      _parent: parent,
      _kind: kind,
      used_hoistable_ids: Default::default(),
      done: None,
      may_throw: false,
      found_break: None,
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
    let prev_done = self.scope.done;
    let (info, done, hoist, found_break, found_continue, may_throw) = {
      let mut child = Analyzer {
        info: take(&mut self.info),
        scope: Scope::new(Some(&self.scope), kind.clone()),
      };
      match kind {
        BlockKind::Function => {}
        _ => {
          if let Some(Done::Forced) = prev_done {
            child.scope.done = Some(Done::Forced);
          }
        }
      }

      op(&mut child);

      (
        take(&mut child.info),
        child.scope.done,
        child.scope.used_hoistable_ids,
        child.scope.found_break,
        child.scope.found_continue,
        child.scope.may_throw,
      )
    };

    self.scope.used_hoistable_ids.extend(hoist);

    // Preserve information about visited ast nodes.
    self.scope.may_throw |= may_throw;
    if self.scope.found_break.is_none() {
      self.scope.found_break = found_break;
    }
    self.scope.found_continue |= found_continue;

    if let Some(done) = done {
      match kind {
        BlockKind::Function => {}
        BlockKind::Block => {
          if let Done::Forced = done {
            self.mark_as_done(lo, done);
          } else if self.scope.done.is_none() {
            self.scope.done = Some(Done::Break)
          }
        }
        BlockKind::Case => {
          if let Done::Forced = done {
            self.mark_as_done(lo, done);
          }
        }
        BlockKind::If => {}
        BlockKind::Loop => {}
        BlockKind::Label(label) => {
          if let Some(Some(id)) = &self.scope.found_break {
            if *id == label {
              // Eat break statemnt
              self.scope.found_break = None;
            }
          }
        }
      }
    }

    self.info = info;
  }

  fn is_forced_done(&self, lo: BytePos) -> bool {
    match self.get_done_reason(lo) {
      Some(Done::Forced) => true,
      _ => false,
    }
  }

  fn get_done_reason(&self, lo: BytePos) -> Option<Done> {
    self.info.get(&lo).map(|md| md.done).flatten()
  }

  /// Mark a statement as finisher - finishes execution - and expose it.
  fn mark_as_done(&mut self, lo: BytePos, done: Done) {
    if self.scope.done.is_none() {
      self.scope.done = Some(done);
    }
    self.info.entry(lo).or_default().done = Some(done);
  }

  /// Visits statement or block. This method handles break and continue.
  ///
  /// This cannot be done in visit_stmt of Visit because
  ///  this operation is very opinionated.
  fn visit_stmt_or_block(&mut self, s: &Stmt) {
    s.visit_with(&Invalid { span: DUMMY_SP }, self);

    // break, continue **may** make execution done
    match s {
      Stmt::Break(..) | Stmt::Continue(..) => {
        self.mark_as_done(s.span().lo, Done::Break)
      }
      _ => {}
    }
  }
}

macro_rules! mark_as_done {
  ($name:ident, $T:ty) => {
    fn $name(&mut self, s: &$T, _: &dyn Node) {
      s.visit_children_with(self);

      self.mark_as_done(s.span().lo, Done::Forced);
    }
  };
}

impl Visit for Analyzer<'_> {
  noop_visit_type!();

  mark_as_done!(visit_return_stmt, ReturnStmt);
  mark_as_done!(visit_throw_stmt, ThrowStmt);

  fn visit_break_stmt(&mut self, n: &BreakStmt, _: &dyn Node) {
    if let Some(label) = &n.label {
      let label = label.to_id();
      self.scope.found_break = Some(Some(label));
    } else {
      self.scope.found_break = Some(None);
    }
  }

  fn visit_continue_stmt(&mut self, _: &ContinueStmt, _: &dyn Node) {
    self.scope.found_continue = true;
  }

  fn visit_block_stmt(&mut self, s: &BlockStmt, _: &dyn Node) {
    s.visit_children_with(self);

    if let Some(done) = self.scope.done {
      self.mark_as_done(s.span.lo, done);
    }
  }

  fn visit_stmts(&mut self, stmts: &[Stmt], _: &dyn Node) {
    for stmt in stmts {
      self.visit_stmt_or_block(stmt);
    }
  }

  fn visit_expr(&mut self, n: &Expr, _: &dyn Node) {
    n.visit_children_with(self);

    if self.scope.done.is_none() {
      match n {
        Expr::Ident(i) => {
          self.scope.used_hoistable_ids.insert(i.to_id());
        }
        Expr::This(..) => {}
        _ => {
          self.scope.may_throw = true;
        }
      }
    }
  }

  fn visit_member_expr(&mut self, n: &MemberExpr, _: &dyn Node) {
    n.obj.visit_with(n, self);
    if n.computed {
      n.prop.visit_with(n, self);
    }
  }

  fn visit_arrow_expr(&mut self, n: &ArrowExpr, _: &dyn Node) {
    self.with_child_scope(BlockKind::Function, n.span().lo, |a| {
      n.visit_children_with(a);
    })
  }

  fn visit_function(&mut self, n: &Function, _: &dyn Node) {
    self.with_child_scope(BlockKind::Function, n.span().lo, |a| {
      n.visit_children_with(a);
    })
  }

  fn visit_catch_clause(&mut self, n: &CatchClause, _: &dyn Node) {
    self.with_child_scope(BlockKind::Block, n.span().lo, |a| {
      n.visit_children_with(a);
    });
  }

  fn visit_constructor(&mut self, n: &Constructor, _: &dyn Node) {
    self.with_child_scope(BlockKind::Function, n.span.lo, |a| {
      n.visit_children_with(a);
    });
  }

  fn visit_getter_prop(&mut self, n: &GetterProp, _: &dyn Node) {
    self.with_child_scope(BlockKind::Function, n.span.lo, |a| {
      n.visit_children_with(a);
    })
  }

  fn visit_setter_prop(&mut self, n: &SetterProp, _: &dyn Node) {
    self.with_child_scope(BlockKind::Function, n.span.lo, |a| {
      n.visit_children_with(a);
    })
  }

  fn visit_switch_stmt(&mut self, n: &SwitchStmt, _: &dyn Node) {
    let prev_done = self.scope.done;
    n.visit_children_with(self);

    let has_default = n.cases.iter().any(|case| case.test.is_none());

    // SwitchStmt finishes execution if all cases finishes execution
    let is_done = has_default
      && n
        .cases
        .iter()
        .map(|case| case.span.lo)
        .all(|lo| self.is_forced_done(lo));

    // A switch statement is finisher or not.
    if is_done {
      self.mark_as_done(n.span.lo, Done::Forced);
    } else {
      self.scope.done = prev_done;
    }
  }

  fn visit_switch_case(&mut self, n: &SwitchCase, _: &dyn Node) {
    let prev_done = self.scope.done;
    let mut case_done = None;

    self.with_child_scope(BlockKind::Case, n.span.lo, |a| {
      n.cons.visit_with(n, a);

      if a.scope.found_break.is_some() {
        case_done = Some(Done::Break);
      } else if let Some(Done::Forced) = a.scope.done {
        case_done = Some(Done::Forced);
      }
    });

    if let Some(Done::Forced) = case_done {
      self.mark_as_done(n.span.lo, Done::Forced);
    }
    self.scope.done = prev_done;
  }

  fn visit_if_stmt(&mut self, n: &IfStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    let prev_done = self.scope.done;

    self.with_child_scope(BlockKind::If, n.cons.span().lo, |a| {
      a.visit_stmt_or_block(&n.cons);
    });

    let cons_reason = self.get_done_reason(n.cons.span().lo);

    match &n.alt {
      Some(alt) => {
        self.with_child_scope(BlockKind::If, alt.span().lo, |a| {
          //
          a.visit_stmt_or_block(&alt);
        });
        let alt_reason = self.get_done_reason(alt.span().lo);

        match (cons_reason, alt_reason) {
          (Some(Done::Forced), Some(Done::Forced)) => {
            self.mark_as_done(n.span.lo, Done::Forced);
          }
          (Some(Done::Break), another) | (another, Some(Done::Break)) => {
            if another.is_some() {
              self.mark_as_done(n.span.lo, Done::Break);
            } else {
              self.scope.done = None
            }
          }
          // TODO: Check for continue
          _ => {}
        }
      }
      None => {
        self.scope.done = prev_done;
      }
    }
  }

  fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
    let unreachable = if self.scope.done.is_some() {
      // Although execution is done, we should handle hoisting.
      match n {
        Stmt::Empty(..) => false,
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

    let mut stmt_done = None;

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      n.body.visit_with(n, a);

      if a.scope.found_break.is_none() {
        if n.test.is_none() {
          // Infinite loop
          a.mark_as_done(n.span.lo, Done::Forced);
          stmt_done = Some(Done::Forced);
        } else if let (_, Value::Known(true)) =
          n.test.as_ref().unwrap().as_bool()
        {
          // Infinite loop
          a.mark_as_done(n.span.lo, Done::Forced);
          stmt_done = Some(Done::Forced);
        }
      }
    });

    if let Some(done) = stmt_done {
      self.scope.done = Some(done)
    }
  }

  fn visit_for_of_stmt(&mut self, n: &ForOfStmt, _: &dyn Node) {
    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
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

    let mut stmt_done = None;

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      n.body.visit_with(n, a);
      if let (_, Value::Known(true)) = n.test.as_bool() {
        if let Some(Done::Forced) = a.get_done_reason(n.body.span().lo) {
          a.mark_as_done(n.span.lo, Done::Forced);
          stmt_done = Some(Done::Forced);
        }

        if a.scope.found_break.is_none() {
          // Infinite loop
          a.mark_as_done(n.span.lo, Done::Forced);
          stmt_done = Some(Done::Forced);
        }
      }
    });

    if let Some(done) = stmt_done {
      self.scope.done = Some(done)
    }
  }

  fn visit_do_while_stmt(&mut self, n: &DoWhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    let mut stmt_done = None;
    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      a.visit_stmt_or_block(&n.body);

      if let Some(done) = a.scope.done {
        stmt_done = Some(done);
        a.mark_as_done(n.span.lo, done);
      }
    });
    if let Some(done) = stmt_done {
      self.scope.done = Some(done);
    }
  }

  fn visit_try_stmt(&mut self, n: &TryStmt, _: &dyn Node) {
    n.finalizer.visit_with(n, self);
    let old_throw = self.scope.may_throw;

    let prev_done = self.scope.done;

    self.scope.may_throw = false;
    n.block.visit_with(n, self);

    let mut block_done = None;

    if self.scope.may_throw {
      if let Some(done) = self.scope.done {
        block_done = Some(done);
        self.scope.done = prev_done;
      }
    } else if let Some(done) = self.scope.done {
      block_done = Some(done);
      self.mark_as_done(n.span.lo, done);
    }

    n.handler.visit_with(n, self);
    match (block_done, self.scope.done) {
      (Some(Done::Forced), Some(Done::Forced)) => {
        self.mark_as_done(n.span.lo, Done::Forced);
      }
      (Some(_try_done), Some(_catch_done)) => {
        self.mark_as_done(n.span.lo, Done::Break);
      }
      _ => {
        self.scope.done = prev_done;
      }
    }

    self.scope.may_throw = old_throw;
  }

  fn visit_labeled_stmt(&mut self, n: &LabeledStmt, _: &dyn Node) {
    self.with_child_scope(BlockKind::Label(n.label.to_id()), n.span.lo, |a| {
      a.visit_stmt_or_block(&n.body);
    });
  }
}
