// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

#[cfg(test)]
mod analyze_test;

use ast_view::ProgramRef;
use std::{
  collections::{BTreeMap, HashSet},
  mem::take,
};
use swc_common::{BytePos, Spanned, DUMMY_SP};
use swc_ecmascript::ast::*;
use swc_ecmascript::{
  utils::{ident::IdentLike, ExprExt, Id, Value},
  visit::{noop_visit_type, Node, Visit, VisitWith},
};

#[derive(Debug, Clone)]
pub struct ControlFlow {
  meta: BTreeMap<BytePos, Metadata>,
}

impl ControlFlow {
  pub fn analyze(program: ProgramRef) -> Self {
    let mut v = Analyzer {
      scope: Scope::new(None, BlockKind::Program),
      info: Default::default(),
    };
    match program {
      ProgramRef::Module(module) => {
        module.visit_with(&Invalid { span: DUMMY_SP }, &mut v)
      }
      ProgramRef::Script(script) => {
        script.visit_with(&Invalid { span: DUMMY_SP }, &mut v)
      }
    }
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
  /// Program (module or script)
  Program,
  /// Function's body
  Function,
  /// Switch case
  Case,
  /// If's body
  If,
  /// Body of a loop
  Loop,
  Label(Id),
  /// Catch clause's body
  Catch,
  /// Finally's body
  Finally,
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct Metadata {
  pub unreachable: bool,
  end: Option<End>,
}

impl Metadata {
  /// Returns true if a node prevents further execution.
  pub fn stops_execution(&self) -> bool {
    self
      .end
      .map_or(false, |d| matches!(d, End::Forced { .. } | End::Break))
  }

  /// Returns true if a node doesn't prevent further execution.
  pub fn continues_execution(&self) -> bool {
    self.end.map_or(true, |d| d == End::Continue)
  }
}

#[derive(Debug)]
struct Analyzer<'a> {
  scope: Scope<'a>,
  info: BTreeMap<BytePos, Metadata>,
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
  end: Option<End>,

  may_throw: bool,

  ///
  /// - None: Not found
  /// - Some(None): Stopped at a break statement without label
  /// - Some(Somd(id)): Stopped at a break statement with label id
  found_break: Option<Option<Id>>,
  found_continue: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum End {
  /// Contains something that stops execution at that point.
  /// This is represented as product of three elements (ret, throw, infinite_loop)
  /// because sometimes these conditions are satisfied _simultaneously_.
  /// See the example below:
  ///
  /// ```typescript
  /// switch (foo) {
  ///   case 1:
  ///     return 1;
  ///   case 2:
  ///     throw 2;
  ///   default:
  ///     return 0;
  /// }
  /// ```
  ///
  /// In this case, the control flow can enter any one branch, which can be interpreted as
  /// `End::Forced { ret: true, throw: true, infinite_loop: false }`.
  Forced {
    /// Unconditionally return
    ret: bool,
    /// Unconditionally throw
    throw: bool,
    /// Unconditionally entering infinite loop
    infinite_loop: bool,
  },

  /// Break or continue
  Break,

  /// Pass through a block, like a function's block statement which ends without returning a value
  /// or throwing an exception. Note that a node marked as `End::Continue` won't prevent further execution, which is
  /// different from `End::Forced` or `End::Break`.
  Continue,
}

impl End {
  fn forced_return() -> Self {
    End::Forced {
      ret: true,
      throw: false,
      infinite_loop: false,
    }
  }

  fn forced_throw() -> Self {
    End::Forced {
      ret: false,
      throw: true,
      infinite_loop: false,
    }
  }

  fn forced_infinite_loop() -> Self {
    End::Forced {
      ret: false,
      throw: false,
      infinite_loop: true,
    }
  }

  fn merge_forced(self, other: Self) -> Option<Self> {
    match (self, other) {
      (
        End::Forced {
          ret: r1,
          throw: t1,
          infinite_loop: i1,
        },
        End::Forced {
          ret: r2,
          throw: t2,
          infinite_loop: i2,
        },
      ) => Some(End::Forced {
        ret: r1 || r2,
        throw: t1 || t2,
        infinite_loop: i1 || i2,
      }),
      _ => None,
    }
  }

  fn is_forced(&self) -> bool {
    matches!(self, End::Forced { .. })
  }
}

impl<'a> Scope<'a> {
  pub fn new(parent: Option<&'a Scope<'a>>, kind: BlockKind) -> Self {
    Self {
      _parent: parent,
      _kind: kind,
      used_hoistable_ids: Default::default(),
      end: None,
      may_throw: false,
      found_break: None,
      found_continue: false,
    }
  }
}

impl Analyzer<'_> {
  /// `lo` is marked as end if child scope is unconditionally finished
  pub(super) fn with_child_scope<F>(
    &mut self,
    kind: BlockKind,
    lo: BytePos,
    op: F,
  ) where
    F: for<'any> FnOnce(&mut Analyzer<'any>),
  {
    let prev_end = self.scope.end;
    let (info, end, hoist, found_break, found_continue, may_throw) = {
      let mut child = Analyzer {
        info: take(&mut self.info),
        scope: Scope::new(Some(&self.scope), kind.clone()),
      };
      match kind {
        BlockKind::Function => {}
        _ => match prev_end {
          Some(e) if matches!(e, End::Forced { .. }) => {
            child.scope.end = Some(e)
          }
          _ => {}
        },
      }

      op(&mut child);

      (
        take(&mut child.info),
        child.scope.end,
        child.scope.used_hoistable_ids,
        child.scope.found_break,
        child.scope.found_continue,
        child.scope.may_throw,
      )
    };

    self.info = info;
    self.scope.used_hoistable_ids.extend(hoist);

    // Preserve information about visited ast nodes.
    self.scope.may_throw |= may_throw;
    if self.scope.found_break.is_none() {
      self.scope.found_break = found_break;
    }
    self.scope.found_continue |= found_continue;

    if let Some(end) = end {
      match kind {
        BlockKind::Program => {}
        BlockKind::Function => {
          match end {
            End::Forced { .. } | End::Continue => self.mark_as_end(lo, end),
            _ => { /* valid code is supposed to be unreachable here */ }
          }
          self.scope.end = prev_end;
        }
        BlockKind::Case => {}
        BlockKind::If => {}
        BlockKind::Loop => match end {
          End::Break | End::Continue => {
            self.mark_as_end(lo, end);
            self.scope.end = prev_end;
          }
          e => {
            self.mark_as_end(lo, e);
            self.scope.end = Some(e);
          }
        },
        BlockKind::Label(label) => {
          if let Some(Some(id)) = &self.scope.found_break {
            if *id == label {
              // Eat break statement
              self.scope.found_break = None;
            }
          }
        }
        BlockKind::Catch => {
          self.mark_as_end(lo, end);
        }
        BlockKind::Finally => {
          self.mark_as_end(lo, end);
          match end {
            e if matches!(e, End::Forced { .. }) => {
              self.scope.end = Some(e);
            }
            _ => {
              self.scope.end = prev_end;
            }
          }
        }
      }
    }
  }

