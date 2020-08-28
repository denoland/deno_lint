use std::collections::{HashMap, HashSet};
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
      scope: Scope {
        _parent: None,
        path: vec![],
        finished: false,
        continue_pos: Default::default(),
        found_break: false,
        used_hoistable_ids: Default::default(),
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
  /// This field exists to handle code like
  ///
  /// `function foo() { return bar(); function bar() { return 1; } }`
  used_hoistable_ids: HashSet<Id>,

  path: Vec<BlockKind>,
  /// Unconditionally ends with return, throw, brak or continue
  finished: bool,
  // What should happen when loop ends with a continue
  continue_pos: Option<BytePos>,

  found_break: bool,
}

impl Analyzer<'_> {
  /// `lo` is used when child operation is `finished`
  fn with_child_scope(
    &mut self,
    kind: BlockKind,
    lo: BytePos,
    op: impl Fn(&mut Analyzer),
  ) {
    let found_break = self.scope.found_break;
    self.scope.path.push(kind);

    self.scope.found_break = false;
    self.scope.finished = false;
    op(self);
    if self.scope.finished {
      self.info.entry(lo).or_default().finished = true;
    }
    self.scope.found_break |= found_break;
    self.scope.continue_pos = None;

    if kind == BlockKind::Function {
      self.scope.finished = false;
      self.scope.found_break = false;
    }

    self.scope.path.pop();
  }

  fn is_finished(&self, lo: BytePos) -> bool {
    self.info.get(&lo).map(|md| md.finished).unwrap_or(false)
  }

  fn is_conditional(&self) -> bool {
    for kind in self.scope.path.iter().rev() {
      match kind {
        BlockKind::Function => return false,
        BlockKind::Case => return false,
        BlockKind::If => return true,
        BlockKind::Loop => {}
      }
    }

    false
  }
}

macro_rules! mark_as_finished {
  ($name:ident, $T:ty) => {
    fn $name(&mut self, _: &$T, _: &dyn Node) {
      if !self.is_conditional() {
        self.scope.finished = true;
      }
    }
  };
}

impl Visit for Analyzer<'_> {
  noop_visit_type!();

  mark_as_finished!(visit_throw_stmt, ThrowStmt);
  mark_as_finished!(visit_continue_stmt, ContinueStmt);

  fn visit_return_stmt(&mut self, r: &ReturnStmt, _: &dyn Node) {
    r.visit_children_with(self);

    if !self.is_conditional() {
      self.scope.finished = true;
    }
  }

  fn visit_expr(&mut self, n: &Expr, _: &dyn Node) {
    n.visit_children_with(self);

    if !self.scope.finished {
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

  fn visit_break_stmt(&mut self, _: &BreakStmt, _: &dyn Node) {
    self.scope.found_break = true;

    if !self.is_conditional() {
      self.scope.finished = true;
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
    let is_finished = n
      .cases
      .iter()
      .map(|case| case.span.lo)
      .all(|lo| self.is_finished(lo));

    self.scope.finished |= is_finished;
    if is_finished {
      self.info.entry(n.span.lo).or_default().finished = true;
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
      n.cons.visit_with(n, a);
    });

    let is_cons_finished = self.is_finished(n.cons.span().lo);

    match &n.alt {
      Some(alt) => {
        self.with_child_scope(BlockKind::If, alt.span().lo, |a| {
          //
          alt.visit_with(n, a);
        });
        let is_alt_finished = self.is_finished(alt.span().lo);

        if is_cons_finished && is_alt_finished {
          self.scope.finished = true;
          self.info.entry(n.span.lo).or_default().finished = true;
        }
      }
      None => {}
    }
  }

  fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
    let unreachable = if self.scope.finished {
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
    });

    if !self.scope.found_break {
      if n.test.is_none() {
        self.scope.finished = true;
      } else if let (_, Value::Known(true)) = n.test.as_ref().unwrap().as_bool()
      {
        self.scope.finished = true;
      }
    }
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

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      a.scope.continue_pos = Some(n.span.lo);

      n.body.visit_with(n, a);

      dbg!(a.scope.found_break);
      if !a.scope.found_break {
        if let (_, Value::Known(true)) = n.test.as_bool() {
          dbg!();
          a.scope.finished = true;
          a.info.entry(n.span.lo).or_default().finished = true;
        }
      }
    });
  }

  fn visit_do_while_stmt(&mut self, n: &DoWhileStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      a.scope.continue_pos = Some(n.span.lo);

      n.body.visit_with(n, a);
    });
  }
}
