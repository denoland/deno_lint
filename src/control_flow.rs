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

#[derive(Debug, Default)]
pub struct ControlFlow {
  meta: BTreeMap<BytePos, Metadata>,
}

impl ControlFlow {
  pub fn analyze(program: &Program) -> Self {
    let mut v = Analyzer {
      scope: Scope::new(None, BlockKind::Program),
      info: Default::default(),
    };
    program.visit_with(&Invalid { span: DUMMY_SP }, &mut v);
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
      .map_or(false, |d| matches!(d, End::Forced | End::Break))
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
  /// Return, Throw, or infinite loop
  Forced,
  /// Break or continue
  Break,
  /// Pass through a block, like a function's block statement which ends without returning a value
  /// or throwing an exception. Note that a node marked as `End::Continue` won't prevent further execution, which is
  /// different from `End::Forced` or `End::Break`.
  Continue,
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
        _ => {
          if let Some(End::Forced) = prev_end {
            child.scope.end = Some(End::Forced);
          }
        }
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
            End::Forced | End::Continue => self.mark_as_end(lo, end),
            _ => unreachable!(),
          }
          self.scope.end = prev_end;
        }
        BlockKind::Case => {}
        BlockKind::If => {}
        BlockKind::Loop => match end {
          End::Forced => {
            self.mark_as_end(lo, End::Forced);
            self.scope.end = Some(End::Forced);
          }
          End::Break | End::Continue => {
            self.mark_as_end(lo, end);
            self.scope.end = prev_end;
          }
        },
        BlockKind::Label(label) => {
          if let Some(Some(id)) = &self.scope.found_break {
            if *id == label {
              // Eat break statemnt
              self.scope.found_break = None;
            }
          }
        }
        BlockKind::Catch => {
          self.mark_as_end(lo, end);
        }
        BlockKind::Finally => {
          self.mark_as_end(lo, end);
          if end == End::Forced {
            self.scope.end = Some(End::Forced);
          } else {
            self.scope.end = prev_end;
          }
        }
      }
    }
  }

  fn is_forced_end(&self, lo: BytePos) -> bool {
    matches!(self.get_end_reason(lo), Some(End::Forced))
  }

  fn get_end_reason(&self, lo: BytePos) -> Option<End> {
    self.info.get(&lo).map(|md| md.end).flatten()
  }

  /// Mark a statement as finisher - finishes execution - and expose it.
  fn mark_as_end(&mut self, lo: BytePos, end: End) {
    // `End::Continue` doesn't mean much about execution status, just indicating that execution has
    // not yet stopped so far. So if `End::Forced` or `End::Break` comes and the current
    // `self.scope.end` is `Some(End::Continue)`, then `self.scope.end` should be replaced with the
    // coming value.
    if matches!(self.scope.end, None | Some(End::Continue)) {
      self.scope.end = Some(end);
    }
    self.info.entry(lo).or_default().end = Some(end);
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

macro_rules! mark_as_end {
  ($name:ident, $T:ty) => {
    fn $name(&mut self, s: &$T, _: &dyn Node) {
      s.visit_children_with(self);

      self.mark_as_end(s.span().lo, End::Forced);
    }
  };
}

impl Visit for Analyzer<'_> {
  noop_visit_type!();

  mark_as_end!(visit_return_stmt, ReturnStmt);
  mark_as_end!(visit_throw_stmt, ThrowStmt);

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

    if self.scope.end.is_none() {
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

    let has_default = n.cases.iter().any(|case| case.test.is_none());

    // SwitchStmt finishes execution if all cases finishes execution
    let is_end = has_default
      && n
        .cases
        .iter()
        .map(|case| case.span.lo)
        .all(|lo| self.is_forced_end(lo));

    // A switch statement is finisher or not.
    if is_end {
      self.mark_as_end(n.span.lo, End::Forced);
    } else {
      self.mark_as_end(n.span.lo, End::Continue);
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
      } else if a.scope.end == Some(End::Forced) {
        case_end = Some(End::Forced);
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
          a.visit_stmt_or_block(&alt);
        });
        let alt_reason = self.get_end_reason(alt.span().lo);

        match (cons_reason, alt_reason) {
          (Some(End::Forced), Some(End::Forced)) => {
            self.mark_as_end(n.span.lo, End::Forced);
          }
          (Some(End::Break), Some(End::Break))
          | (Some(End::Forced), Some(End::Break))
          | (Some(End::Break), Some(End::Forced)) => {
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
      .map_or(false, |d| matches!(d, End::Forced | End::Break));

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

    let mut is_infinite_loop = false;

    self.with_child_scope(BlockKind::Loop, n.body.span().lo, |a| {
      n.body.visit_with(n, a);

      if a.scope.found_break.is_none() {
        match &n.test {
          None => {
            // Infinite loop
            a.mark_as_end(n.span.lo, End::Forced);
            is_infinite_loop = true;
          }
          Some(test) => {
            if matches!(test.as_bool(), (_, Value::Known(true))) {
              // Infinite loop
              a.mark_as_end(n.span.lo, End::Forced);
              is_infinite_loop = true;
            }
          }
        }
      }

      if !is_infinite_loop {
        a.mark_as_end(n.span.lo, End::Continue);
        a.scope.end = Some(End::Continue);
      }
    });

    if is_infinite_loop {
      self.scope.end = Some(End::Forced);
    }
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
      let return_or_throw = a.get_end_reason(body_lo) == Some(End::Forced);
      let infinite_loop = a.scope.found_break.is_none();

      if unconditionally_enter && (return_or_throw || infinite_loop) {
        a.mark_as_end(body_lo, End::Forced);
        a.scope.end = Some(End::Forced);
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

      let return_or_throw = a.get_end_reason(body_lo) == Some(End::Forced);
      let infinite_loop = matches!(n.test.as_bool(), (_, Value::Known(true)))
        && a.scope.found_break.is_none();

      if return_or_throw || infinite_loop {
        a.mark_as_end(body_lo, End::Forced);
        a.scope.end = Some(End::Forced);
      }
    });

    if self.get_end_reason(body_lo) == Some(End::Forced) {
      self.mark_as_end(n.span.lo, End::Forced);
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
      handler.visit_with(n, self);
      match (try_block_end, self.scope.end) {
        (Some(End::Forced), Some(End::Forced)) => {
          self.mark_as_end(n.span.lo, End::Forced);
        }
        (Some(End::Forced), Some(End::Break)) => {
          self.mark_as_end(n.span.lo, End::Break);
        }
        _ => {
          self.mark_as_end(n.span.lo, End::Continue);
          self.scope.end = prev_end;
        }
      }
    } else if matches!(try_block_end, Some(End::Forced) | Some(End::Break)) {
      self.mark_as_end(n.span.lo, try_block_end.unwrap());
    } else if let Some(finalizer) = &n.finalizer {
      self.mark_as_end(
        n.span.lo,
        self
          .get_end_reason(finalizer.span.lo)
          .unwrap_or(End::Continue),
      );
      self.scope.end = prev_end;
    } else {
      self.scope.end = prev_end;
    }

    self.scope.may_throw = old_throw;
  }

  fn visit_labeled_stmt(&mut self, n: &LabeledStmt, _: &dyn Node) {
    self.with_child_scope(BlockKind::Label(n.label.to_id()), n.span.lo, |a| {
      a.visit_stmt_or_block(&n.body);
    });
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  fn analyze_flow(src: &str) -> ControlFlow {
    let program = parse(src);
    ControlFlow::analyze(&program)
  }

  macro_rules! assert_flow {
    ($flow:ident, $lo:expr, $unreachable:expr, $end:expr) => {
      assert_eq!(
        $flow.meta(BytePos($lo)).unwrap(),
        &Metadata {
          unreachable: $unreachable,
          end: $end,
        }
      );
    };
  }

  #[test]
  fn while_1() {
    let src = r#"
function foo() {
  while (a) {
    break;
  }
  return 1;
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 30, false, Some(End::Continue)); // BlockStmt of while
    assert_flow!(flow, 49, false, Some(End::Forced)); // return stmt
  }

  #[test]
  fn while_2() {
    let src = r#"
function foo() {
  while (a) {
    break;
  }
  bar();
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 30, false, Some(End::Continue)); // BlockStmt of while
    assert_flow!(flow, 49, false, None); // `bar();`
  }

  #[test]
  fn while_3() {
    let src = r#"
function foo() {
  while (a) {
    bar();
  }
  baz();
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 30, false, Some(End::Continue)); // BlockStmt of while
    assert_flow!(flow, 36, false, None); // `bar();`
    assert_flow!(flow, 49, false, None); // `baz();`
  }

  #[test]
  fn while_4() {
    let src = r#"
function foo() {
  while (a) {
    return 1;
  }
  baz();
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`

    // BlockStmt of while
    // This block contains `return 1;` but whether entering the block depends on the specific value
    // of `a`, so we treat it as `End::Continue`.
    assert_flow!(flow, 30, false, Some(End::Continue));

    assert_flow!(flow, 36, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 52, false, None); // `baz();`
  }

  #[test]
  fn while_5() {
    let src = r#"
function foo() {
  while (true) {
    return 1;
  }
  baz();
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`

    // BlockStmt of while
    // This block contains `return 1;` and it returns `1` _unconditionally_.
    assert_flow!(flow, 33, false, Some(End::Forced));

    assert_flow!(flow, 39, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 55, true, None); // `baz();`
  }

  #[test]
  fn do_while_1() {
    let src = r#"
function foo() {
  do {
    break;
  } while (a);
  return 1;
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 23, false, Some(End::Break)); // BlockStmt of do-while
    assert_flow!(flow, 53, false, Some(End::Forced)); // return stmt
  }

  #[test]
  fn do_while_2() {
    let src = r#"
function foo() {
  do {
    break;
  } while (a);
  bar();
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 23, false, Some(End::Break)); // BlockStmt of do-while
    assert_flow!(flow, 53, false, None); // `bar();`
  }

  #[test]
  fn do_while_3() {
    let src = r#"
function foo() {
  do {
    bar();
  } while (a);
  baz();
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 23, false, Some(End::Continue)); // BlockStmt of do-while
    assert_flow!(flow, 53, false, None); // `bar();`
  }

  #[test]
  fn do_while_4() {
    // infinite loop
    let src = r#"
function foo() {
  do {
    bar();
  } while (true);
  return 1;
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 23, false, Some(End::Forced)); // BlockStmt of do-while
    assert_flow!(flow, 56, true, Some(End::Forced)); // return stmt
  }

  #[test]
  fn do_while_5() {
    let src = r#"
function foo() {
  do {
    return 0;
  } while (a);
  return 1;
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 23, false, Some(End::Forced)); // BlockStmt of do-while
    assert_flow!(flow, 56, true, Some(End::Forced)); // return stmt
  }

  #[test]
  fn do_while_6() {
    let src = r#"
function foo() {
  do {
    throw 0;
  } while (false);
  return 1;
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 23, false, Some(End::Forced)); // BlockStmt of do-while
    assert_flow!(flow, 59, true, Some(End::Forced)); // return stmt
  }

  #[test]
  fn do_while_7() {
    let src = r#"
function foo() {
  do {
    throw 0;
  } while (a);
  return 1;
}
      "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 23, false, Some(End::Forced)); // BlockStmt of do-while
    assert_flow!(flow, 29, false, Some(End::Forced)); // throw stmt
    assert_flow!(flow, 55, true, Some(End::Forced)); // return stmt
  }

  #[test]
  fn for_1() {
    let src = r#"
function foo() {
  for (let i = 0; f(); i++) {
    return 1;
  }
  bar();
}
    "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`

    // BlockStmt of for statement
    // This is marked as `End::Continue` because it's quite difficult to decide statically whether
    // the program enters the block or not.
    assert_flow!(flow, 46, false, Some(End::Continue));

    assert_flow!(flow, 52, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 68, false, None); // `bar();`
  }

  #[test]
  fn for_2() {
    // infinite loop
    let src = r#"
function foo() {
  for (let i = 0; true; i++) {
    return 1;
  }
  bar();
}
    "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 47, false, Some(End::Forced)); // BlockStmt of for statement
    assert_flow!(flow, 53, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 69, true, None); // `bar();`
  }

  #[test]
  fn for_3() {
    // infinite loop
    let src = r#"
function foo() {
  for (let i = 0;; i++) {
    return 1;
  }
  bar();
}
    "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 42, false, Some(End::Forced)); // BlockStmt of for statement
    assert_flow!(flow, 48, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 64, true, None); // `bar();`
  }

  #[test]
  fn for_4() {
    // never enter the block of for
    let src = r#"
function foo() {
  for (let i = 0; false; i++) {
    return 1;
  }
  bar();
}
    "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 48, false, Some(End::Continue)); // BlockStmt of for statement
    assert_flow!(flow, 54, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 70, false, None); // `bar();`
  }

  #[test]
  fn for_in_1() {
    let src = r#"
function foo() {
  for (let i in {}) {
    return 1;
  }
  bar();
}
    "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 38, false, Some(End::Continue)); // BlockStmt of for-in
    assert_flow!(flow, 44, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 60, false, None); // `bar();`
  }

  #[test]
  fn for_of_1() {
    let src = r#"
function foo() {
  for (let i of []) {
    return 1;
  }
  bar();
}
    "#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 38, false, Some(End::Continue)); // BlockStmt of for-of
    assert_flow!(flow, 44, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 60, false, None); // `bar();`
  }

  #[test]
  fn try_1() {
    let src = r#"
function foo() {
  try {
    return 1;
  } finally {
    bar();
  }
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Forced)); // TryStmt
    assert_flow!(flow, 24, false, Some(End::Forced)); // BlockStmt of try
    assert_flow!(flow, 30, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 52, false, Some(End::Continue)); // BlockStmt of finally
    assert_flow!(flow, 58, false, None); // `bar();`
  }

  #[test]
  fn try_2() {
    let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    return 2;
  }
  bar();
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Forced)); // TryStmt
    assert_flow!(flow, 24, false, Some(End::Forced)); // BlockStmt of try
    assert_flow!(flow, 30, false, Some(End::Forced)); // throw stmt
    assert_flow!(flow, 43, false, Some(End::Forced)); // catch
    assert_flow!(flow, 53, false, Some(End::Forced)); // BlockStmt of catch
    assert_flow!(flow, 59, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 75, true, None); // `bar();`
  }

  #[test]
  fn try_3() {
    let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    bar();
  }
  baz();
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Continue)); // TryStmt
    assert_flow!(flow, 24, false, Some(End::Forced)); // BlockStmt of try
    assert_flow!(flow, 30, false, Some(End::Forced)); // throw stmt
    assert_flow!(flow, 43, false, Some(End::Continue)); // catch
    assert_flow!(flow, 53, false, Some(End::Continue)); // BlockStmt of catch
    assert_flow!(flow, 59, false, None); // `bar();`
    assert_flow!(flow, 72, false, None); // `baz();`
  }

  #[test]
  fn try_4() {
    let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    bar();
  } finally {
    baz();
  }
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Continue)); // TryStmt
    assert_flow!(flow, 24, false, Some(End::Forced)); // BlockStmt of try
    assert_flow!(flow, 30, false, Some(End::Forced)); // throw stmt
    assert_flow!(flow, 43, false, Some(End::Continue)); // catch
    assert_flow!(flow, 53, false, Some(End::Continue)); // BlockStmt of catch
    assert_flow!(flow, 59, false, None); // `bar();`
    assert_flow!(flow, 78, false, Some(End::Continue)); // BlockStmt of finally
    assert_flow!(flow, 84, false, None); // `baz();`
  }

  #[test]
  fn try_5() {
    let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    return 2;
  } finally {
    bar();
  }
  baz();
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Forced)); // TryStmt
    assert_flow!(flow, 24, false, Some(End::Forced)); // BlockStmt of try
    assert_flow!(flow, 30, false, Some(End::Forced)); // throw stmt
    assert_flow!(flow, 43, false, Some(End::Forced)); // catch
    assert_flow!(flow, 53, false, Some(End::Forced)); // BlockStmt of catch
    assert_flow!(flow, 59, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 81, false, Some(End::Continue)); // BlockStmt of finally
    assert_flow!(flow, 87, false, None); // `bar();`
    assert_flow!(flow, 100, true, None); // `baz();`
  }

  #[test]
  fn try_6() {
    let src = r#"
try {}
finally {
  break;
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 1, false, Some(End::Break)); // try stmt
    assert_flow!(flow, 5, false, Some(End::Continue)); // BlockStmt of try
    assert_flow!(flow, 16, false, Some(End::Break)); // BlockStmt of finally
    assert_flow!(flow, 20, false, Some(End::Break)); // break stmt
  }

  #[test]
  fn try_7() {
    let src = r#"
try {
  throw 0;
} catch (e) {
  break;
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 1, false, Some(End::Break)); // try stmt
    assert_flow!(flow, 5, false, Some(End::Forced)); // BlockStmt of try
    assert_flow!(flow, 9, false, Some(End::Forced)); // throw stmt
    assert_flow!(flow, 20, false, Some(End::Break)); // catch
    assert_flow!(flow, 30, false, Some(End::Break)); // BloskStmt of catch
    assert_flow!(flow, 34, false, Some(End::Break)); // break stmt
  }

  #[test]
  fn try_8() {
    let src = r#"
try {
  break;
} finally {}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 1, false, Some(End::Break)); // try stmt
    assert_flow!(flow, 5, false, Some(End::Break)); // BlockStmt of try
    assert_flow!(flow, 9, false, Some(End::Break)); // break stmt
    assert_flow!(flow, 26, false, Some(End::Continue)); // finally
  }

  #[test]
  fn if_1() {
    let src = r#"
function foo() {
  if (a) {
    return 1;
  }
  bar();
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Continue)); // if
    assert_flow!(flow, 27, false, Some(End::Forced)); // BloskStmt of if
    assert_flow!(flow, 33, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 49, false, None); // `bar();`
  }

  #[test]
  fn if_2() {
    let src = r#"
function foo() {
  if (a) {
    bar();
  } else {
    return 1;
  }
  baz();
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Continue)); // if
    assert_flow!(flow, 27, false, Some(End::Continue)); // BloskStmt of if
    assert_flow!(flow, 33, false, None); // `bar();`
    assert_flow!(flow, 49, false, Some(End::Forced)); // else
    assert_flow!(flow, 55, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 71, false, None); // `baz();`
  }

  #[test]
  fn if_3() {
    let src = r#"
function foo() {
  if (a) {
    return 1;
  } else {
    bar();
  }
  return 0;
}
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 16, false, Some(End::Forced)); // BlockStmt of `foo`
    assert_flow!(flow, 20, false, Some(End::Continue)); // if
    assert_flow!(flow, 27, false, Some(End::Forced)); // BloskStmt of if
    assert_flow!(flow, 33, false, Some(End::Forced)); // `return 1;`
    assert_flow!(flow, 52, false, Some(End::Continue)); // else
    assert_flow!(flow, 58, false, None); // `bar();`
    assert_flow!(flow, 71, false, Some(End::Forced)); // `return 0;`
  }

  #[test]
  fn switch_1() {
    let src = r#"
switch (foo) {
  case 1:
    return 0;
  default: {
    if (bar) {
      break;
    }
    return 0;
  }
}
throw err;
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 1, false, Some(End::Continue)); // switch stmt
    assert_flow!(flow, 18, false, Some(End::Forced)); // `case 1`
    assert_flow!(flow, 30, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 42, false, Some(End::Break)); // `default`
    assert_flow!(flow, 51, false, Some(End::Forced)); // BlockStmt of `default`
    assert_flow!(flow, 57, false, Some(End::Continue)); // if
    assert_flow!(flow, 66, false, Some(End::Break)); // BlockStmt of if
    assert_flow!(flow, 74, false, Some(End::Break)); // break stmt
    assert_flow!(flow, 91, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 107, false, Some(End::Forced)); // throw stmt
  }

  #[test]
  fn switch_2() {
    let src = r#"
switch (foo) {
  case 1:
    return 0;
  default: {
    return 0;
  }
}
throw err;
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 1, false, Some(End::Forced)); // switch stmt
    assert_flow!(flow, 18, false, Some(End::Forced)); // `case 1`
    assert_flow!(flow, 30, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 42, false, Some(End::Forced)); // `default`
    assert_flow!(flow, 51, false, Some(End::Forced)); // BlockStmt of `default`
    assert_flow!(flow, 57, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 73, true, Some(End::Forced)); // throw stmt
  }

  #[test]
  fn switch_3() {
    let src = r#"
switch (foo) {
  case 1:
    break;
  default: {
    return 0;
  }
}
throw err;
"#;
    let flow = analyze_flow(src);
    assert_flow!(flow, 1, false, Some(End::Continue)); // switch stmt
    assert_flow!(flow, 18, false, Some(End::Break)); // `case 1`
    assert_flow!(flow, 30, false, Some(End::Break)); // break stmt
    assert_flow!(flow, 39, false, Some(End::Forced)); // `default`
    assert_flow!(flow, 48, false, Some(End::Forced)); // BlockStmt of `default`
    assert_flow!(flow, 54, false, Some(End::Forced)); // return stmt
    assert_flow!(flow, 70, false, Some(End::Forced)); // throw stmt
  }
}