  fn get_end_reason(&self, lo: BytePos) -> Option<End> {
    self.info.get(&lo).map(|md| md.end).flatten()
  }

  /// Mark a statement as finisher - finishes execution - and expose it.
  fn mark_as_end(&mut self, lo: BytePos, end: End) {
    let new_end = match self.scope.end {
      // `End::Continue` doesn't mean much about execution status, just indicating that execution has
      // not yet stopped so far. So if `End::Forced` or `End::Break` comes and the current
      // `self.scope.end` is `Some(End::Continue)`, then `self.scope.end` should be replaced with the
      // coming value.
      None | Some(End::Continue) => {
        self.scope.end = Some(end);
        Some(end)
      }
      Some(End::Break) => Some(end),
      Some(e) => e.merge_forced(end).or(self.scope.end),
    };

    self.info.entry(lo).or_default().end = new_end;
  }

  /// Visits statement or block. This method handles break and continue.
  ///
  /// This cannot be done in visit_stmt of Visit because
  ///  this operation is very opinionated.
  fn visit_stmt_or_block(&mut self, s: &Stmt) {
    s.visit_with(&Invalid { span: DUMMY_SP }, self);

    // break, continue **may** make execution end
    match s {
      Stmt::Break(..) | Stmt::Continue(..) => {
        self.mark_as_end(s.span().lo, End::Break)
      }
      _ => {}
    }
  }
}

