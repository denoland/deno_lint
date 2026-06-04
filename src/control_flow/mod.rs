// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

#[cfg(test)]
mod analyze_test;

use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::walk;
use deno_ast::oxc::ast_visit::Visit;
use deno_ast::oxc::span::GetSpan;
use deno_ast::ParsedSource;
use std::{
  collections::{BTreeMap, HashSet},
  mem::take,
};

#[derive(Debug, Clone)]
pub struct ControlFlow {
  meta: BTreeMap<u32, Metadata>,
}

impl ControlFlow {
  pub fn analyze(parsed_source: &ParsedSource) -> Self {
    let mut v = Analyzer {
      scope: Scope::new(None, BlockKind::Program),
      info: Default::default(),
    };
    v.visit_program(parsed_source.program());
    ControlFlow { meta: v.info }
  }

  /// start_pos can be extracted from span.start of
  ///
  /// - All statements (including stmt.span().start)
  /// - [SwitchCase]
  pub fn meta(&self, start_pos: u32) -> Option<&Metadata> {
    self.meta.get(&start_pos)
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
  Label(String),
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
      .is_some_and(|d| matches!(d, End::Forced { .. } | End::Break))
  }

  /// Returns true if a node doesn't prevent further execution.
  pub fn continues_execution(&self) -> bool {
    self.end.is_none_or(|d| d == End::Continue)
  }
}

#[derive(Debug)]
struct Analyzer<'a> {
  scope: Scope<'a>,
  info: BTreeMap<u32, Metadata>,
}

#[derive(Debug)]
struct Scope<'a> {
  _parent: Option<&'a Scope<'a>>,
  /// This field exists to handle code like
  ///
  /// `function foo() { return bar(); function bar() { return 1; } }`
  used_hoistable_ids: HashSet<String>,

  _kind: BlockKind,

  /// Unconditionally ends with return, throw
  end: Option<End>,

  may_throw: bool,

  ///
  /// - None: Not found
  /// - Some(None): Stopped at a break statement without label
  /// - Some(Some(id)): Stopped at a break statement with label id
  found_break: Option<Option<String>>,
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

/// Simple check if an expression is known to be truthy at compile time.
fn is_definitely_truthy(expr: &Expression) -> bool {
  match expr {
    Expression::BooleanLiteral(b) => b.value,
    Expression::NumericLiteral(n) => n.value != 0.0 && !n.value.is_nan(),
    Expression::StringLiteral(s) => !s.value.is_empty(),
    _ => false,
  }
}

fn stmt_start(s: &Statement) -> u32 {
  s.span().start
}

impl Analyzer<'_> {
  /// `lo` is marked as end if child scope is unconditionally finished
  pub(super) fn with_child_scope<F>(
    &mut self,
    kind: BlockKind,
    start_pos: u32,
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

    self.scope.found_continue |= found_continue;

    match kind {
      BlockKind::Case => {}
      BlockKind::Function => {}
      BlockKind::Loop => {}
      _ => {
        if self.scope.found_break.is_none() {
          self.scope.found_break = found_break;
        }
      }
    };

    if let Some(end) = end {
      match kind {
        BlockKind::Program => {}
        BlockKind::Function => {
          match end {
            End::Forced { .. } | End::Continue => {
              self.mark_as_end(start_pos, end)
            }
            _ => { /* valid code is supposed to be unreachable here */ }
          }
          self.scope.end = prev_end;
        }
        BlockKind::Case => {}
        BlockKind::If => {}
        BlockKind::Loop => match end {
          End::Break | End::Continue => {
            self.mark_as_end(start_pos, end);
            self.scope.end = prev_end;
          }
          e => {
            self.mark_as_end(start_pos, e);
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
          self.mark_as_end(start_pos, end);
        }
        BlockKind::Finally => {
          self.mark_as_end(start_pos, end);
        }
      }
    }
  }

  fn get_end_reason(&self, start: u32) -> Option<End> {
    self.info.get(&start).and_then(|md| md.end)
  }

  /// Mark a statement as finisher - finishes execution - and expose it.
  fn mark_as_end(&mut self, start: u32, end: End) {
    let new_end = match self.scope.end {
      None | Some(End::Continue) => {
        self.scope.end = Some(end);
        Some(end)
      }
      Some(End::Break) => Some(end),
      Some(e) => e.merge_forced(end).or(self.scope.end),
    };

    self.info.entry(start).or_default().end = new_end;
  }

  /// Visits statement or block. This method handles break and continue.
  fn visit_stmt_or_block(&mut self, s: &Statement<'_>) {
    self.visit_statement(s);

    // break, continue **may** make execution end
    match s {
      Statement::BreakStatement(..) | Statement::ContinueStatement(..) => {
        self.mark_as_end(stmt_start(s), End::Break)
      }
      _ => {}
    }
  }
}

