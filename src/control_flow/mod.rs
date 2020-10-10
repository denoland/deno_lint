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

#[derive(Debug)]
pub struct ControlFlow {
  meta: BTreeMap<BytePos, Metadata>,
}

impl ControlFlow {
  pub fn analyze(m: &Module) -> Self {
    let mut v = Analyzer {
      scope: Scope::new(None, BlockKind::Module),
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
  Module,
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

#[derive(Debug, Default, Clone, PartialEq, Eq)]
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
  done: Option<Done>,

  may_throw: bool,

  ///
  /// - None: Not found
  /// - Some(None): Stopped at a break statement without label
  /// - Some(Somd(id)): Stopped at a break statement with label id
  found_break: Option<Option<Id>>,
  found_continue: bool,
}

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum Done {
  /// Return, Throw, or infinite loop
  Forced,
  // Break or continue
  Break,
  // Pass through a block, like a function's block statement which ends without returning a value
  // or throwing an exception
  Pass,
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

    self.info = info;
    self.scope.used_hoistable_ids.extend(hoist);

    // Preserve information about visited ast nodes.
    self.scope.may_throw |= may_throw;
    if self.scope.found_break.is_none() {
      self.scope.found_break = found_break;
    }
    self.scope.found_continue |= found_continue;

    if let Some(done) = done {
      match kind {
        BlockKind::Module => {}
        BlockKind::Function => {
          match done {
            Done::Forced | Done::Pass => self.mark_as_done(lo, done),
            _ => unreachable!(),
          }
          self.scope.done = prev_done;
        }
        BlockKind::Block => {
          if let Done::Forced = done {
            self.mark_as_done(lo, done);
          } else if self.scope.done.is_none() {
            self.scope.done = Some(Done::Break);
          }
        }
        BlockKind::Case => {
          if let Done::Forced = done {
            self.mark_as_done(lo, done);
          }
        }
        BlockKind::If => {}
        BlockKind::Loop => match done {
          Done::Forced => {
            self.mark_as_done(lo, Done::Forced);
            self.scope.done = Some(Done::Forced);
          }
          Done::Break | Done::Pass => {
            self.mark_as_done(lo, done);
            self.scope.done = prev_done;
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
      }
    }
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
    } else {
      self.mark_as_done(s.span.lo, Done::Pass);
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

    self.info.entry(n.span().lo).or_default().unreachable = unreachable;

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
    let body_lo = n.body.span().lo;

    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      // it's impossible to decide whether it enters loop block unconditionally, so we always mark
      // it as `Done::Pass`.
      a.mark_as_done(body_lo, Done::Pass);
      a.scope.done = Some(Done::Pass);
    });
  }

  fn visit_for_in_stmt(&mut self, n: &ForInStmt, _: &dyn Node) {
    let body_lo = n.body.span().lo;

    n.right.visit_with(n, self);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      // it's impossible to decide whether it enters loop block unconditionally, so we always mark
      // it as `Done::Pass`.
      a.mark_as_done(body_lo, Done::Pass);
      a.scope.done = Some(Done::Pass);
    });
  }

  fn visit_while_stmt(&mut self, n: &WhileStmt, _: &dyn Node) {
    let body_lo = n.body.span().lo;

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      let unconditionally_enter =
        matches!(n.test.as_bool(), (_, Value::Known(true)));
      let return_or_throw = a.get_done_reason(body_lo) == Some(Done::Forced);
      let infinite_loop = a.scope.found_break.is_none();

      if unconditionally_enter && (return_or_throw || infinite_loop) {
        a.mark_as_done(body_lo, Done::Forced);
        a.scope.done = Some(Done::Forced);
      } else {
        a.mark_as_done(body_lo, Done::Pass);
        a.scope.done = Some(Done::Pass);
      }
    });

    n.test.visit_with(n, self);
  }

  fn visit_do_while_stmt(&mut self, n: &DoWhileStmt, _: &dyn Node) {
    let body_lo = n.body.span().lo;

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      n.body.visit_with(n, a);

      let return_or_throw = a.get_done_reason(body_lo) == Some(Done::Forced);
      let infinite_loop = matches!(n.test.as_bool(), (_, Value::Known(true)))
        && a.scope.found_break.is_none();

      if return_or_throw || infinite_loop {
        a.mark_as_done(body_lo, Done::Forced);
        a.scope.done = Some(Done::Forced);
      }
    });

    n.test.visit_with(n, self);
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

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  fn analyze_flow(src: &str) -> ControlFlow {
    let module = parse(src);
    ControlFlow::analyze(&module)
  }

  macro_rules! assert_meta {
    ($flow:ident, $lo:expr, $unreachable:expr, $done:expr) => {
      assert_eq!(
        $flow.meta(BytePos($lo)).unwrap(),
        &Metadata {
          unreachable: $unreachable,
          done: $done,
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Forced)); // BlockStmt of `foo`
    assert_meta!(flow, 30, false, Some(Done::Pass)); // BlockStmt of while
    assert_meta!(flow, 49, false, Some(Done::Forced)); // return stmt
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Pass)); // BlockStmt of `foo`
    assert_meta!(flow, 30, false, Some(Done::Pass)); // BlockStmt of while
    assert_meta!(flow, 49, false, None); // `bar();`
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Pass)); // BlockStmt of `foo`
    assert_meta!(flow, 30, false, Some(Done::Pass)); // BlockStmt of while
    assert_meta!(flow, 36, false, None); // `bar();`
    assert_meta!(flow, 49, false, None); // `baz();`
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Pass)); // BlockStmt of `foo`

    // BlockStmt of while
    // This block contains `return 1;` but whether entering the block depends on the specific value
    // of `a`, so we treat it as `Done::Pass`.
    assert_meta!(flow, 30, false, Some(Done::Pass));

    assert_meta!(flow, 36, false, Some(Done::Forced)); // return stmt
    assert_meta!(flow, 52, false, None); // `baz();`
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Forced)); // BlockStmt of `foo`

    // BlockStmt of while
    // This block contains `return 1;` and it returns `1` _unconditionally_.
    assert_meta!(flow, 33, false, Some(Done::Forced));

    assert_meta!(flow, 39, false, Some(Done::Forced)); // return stmt
    assert_meta!(flow, 55, true, None); // `baz();`
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
    assert_meta!(flow, 16, false, Some(Done::Forced)); // BlockStmt of `foo`
    assert_meta!(flow, 23, false, Some(Done::Break)); // BlockStmt of do-while
    assert_meta!(flow, 53, false, Some(Done::Forced)); // return stmt
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
    assert_meta!(flow, 16, false, Some(Done::Pass)); // BlockStmt of `foo`
    assert_meta!(flow, 23, false, Some(Done::Break)); // BlockStmt of do-while
    assert_meta!(flow, 53, false, None); // `bar();`
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
    assert_meta!(flow, 16, false, Some(Done::Pass)); // BlockStmt of `foo`
    assert_meta!(flow, 23, false, Some(Done::Pass)); // BlockStmt of do-while
    assert_meta!(flow, 53, false, None); // `bar();`
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Forced)); // BlockStmt of `foo`
    assert_meta!(flow, 23, false, Some(Done::Forced)); // BlockStmt of do-while
    assert_meta!(flow, 56, true, Some(Done::Forced)); // return stmt
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Forced)); // BlockStmt of `foo`
    assert_meta!(flow, 23, false, Some(Done::Forced)); // BlockStmt of do-while
    assert_meta!(flow, 56, true, Some(Done::Forced)); // return stmt
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Forced)); // BlockStmt of `foo`
    assert_meta!(flow, 23, false, Some(Done::Forced)); // BlockStmt of do-while
    assert_meta!(flow, 59, true, Some(Done::Forced)); // return stmt
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Pass)); // BlockStmt of `foo`
    assert_meta!(flow, 38, false, Some(Done::Pass)); // BlockStmt of for-in
    assert_meta!(flow, 44, false, Some(Done::Forced)); // return stmt
    assert_meta!(flow, 60, false, None); // `bar();`
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
    dbg!(&flow);
    assert_meta!(flow, 16, false, Some(Done::Pass)); // BlockStmt of `foo`
    assert_meta!(flow, 38, false, Some(Done::Pass)); // BlockStmt of for-of
    assert_meta!(flow, 44, false, Some(Done::Forced)); // return stmt
    assert_meta!(flow, 60, false, None); // `bar();`
  }

  #[test]
  fn piyo() {
    let src = "function foo() { var x = 1; for (x in {}) { return; } x = 2; }";
    let flow = analyze_flow(src);
    dbg!(&flow);
    panic!();
  }

  #[test]
  fn hoge() {
    let src = r#"
const obj = {
  get root() {
    let primary = this;
    while (true) {
      if (primary.parent !== undefined) {
          primary = primary.parent;
      } else {
          return primary;
      }
    }
    //return 'a';
  }
};
      "#;
    let module = parse(src);
    let flow = ControlFlow::analyze(&module);
    dbg!(flow);
    panic!();
  }
}