impl Visit for Analyzer<'_> {
  noop_visit_type!();

  fn visit_return_stmt(&mut self, n: &ReturnStmt, _: &dyn Node) {
    n.visit_children_with(self);
    self.mark_as_end(n.span().lo, End::forced_return());
  }

  fn visit_throw_stmt(&mut self, n: &ThrowStmt, _: &dyn Node) {
    n.visit_children_with(self);
    self.mark_as_end(n.span().lo, End::forced_throw());
  }

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

    if let Some(end) = self.scope.end {
      self.mark_as_end(s.span.lo, end);
    } else {
      self.mark_as_end(s.span.lo, End::Continue);
    }
  }

  fn visit_stmts(&mut self, stmts: &[Stmt], _: &dyn Node) {
    for stmt in stmts {
      self.visit_stmt_or_block(stmt);
    }
  }

  fn visit_expr(&mut self, n: &Expr, _: &dyn Node) {
    n.visit_children_with(self);

    if matches!(self.scope.end, None | Some(End::Continue)) {
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
    self.with_child_scope(BlockKind::Catch, n.span().lo, |a| {
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
    let prev_end = self.scope.end;
    n.visit_children_with(self);

    let end = {
      let has_default = n.cases.iter().any(|case| case.test.is_none());
      let forced_end = n
        .cases
        .iter()
        .filter_map(|case| self.get_end_reason(case.span.lo))
        .fold(
          Some(End::Forced {
            ret: false,
            throw: false,
            infinite_loop: false,
          }),
          |acc, cur| {
            if let Some(acc) = acc {
              acc.merge_forced(cur)
            } else {
              None
            }
          },
        );

      match forced_end {
        Some(e) if has_default => e,
        _ => End::Continue,
      }
    };

    self.mark_as_end(n.span.lo, end);

    if !matches!(end, End::Forced { .. }) {
      self.scope.end = prev_end;
    }
  }

  fn visit_switch_case(&mut self, n: &SwitchCase, _: &dyn Node) {
    let prev_end = self.scope.end;
    let mut case_end = None;

    self.with_child_scope(BlockKind::Case, n.span.lo, |a| {
      n.cons.visit_with(n, a);

      if a.scope.found_break.is_some() {
        case_end = Some(End::Break);
      } else if matches!(a.scope.end, Some(End::Forced { .. })) {
        case_end = a.scope.end;
      }
    });

    if let Some(end) = case_end {
      self.mark_as_end(n.span.lo, end);
    } else {
      self.mark_as_end(n.span.lo, End::Continue);
    }

    self.scope.end = prev_end;
  }

  fn visit_if_stmt(&mut self, n: &IfStmt, _: &dyn Node) {
    n.test.visit_with(n, self);

    let prev_end = self.scope.end;

    self.with_child_scope(BlockKind::If, n.cons.span().lo, |a| {
      a.visit_stmt_or_block(&n.cons);
    });

    let cons_reason = self.get_end_reason(n.cons.span().lo);

    match &n.alt {
      Some(alt) => {
        self.with_child_scope(BlockKind::If, alt.span().lo, |a| {
          a.visit_stmt_or_block(alt);
        });
        let alt_reason = self.get_end_reason(alt.span().lo);

        match (cons_reason, alt_reason) {
          (Some(x), Some(y)) if x.is_forced() && y.is_forced() => {
            // This `unwrap` is safe; `x` and `y` are surely `Some(End::Forced { .. })`
            let end = x.merge_forced(y).unwrap();
            self.mark_as_end(n.span.lo, end);
          }
          (Some(End::Break), Some(End::Break))
          | (Some(End::Forced { .. }), Some(End::Break))
          | (Some(End::Break), Some(End::Forced { .. })) => {
            self.mark_as_end(n.span.lo, End::Break);
          }
          // TODO: Check for continue
          _ => {
            self.mark_as_end(n.span.lo, End::Continue);
          }
        }
      }
      None => {
        self.mark_as_end(n.span.lo, End::Continue);
        self.scope.end = prev_end;
      }
    }
  }

  fn visit_stmt(&mut self, n: &Stmt, _: &dyn Node) {
    let scope_end = self
      .scope
      .end
      .map_or(false, |d| matches!(d, End::Forced { .. } | End::Break));

    let unreachable = if scope_end {
      // Although execution is ended, we should handle hoisting.
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

    self.info.entry(n.span().lo).or_default().unreachable = unreachable;

    n.visit_children_with(self);
  }

  // loops

  fn visit_for_stmt(&mut self, n: &ForStmt, _: &dyn Node) {
    n.init.visit_with(n, self);
    n.update.visit_with(n, self);
    n.test.visit_with(n, self);

    let mut forced_end = None;

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      n.body.visit_with(n, a);

      let has_break = matches!(a.scope.found_break, Some(None));

      if !has_break {
        let end = match a.get_end_reason(n.body.span().lo) {
          Some(e) if e.is_forced() => e,
          _ => End::forced_infinite_loop(),
        };
        match &n.test {
          None => {
            a.mark_as_end(n.span.lo, end);
            forced_end = Some(end);
          }
          Some(test) => {
            if matches!(test.as_bool(), (_, Value::Known(true))) {
              a.mark_as_end(n.span.lo, end);
              forced_end = Some(end);
            }
          }
        }
      }

      if forced_end.is_none() || has_break {
        a.mark_as_end(n.body.span().lo, End::Continue);
        a.scope.end = Some(End::Continue);
      }
    });
  }

  fn visit_for_of_stmt(&mut self, n: &ForOfStmt, _: &dyn Node) {
    let body_lo = n.body.span().lo;

    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      // it's impossible to decide whether it enters loop block unconditionally, so we always mark
      // it as `End::Continue`.
      a.mark_as_end(body_lo, End::Continue);
      a.scope.end = Some(End::Continue);
    });
  }

  fn visit_for_in_stmt(&mut self, n: &ForInStmt, _: &dyn Node) {
    let body_lo = n.body.span().lo;

    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      // it's impossible to decide whether it enters loop block unconditionally, so we always mark
      // it as `End::Continue`.
      a.mark_as_end(body_lo, End::Continue);
      a.scope.end = Some(End::Continue);
    });
  }

  fn visit_while_stmt(&mut self, n: &WhileStmt, _: &dyn Node) {
    let body_lo = n.body.span().lo;

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      let unconditionally_enter =
        matches!(n.test.as_bool(), (_, Value::Known(true)));
      let end_reason = a.get_end_reason(body_lo);
      let return_or_throw = end_reason.map_or(false, |e| e.is_forced());
      let has_break = matches!(a.scope.found_break, Some(None));

      if unconditionally_enter && return_or_throw && !has_break {
        // This `unwrap` is safe;
        // if `return_or_throw` is true, `end_reason` is surely wrapped in `Some`.
        a.mark_as_end(body_lo, end_reason.unwrap());
        a.scope.end = end_reason;
      } else if unconditionally_enter && !has_break {
        let end = End::forced_infinite_loop();
        a.mark_as_end(body_lo, end);
        a.scope.end = Some(end);
      } else {
        a.mark_as_end(body_lo, End::Continue);
        a.scope.end = Some(End::Continue);
      }
    });

    n.test.visit_with(n, self);
  }

  fn visit_do_while_stmt(&mut self, n: &DoWhileStmt, _: &dyn Node) {
    let body_lo = n.body.span().lo;

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      let end_reason = a.get_end_reason(body_lo);
      let return_or_throw = end_reason.map_or(false, |e| e.is_forced());
      let infinite_loop = matches!(n.test.as_bool(), (_, Value::Known(true)))
        && a.scope.found_break.is_none();
      let has_break = matches!(a.scope.found_break, Some(None));

      if return_or_throw && !has_break {
        // This `unwrap` is safe;
        // if `return_or_throw` is true, `end_reason` is surely wrapped in `Some`.
        a.mark_as_end(body_lo, end_reason.unwrap());
        a.scope.end = end_reason;
      } else if infinite_loop {
        let end = End::forced_infinite_loop();
        a.mark_as_end(body_lo, end);
        a.scope.end = Some(end);
      } else {
        a.mark_as_end(body_lo, End::Continue);
        a.scope.end = Some(End::Continue);
      }
    });

    match self.get_end_reason(body_lo) {
      Some(e) if e.is_forced() => {
        self.mark_as_end(n.span.lo, e);
      }
      _ => {}
    }

    n.test.visit_with(n, self);
  }

  fn visit_try_stmt(&mut self, n: &TryStmt, _: &dyn Node) {
    if let Some(finalizer) = &n.finalizer {
      self.with_child_scope(BlockKind::Finally, finalizer.span.lo, |a| {
        n.finalizer.visit_with(n, a);
      });
    }
    let old_throw = self.scope.may_throw;

    let prev_end = self.scope.end;

    self.scope.may_throw = false;
    n.block.visit_with(n, self);

    let mut try_block_end = None;

    if self.scope.may_throw {
      if let Some(end) = self.scope.end {
        try_block_end = Some(end);
        self.scope.end = prev_end;
      }
    } else if let Some(end) = self.scope.end {
      try_block_end = Some(end);
      self.mark_as_end(n.span.lo, end);
    }

    if let Some(handler) = &n.handler {
      self.scope.may_throw = false;
      handler.visit_with(n, self);

      match (try_block_end, self.scope.end) {
        (Some(x), Some(y)) if x.is_forced() && y.is_forced() => {
          // This `unwrap` is safe; `x` and `y` are surely `Some(End::Forced { .. })`
          self.mark_as_end(n.span.lo, x.merge_forced(y).unwrap());
        }
        (Some(x), Some(End::Break)) if x.is_forced() => {
          self.mark_as_end(n.span.lo, End::Break);
        }
        _ => {
          self.mark_as_end(n.span.lo, End::Continue);
          self.scope.end = prev_end;
        }
      }
    } else if matches!(
      try_block_end,
      Some(End::Forced { .. }) | Some(End::Break)
    ) {
      // This `unwrap` is safe; `try_block_end` is surely wrapped in `Some`
      self.mark_as_end(n.span.lo, try_block_end.unwrap());
    } else if let Some(finalizer) = &n.finalizer {
      self.mark_as_end(
        n.span.lo,
        self
          .get_end_reason(finalizer.span.lo)
          .unwrap_or(End::Continue),
      );
      self.scope.end = prev_end;
      self.scope.may_throw = old_throw;
    } else {
      self.scope.end = prev_end;
    }
  }

  fn visit_labeled_stmt(&mut self, n: &LabeledStmt, _: &dyn Node) {
    self.with_child_scope(BlockKind::Label(n.label.to_id()), n.span.lo, |a| {
      a.visit_stmt_or_block(&n.body);
    });
  }
}