impl<'a> Visit<'a> for Analyzer<'_> {
  fn visit_return_statement(&mut self, n: &ReturnStatement<'a>) {
    walk::walk_return_statement(self, n);
    self.mark_as_end(n.span.start, End::forced_return());
  }

  fn visit_throw_statement(&mut self, n: &ThrowStatement<'a>) {
    walk::walk_throw_statement(self, n);
    self.mark_as_end(n.span.start, End::forced_throw());
  }

  fn visit_break_statement(&mut self, n: &BreakStatement<'a>) {
    if let Some(label) = &n.label {
      let label = label.name.to_string();
      self.scope.found_break = Some(Some(label));
    } else {
      self.scope.found_break = Some(None);
    }
  }

  fn visit_continue_statement(&mut self, _: &ContinueStatement<'a>) {
    self.scope.found_continue = true;
  }

  fn visit_block_statement(&mut self, s: &BlockStatement<'a>) {
    walk::walk_block_statement(self, s);

    if let Some(end) = self.scope.end {
      self.mark_as_end(s.span.start, end);
    } else {
      self.mark_as_end(s.span.start, End::Continue);
    }
  }

  fn visit_statements(
    &mut self,
    stmts: &deno_ast::oxc::allocator::Vec<'a, Statement<'a>>,
  ) {
    for stmt in stmts {
      self.visit_stmt_or_block(stmt);
    }
  }

  fn visit_expression(&mut self, n: &Expression<'a>) {
    walk::walk_expression(self, n);

    if matches!(self.scope.end, None | Some(End::Continue)) {
      match n {
        Expression::Identifier(i) => {
          self.scope.used_hoistable_ids.insert(i.name.to_string());
        }
        Expression::ThisExpression(..) => {}
        _ => {
          self.scope.may_throw = true;
        }
      }
    }
  }

  fn visit_member_expression(&mut self, n: &MemberExpression<'a>) {
    match n {
      MemberExpression::StaticMemberExpression(s) => {
        self.visit_expression(&s.object);
      }
      MemberExpression::ComputedMemberExpression(c) => {
        self.visit_expression(&c.object);
        self.visit_expression(&c.expression);
      }
      MemberExpression::PrivateFieldExpression(p) => {
        self.visit_expression(&p.object);
      }
    }
  }

  fn visit_arrow_function_expression(
    &mut self,
    n: &ArrowFunctionExpression<'a>,
  ) {
    self.with_child_scope(BlockKind::Function, n.span.start, |a| {
      walk::walk_arrow_function_expression(a, n);
    })
  }

  fn visit_function(
    &mut self,
    n: &Function<'a>,
    _flags: deno_ast::oxc::syntax::scope::ScopeFlags,
  ) {
    self.with_child_scope(BlockKind::Function, n.span.start, |a| {
      walk::walk_function(
        a,
        n,
        deno_ast::oxc::syntax::scope::ScopeFlags::Function,
      );
    })
  }

  fn visit_catch_clause(&mut self, n: &CatchClause<'a>) {
    self.with_child_scope(BlockKind::Catch, n.span.start, |a| {
      walk::walk_catch_clause(a, n);
    });
  }

  fn visit_object_property(&mut self, n: &ObjectProperty<'a>) {
    match n.kind {
      PropertyKind::Get => {
        self.with_child_scope(BlockKind::Function, n.span.start, |a| {
          walk::walk_object_property(a, n);
        });
      }
      PropertyKind::Set => {
        self.with_child_scope(BlockKind::Function, n.span.start, |a| {
          walk::walk_object_property(a, n);
        });
      }
      _ => {
        walk::walk_object_property(self, n);
      }
    }
  }

  fn visit_switch_statement(&mut self, n: &SwitchStatement<'a>) {
    let prev_end = self.scope.end;
    walk::walk_switch_statement(self, n);

    let end = {
      let has_default = n.cases.iter().any(|case| case.test.is_none());
      let forced_end = n
        .cases
        .iter()
        .filter_map(|case| self.get_end_reason(case.span.start))
        .try_fold(
          End::Forced {
            ret: false,
            throw: false,
            infinite_loop: false,
          },
          |acc, cur| acc.merge_forced(cur),
        );

      match forced_end {
        Some(e) if has_default => e,
        _ => End::Continue,
      }
    };

    self.mark_as_end(n.span.start, end);

    if !matches!(end, End::Forced { .. }) {
      self.scope.end = prev_end;
    }
  }

  fn visit_switch_case(&mut self, n: &SwitchCase<'a>) {
    let prev_end = self.scope.end;
    let mut case_end = None;

    self.with_child_scope(BlockKind::Case, n.span.start, |a| {
      for stmt in &n.consequent {
        a.visit_statement(stmt);
        // After a break/continue, mark scope.end so subsequent statements are
        // flagged as unreachable, but do NOT call mark_as_end (which would store
        // End::Break in the info map — tests expect `end: None` for break stmts).
        match stmt {
          Statement::BreakStatement(..) | Statement::ContinueStatement(..) => {
            if matches!(a.scope.end, None | Some(End::Continue)) {
              a.scope.end = Some(End::Break);
            }
          }
          _ => {}
        }
      }

      if a.scope.found_break.is_some() {
        case_end = Some(End::Break);
      } else if matches!(a.scope.end, Some(End::Forced { .. })) {
        case_end = a.scope.end;
      }
    });

    if let Some(end) = case_end {
      self.mark_as_end(n.span.start, end);
    } else {
      self.mark_as_end(n.span.start, End::Continue);
    }

    self.scope.end = prev_end;
  }

  fn visit_if_statement(&mut self, n: &IfStatement<'a>) {
    self.visit_expression(&n.test);

    let prev_end = self.scope.end;

    self.with_child_scope(BlockKind::If, stmt_start(&n.consequent), |a| {
      a.visit_stmt_or_block(&n.consequent);
    });

    let cons_reason = self.get_end_reason(stmt_start(&n.consequent));

    match &n.alternate {
      Some(alt) => {
        self.with_child_scope(BlockKind::If, stmt_start(alt), |a| {
          a.visit_stmt_or_block(alt);
        });
        let alt_reason = self.get_end_reason(stmt_start(alt));

        match (cons_reason, alt_reason) {
          (Some(x), Some(y)) if x.is_forced() && y.is_forced() => {
            // This `unwrap` is safe; `x` and `y` are surely `Some(End::Forced { .. })`
            let end = x.merge_forced(y).unwrap();
            self.mark_as_end(n.span.start, end);
          }
          (Some(End::Break), Some(End::Break))
          | (Some(End::Forced { .. }), Some(End::Break))
          | (Some(End::Break), Some(End::Forced { .. })) => {
            self.mark_as_end(n.span.start, End::Break);
          }
          // TODO: Check for continue
          _ => {
            self.mark_as_end(n.span.start, End::Continue);
          }
        }
      }
      None => {
        self.mark_as_end(n.span.start, End::Continue);
        self.scope.end = prev_end;
      }
    }
  }

  fn visit_statement(&mut self, n: &Statement<'a>) {
    let scope_end = self
      .scope
      .end
      .is_some_and(|d| matches!(d, End::Forced { .. } | End::Break));

    let unreachable = if scope_end {
      // Although execution is ended, we should handle hoisting.
      match n {
        Statement::EmptyStatement(..) => false,
        Statement::FunctionDeclaration(func)
          if func.id.as_ref().is_some_and(|id| {
            self.scope.used_hoistable_ids.contains(id.name.as_str())
          }) =>
        {
          false
        }
        Statement::VariableDeclaration(decl)
          if decl.kind == VariableDeclarationKind::Var
            && decl.declarations.iter().all(|decl| decl.init.is_none()) =>
        {
          false
        }
        // It's unreachable
        _ => true,
      }
    } else {
      false
    };

    self.info.entry(stmt_start(n)).or_default().unreachable = unreachable;

    walk::walk_statement(self, n);
  }

  // loops

  fn visit_for_statement(&mut self, n: &ForStatement<'a>) {
    if let Some(init) = &n.init {
      self.visit_for_statement_init(init);
    }
    if let Some(update) = &n.update {
      self.visit_expression(update);
    }
    if let Some(test) = &n.test {
      self.visit_expression(test);
    }

    let mut forced_end = None;

    self.with_child_scope(BlockKind::Loop, stmt_start(&n.body), |a| {
      a.visit_statement(&n.body);

      let has_break = matches!(a.scope.found_break, Some(None));

      if !has_break {
        let end = match a.get_end_reason(stmt_start(&n.body)) {
          Some(e) if e.is_forced() => e,
          _ => End::forced_infinite_loop(),
        };
        match &n.test {
          None => {
            a.mark_as_end(n.span.start, end);
            forced_end = Some(end);
          }
          Some(test) => {
            if is_definitely_truthy(test) {
              a.mark_as_end(n.span.start, end);
              forced_end = Some(end);
            }
          }
        }
      }

      if forced_end.is_none() || has_break {
        a.mark_as_end(stmt_start(&n.body), End::Continue);
        a.scope.end = Some(End::Continue);
      }
    });
  }

  fn visit_for_of_statement(&mut self, n: &ForOfStatement<'a>) {
    let body_lo = stmt_start(&n.body);

    self.visit_expression(&n.right);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      a.visit_statement(&n.body);

      // it's impossible to decide whether it enters loop block unconditionally, so we always mark
      // it as `End::Continue`.
      a.mark_as_end(body_lo, End::Continue);
      a.scope.end = Some(End::Continue);
    });
  }

  fn visit_for_in_statement(&mut self, n: &ForInStatement<'a>) {
    let body_lo = stmt_start(&n.body);

    self.visit_expression(&n.right);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      a.visit_statement(&n.body);

      // it's impossible to decide whether it enters loop block unconditionally, so we always mark
      // it as `End::Continue`.
      a.mark_as_end(body_lo, End::Continue);
      a.scope.end = Some(End::Continue);
    });
  }

  fn visit_while_statement(&mut self, n: &WhileStatement<'a>) {
    let body_lo = stmt_start(&n.body);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      a.visit_statement(&n.body);

      let unconditionally_enter = is_definitely_truthy(&n.test);
      let end_reason = a.get_end_reason(body_lo);
      let return_or_throw = end_reason.is_some_and(|e| e.is_forced());
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

    self.visit_expression(&n.test);
  }

  fn visit_do_while_statement(&mut self, n: &DoWhileStatement<'a>) {
    let body_lo = stmt_start(&n.body);

    self.with_child_scope(BlockKind::Loop, body_lo, |a| {
      a.visit_statement(&n.body);

      let end_reason = a.get_end_reason(body_lo);
      let return_or_throw = end_reason.is_some_and(|e| e.is_forced());
      let infinite_loop =
        is_definitely_truthy(&n.test) && a.scope.found_break.is_none();
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
        self.mark_as_end(n.span.start, e);
      }
      _ => {}
    }

    self.visit_expression(&n.test);
  }

  fn visit_try_statement(&mut self, n: &TryStatement<'a>) {
    let old_throw = self.scope.may_throw;

    let prev_end = self.scope.end;

    self.scope.may_throw = false;
    self.visit_block_statement(&n.block);

    let try_block_end = self.scope.end;
    let try_block_may_throw = self.scope.may_throw;

    if let Some(handler) = &n.handler {
      if try_block_may_throw {
        self.scope.end = prev_end;
      }
      self.scope.may_throw = false;
      self.visit_catch_clause(handler);

      if try_block_may_throw {
        match (try_block_end, self.scope.end) {
          (
            Some(End::Forced {
              ret: false,
              throw: true,
              infinite_loop: false,
            }),
            _,
          ) => {}
          (Some(x), Some(y)) if x.is_forced() && y.is_forced() => {
            // This `unwrap` is safe; `x` and `y` are surely `Some(End::Forced { .. })`
            self.scope.end = Some(x.merge_forced(y).unwrap());
          }
          (_, Some(y)) if y.is_forced() => {
            self.scope.end = try_block_end;
          }
          (None | Some(End::Continue), Some(End::Break)) => {
            self.scope.end = try_block_end;
          }
          _ => {}
        }
      } else {
        self.scope.end = try_block_end;
      }
    }

    if let Some(finalizer) = &n.finalizer {
      let try_catch_end = self.scope.end;
      self.scope.end = prev_end;
      self.with_child_scope(BlockKind::Finally, finalizer.span.start, |a| {
        a.visit_block_statement(finalizer);
      });
      match (try_catch_end, self.scope.end) {
        (Some(x), Some(End::Break)) if x.is_forced() => {
          self.scope.end = Some(x);
        }
        (Some(x), None | Some(End::Continue)) => {
          self.scope.end = Some(x);
        }
        _ => {}
      }
    }

    if let Some(end) = self.scope.end {
      self.mark_as_end(n.span.start, end);
    }
    self.scope.may_throw |= old_throw;
  }

  fn visit_labeled_statement(&mut self, n: &LabeledStatement<'a>) {
    self.with_child_scope(
      BlockKind::Label(n.label.name.to_string()),
      n.span.start,
      |a| {
        a.visit_stmt_or_block(&n.body);
      },
    );
  }
}
