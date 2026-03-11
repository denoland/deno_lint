// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  ArrowFunctionExpression, AssignmentExpression, AssignmentTarget,
  AssignmentTargetMaybeDefault, AssignmentTargetProperty,
  BindingPattern, CatchClause, Class,
  DoWhileStatement, Expression, ExpressionStatement, ForInStatement,
  ForOfStatement, ForStatement, ForStatementInit, ForStatementLeft,
  Function, IfStatement, MethodDefinition,
  Program, SimpleAssignmentTarget, Statement, SwitchStatement,
  UpdateExpression, VariableDeclaration, VariableDeclarationKind,
  WhileStatement, WithStatement,
};
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::span::{CompactStr, GetSpan, Span};
use deno_ast::oxc::syntax::scope::ScopeFlags;
use derive_more::Display;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::mem;
use std::rc::Rc;

#[derive(Debug)]
pub struct PreferConst;

const CODE: &str = "prefer-const";

#[derive(Display)]
enum PreferConstMessage {
  #[display(fmt = "`{}` is never reassigned", _0)]
  NeverReassigned(String),
}

#[derive(Display)]
enum PreferConstHint {
  #[display(fmt = "Use `const` instead")]
  UseConst,
}

impl LintRule for PreferConst {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut collector = VariableCollector::new();
    collector.visit_program(program);

    let mut visitor = PreferConstVisitor::new(
      context,
      mem::take(&mut collector.scopes),
      mem::take(&mut collector.var_groups),
    );
    visitor.visit_program(program);
  }
}

type Scope = Rc<RefCell<RawScope>>;

#[derive(Debug)]
struct RawScope {
  parent: Option<Scope>,
  variables: BTreeMap<CompactStr, Span>,
}

impl RawScope {
  fn new(parent: Option<Scope>) -> Self {
    Self {
      parent,
      variables: BTreeMap::new(),
    }
  }
}

#[derive(Debug)]
struct DeclInfo {
  /// the range of its declaration
  range: Span,
  /// `true` if this is declared in the other scope
  in_other_scope: bool,
}

#[derive(Debug)]
enum ScopeAnalysisError {
  ScopeNotFound,
}

/// Looks for the declaration range of the given variable by traversing from the given scope to the parents.
fn get_decl_by_ident(
  scope: Scope,
  name: &str,
) -> Option<DeclInfo> {
  let mut cur_scope = Some(scope);
  let mut is_current_scope = true;
  while let Some(cur) = cur_scope {
    if let Some(&range) = cur.borrow().variables.get(name) {
      return Some(DeclInfo {
        range,
        in_other_scope: !is_current_scope,
      });
    }
    cur_scope = cur.borrow().parent.as_ref().cloned();
    is_current_scope = false;
  }
  None
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
enum VarStatus {
  Declared,
  Initialized,
  Reassigned,
}

impl VarStatus {
  fn next(&mut self) {
    use VarStatus::*;
    *self = match *self {
      Declared => Initialized,
      Initialized => Reassigned,
      Reassigned => Reassigned,
    }
  }

  fn force_reassigned(&mut self) {
    *self = VarStatus::Reassigned;
  }
}

#[derive(Default, Debug)]
struct DisjointSet {
  parents: BTreeMap<Span, Span>,
  roots: HashMap<Span, (VarStatus, usize)>,
}

impl DisjointSet {
  fn new() -> Self {
    Self::default()
  }

  fn proceed_status(&mut self, range: Span, force_reassigned: bool) {
    let root = self.get_root(range).unwrap();
    let (ref mut st, _) = self.roots.get_mut(&root).unwrap();
    if force_reassigned {
      st.force_reassigned();
    } else {
      st.next();
    }
  }

  fn add_root(&mut self, range: Span, status: VarStatus) {
    if self.parents.contains_key(&range) {
      return;
    }
    self.parents.insert(range, range);
    self.roots.insert(range, (status, 1));
  }

  fn get_root(&mut self, range: Span) -> Option<Span> {
    match self.parents.get(&range) {
      None => None,
      Some(&par_range) if range == par_range => Some(range),
      Some(&par_range) => {
        let root = self.get_root(par_range);
        if let (Some(root_range), Some(par)) =
          (root, self.parents.get_mut(&range))
        {
          *par = root_range;
        }
        root
      }
    }
  }

  fn unite(&mut self, range1: Span, range2: Span) -> Option<()> {
    let rs1 = self.get_root(range1)?;
    let rs2 = self.get_root(range2)?;
    if rs1 == rs2 {
      return None;
    }

    let &(status1, rank1) = self.roots.get(&rs1)?;
    let &(status2, rank2) = self.roots.get(&rs2)?;

    let next_status = std::cmp::max(status1, status2);

    if rank1 <= rank2 {
      let p = self.parents.get_mut(&rs1)?;
      *p = rs2;
      let r = self.roots.get_mut(&rs2)?;
      *r = (next_status, std::cmp::max(rank1 + 1, rank2));
      self.roots.remove(&rs1);
    } else {
      let p = self.parents.get_mut(&rs2)?;
      *p = rs1;
      let r = self.roots.get_mut(&rs1)?;
      *r = (next_status, rank1);
      self.roots.remove(&rs2);
    }

    Some(())
  }

  fn dump(&mut self) -> Vec<Span> {
    self
      .parents
      .clone()
      .keys()
      .filter_map(|&cur| {
        let root = self.get_root(cur)?;
        if matches!(self.roots.get(&root), Some((VarStatus::Initialized, _))) {
          Some(cur)
        } else {
          None
        }
      })
      .collect()
  }
}

#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
enum ScopeRange {
  Global,
  Block(Span),
}

#[derive(Debug)]
struct VariableCollector {
  scopes: BTreeMap<ScopeRange, Scope>,
  cur_scope: ScopeRange,
  var_groups: DisjointSet,
}

/// Info about an identifier extracted from a binding pattern
struct IdentInfo {
  name: CompactStr,
  span: Span,
}

/// Extracts identifier infos from a BindingPattern recursively
fn extract_idents_from_binding<'a>(
  idents: &mut Vec<IdentInfo>,
  pattern: &BindingPattern<'a>,
) {
  match pattern {
    BindingPattern::BindingIdentifier(ident) => {
      idents.push(IdentInfo {
        name: ident.name.to_compact_str(),
        span: ident.span,
      });
    }
    BindingPattern::ObjectPattern(obj) => {
      for prop in &obj.properties {
        extract_idents_from_binding(idents, &prop.value);
      }
      if let Some(rest) = &obj.rest {
        extract_idents_from_binding(idents, &rest.argument);
      }
    }
    BindingPattern::ArrayPattern(arr) => {
      for elem in arr.elements.iter().flatten() {
        extract_idents_from_binding(idents, elem);
      }
      if let Some(rest) = &arr.rest {
        extract_idents_from_binding(idents, &rest.argument);
      }
    }
    BindingPattern::AssignmentPattern(assign) => {
      extract_idents_from_binding(idents, &assign.left);
    }
  }
}

/// Info extracted from an AssignmentTarget
enum AssignTargetIdentInfo {
  Ident { name: CompactStr, span: Span },
  MemberExpr,
}

/// Extracts identifiers from an AssignmentTargetMaybeDefault
fn extract_idents_from_assign_target_maybe_default(
  idents: &mut Vec<AssignTargetIdentInfo>,
  target: &AssignmentTargetMaybeDefault,
) {
  match target {
    AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(
      with_default,
    ) => {
      extract_idents_from_assign_target(idents, &with_default.binding);
    }
    _ => {
      if let Some(target) = target.as_assignment_target() {
        extract_idents_from_assign_target(idents, target);
      }
    }
  }
}

/// Extracts identifiers from an AssignmentTarget recursively
fn extract_idents_from_assign_target(
  idents: &mut Vec<AssignTargetIdentInfo>,
  target: &AssignmentTarget,
) {
  match target {
    AssignmentTarget::AssignmentTargetIdentifier(ident) => {
      idents.push(AssignTargetIdentInfo::Ident {
        name: ident.name.to_compact_str(),
        span: ident.span,
      });
    }
    AssignmentTarget::ObjectAssignmentTarget(obj) => {
      for prop in &obj.properties {
        match prop {
          AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
            ident_prop,
          ) => {
            idents.push(AssignTargetIdentInfo::Ident {
              name: ident_prop.binding.name.to_compact_str(),
              span: ident_prop.binding.span,
            });
          }
          AssignmentTargetProperty::AssignmentTargetPropertyProperty(
            kv_prop,
          ) => {
            extract_idents_from_assign_target_maybe_default(
              idents,
              &kv_prop.binding,
            );
          }
        }
      }
      if let Some(rest) = &obj.rest {
        extract_idents_from_assign_target(idents, &rest.target);
      }
    }
    AssignmentTarget::ArrayAssignmentTarget(arr) => {
      for elem in arr.elements.iter().flatten() {
        extract_idents_from_assign_target_maybe_default(idents, elem);
      }
      if let Some(rest) = &arr.rest {
        extract_idents_from_assign_target(idents, &rest.target);
      }
    }
    // Member expressions and other targets
    _ => {
      idents.push(AssignTargetIdentInfo::MemberExpr);
    }
  }
}

impl VariableCollector {
  fn new() -> Self {
    Self {
      scopes: BTreeMap::new(),
      cur_scope: ScopeRange::Global,
      var_groups: DisjointSet::new(),
    }
  }

  fn insert_var(&mut self, info: &IdentInfo, status: VarStatus) {
    self.var_groups.add_root(info.span, status);
    let mut scope = self.scopes.get(&self.cur_scope).unwrap().borrow_mut();
    scope.variables.insert(info.name.clone(), info.span);
  }

  fn insert_vars(&mut self, idents: &[IdentInfo], status: VarStatus) {
    match idents {
      [] => {}
      [ident] => {
        self.insert_var(ident, status);
      }
      [first, others @ ..] => {
        self.insert_var(first, status);
        for i in others {
          self.insert_var(i, status);
          self.var_groups.unite(first.span, i.span);
        }
      }
    }
  }

  fn extract_decl_idents(
    &mut self,
    pattern: &BindingPattern,
    has_init: bool,
  ) {
    let status = if has_init {
      VarStatus::Initialized
    } else {
      VarStatus::Declared
    };
    let mut idents = Vec::new();
    extract_idents_from_binding(&mut idents, pattern);
    self.insert_vars(&idents, status);
  }

  fn with_child_scope<F>(&mut self, span: Span, op: F)
  where
    F: FnOnce(&mut VariableCollector),
  {
    let parent_scope_range = self.cur_scope;
    let parent_scope = self.scopes.get(&parent_scope_range).cloned();
    let child_scope = RawScope::new(parent_scope);
    self.scopes.insert(
      ScopeRange::Block(span),
      Rc::new(RefCell::new(child_scope)),
    );
    self.cur_scope = ScopeRange::Block(span);
    op(self);
    self.cur_scope = parent_scope_range;
  }

  fn insert_params_as_reassigned(&mut self, params: &[IdentInfo]) {
    for ident in params {
      self.insert_var(ident, VarStatus::Reassigned);
    }
  }
}

impl<'a> Visit<'a> for VariableCollector {
  fn visit_program(&mut self, program: &Program<'a>) {
    let scope = RawScope::new(None);
    self
      .scopes
      .insert(ScopeRange::Global, Rc::new(RefCell::new(scope)));
    walk::walk_program(self, program);
  }

  fn visit_function(
    &mut self,
    function: &Function<'a>,
    _flags: ScopeFlags,
  ) {
    self.with_child_scope(function.span, |a| {
      for param in &function.params.items {
        walk::walk_formal_parameter(a, param);
        let mut idents = Vec::new();
        extract_idents_from_binding(&mut idents, &param.pattern);
        a.insert_params_as_reassigned(&idents);
      }
      if let Some(body) = &function.body {
        walk::walk_function_body(a, body);
      }
    });
  }

  fn visit_arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression<'a>,
  ) {
    self.with_child_scope(arrow.span, |a| {
      for param in &arrow.params.items {
        walk::walk_formal_parameter(a, param);
        let mut idents = Vec::new();
        extract_idents_from_binding(&mut idents, &param.pattern);
        a.insert_params_as_reassigned(&idents);
      }
      walk::walk_function_body(a, &arrow.body);
    });
  }

  fn visit_block_statement(
    &mut self,
    block: &deno_ast::oxc::ast::ast::BlockStatement<'a>,
  ) {
    self.with_child_scope(block.span, |a| {
      walk::walk_block_statement(a, block);
    });
  }

  fn visit_for_statement(&mut self, for_stmt: &ForStatement<'a>) {
    self.with_child_scope(for_stmt.span, |a| {
      match &for_stmt.init {
        Some(ForStatementInit::VariableDeclaration(var_decl)) => {
          walk::walk_variable_declaration(a, var_decl);
          if var_decl.kind == VariableDeclarationKind::Let {
            let mut idents = Vec::new();
            let mut has_init = false;
            for decl in &var_decl.declarations {
              extract_idents_from_binding(&mut idents, &decl.id);
              has_init |= decl.init.is_some();
            }
            let status = if has_init {
              VarStatus::Initialized
            } else {
              VarStatus::Declared
            };
            a.insert_vars(&idents, status);
          }
        }
        Some(init) => {
          walk::walk_for_statement_init(a, init);
        }
        None => {}
      }

      if let Some(test_expr) = &for_stmt.test {
        walk::walk_expression(a, test_expr);
      }
      if let Some(update_expr) = &for_stmt.update {
        walk::walk_expression(a, update_expr);
      }

      if let Statement::BlockStatement(block_stmt) = &for_stmt.body {
        walk::walk_block_statement(a, block_stmt);
      } else {
        walk::walk_statement(a, &for_stmt.body);
      }
    });
  }

  fn visit_for_of_statement(&mut self, for_of_stmt: &ForOfStatement<'a>) {
    self.with_child_scope(for_of_stmt.span, |a| {
      if let ForStatementLeft::VariableDeclaration(var_decl) =
        &for_of_stmt.left
      {
        if var_decl.kind == VariableDeclarationKind::Let {
          for decl in &var_decl.declarations {
            a.extract_decl_idents(&decl.id, true);
          }
        }
      }

      walk::walk_expression(a, &for_of_stmt.right);

      if let Statement::BlockStatement(block_stmt) = &for_of_stmt.body {
        walk::walk_block_statement(a, block_stmt);
      } else {
        walk::walk_statement(a, &for_of_stmt.body);
      }
    });
  }

  fn visit_for_in_statement(&mut self, for_in_stmt: &ForInStatement<'a>) {
    self.with_child_scope(for_in_stmt.span, |a| {
      if let ForStatementLeft::VariableDeclaration(var_decl) =
        &for_in_stmt.left
      {
        if var_decl.kind == VariableDeclarationKind::Let {
          for decl in &var_decl.declarations {
            a.extract_decl_idents(&decl.id, true);
          }
        }
      }

      walk::walk_expression(a, &for_in_stmt.right);

      if let Statement::BlockStatement(block_stmt) = &for_in_stmt.body {
        walk::walk_block_statement(a, block_stmt);
      } else {
        walk::walk_statement(a, &for_in_stmt.body);
      }
    });
  }

  fn visit_if_statement(&mut self, if_stmt: &IfStatement<'a>) {
    self.with_child_scope(if_stmt.span, |a| {
      walk::walk_expression(a, &if_stmt.test);
      if let Statement::BlockStatement(body) = &if_stmt.consequent {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &if_stmt.consequent);
      }
    });

    if let Some(alt) = &if_stmt.alternate {
      self.with_child_scope(alt.span(), |a| {
        walk::walk_statement(a, alt);
      });
    }
  }

  fn visit_switch_statement(&mut self, switch_stmt: &SwitchStatement<'a>) {
    self.with_child_scope(switch_stmt.span, |a| {
      walk::walk_expression(a, &switch_stmt.discriminant);
      for case in &switch_stmt.cases {
        walk::walk_switch_case(a, case);
      }
    });
  }

  fn visit_while_statement(&mut self, while_stmt: &WhileStatement<'a>) {
    self.with_child_scope(while_stmt.span, |a| {
      walk::walk_expression(a, &while_stmt.test);
      if let Statement::BlockStatement(body) = &while_stmt.body {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &while_stmt.body);
      }
    });
  }

  fn visit_do_while_statement(
    &mut self,
    do_while_stmt: &DoWhileStatement<'a>,
  ) {
    self.with_child_scope(do_while_stmt.span, |a| {
      if let Statement::BlockStatement(body) = &do_while_stmt.body {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &do_while_stmt.body);
      }
      walk::walk_expression(a, &do_while_stmt.test);
    });
  }

  fn visit_with_statement(&mut self, with_stmt: &WithStatement<'a>) {
    self.with_child_scope(with_stmt.span, |a| {
      walk::walk_expression(a, &with_stmt.object);
      if let Statement::BlockStatement(body) = &with_stmt.body {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &with_stmt.body);
      }
    });
  }

  fn visit_catch_clause(&mut self, catch_clause: &CatchClause<'a>) {
    self.with_child_scope(catch_clause.span, |a| {
      if let Some(param) = &catch_clause.param {
        let mut idents = Vec::new();
        extract_idents_from_binding(&mut idents, &param.pattern);
        a.insert_params_as_reassigned(&idents);
      }
      walk::walk_block_statement(a, &catch_clause.body);
    });
  }

  fn visit_class(&mut self, class: &Class<'a>) {
    for decorator in &class.decorators {
      walk::walk_decorator(self, decorator);
    }
    if let Some(super_class) = &class.super_class {
      walk::walk_expression(self, super_class);
    }
    self.with_child_scope(class.span, |a| {
      for member in &class.body.body {
        walk::walk_class_element(a, member);
      }
    });
  }

  fn visit_method_definition(
    &mut self,
    method: &MethodDefinition<'a>,
  ) {
    // Handle constructors specially - they have their own scope
    if method.kind == deno_ast::oxc::ast::ast::MethodDefinitionKind::Constructor
    {
      self.with_child_scope(method.span, |a| {
        // Constructor params: handle TS parameter properties
        for param in &method.value.params.items {
          walk::walk_formal_parameter(a, param);
          let mut idents = Vec::new();
          extract_idents_from_binding(&mut idents, &param.pattern);
          a.insert_params_as_reassigned(&idents);
        }
        if let Some(body) = &method.value.body {
          walk::walk_function_body(a, body);
        }
      });
    } else {
      // Regular methods use visit_function
      walk::walk_method_definition(self, method);
    }
  }

  fn visit_variable_declaration(
    &mut self,
    var_decl: &VariableDeclaration<'a>,
  ) {
    walk::walk_variable_declaration(self, var_decl);
    if var_decl.kind == VariableDeclarationKind::Let {
      for decl in &var_decl.declarations {
        self.extract_decl_idents(&decl.id, decl.init.is_some());
      }
    }
  }
}

struct PreferConstVisitor<'c, 'view> {
  scopes: BTreeMap<ScopeRange, Scope>,
  cur_scope: ScopeRange,
  var_groups: DisjointSet,
  context: &'c mut Context<'view>,
  scope_analysis_error_occurred: bool,
}

impl<'c, 'view> PreferConstVisitor<'c, 'view> {
  fn new(
    context: &'c mut Context<'view>,
    scopes: BTreeMap<ScopeRange, Scope>,
    var_groups: DisjointSet,
  ) -> Self {
    Self {
      context,
      scopes,
      var_groups,
      cur_scope: ScopeRange::Global,
      scope_analysis_error_occurred: false,
    }
  }

  fn report(&mut self, range: Span) {
    let range_text =
      self.context.source_text()[range.start as usize..range.end as usize]
        .to_string();
    self.context.add_diagnostic_with_hint(
      range,
      CODE,
      PreferConstMessage::NeverReassigned(range_text),
      PreferConstHint::UseConst,
    );
  }

  fn with_child_scope<F>(&mut self, span: Span, op: F)
  where
    F: FnOnce(&mut Self),
  {
    let parent_scope_range = self.cur_scope;
    self.cur_scope = ScopeRange::Block(span);
    op(self);
    self.cur_scope = parent_scope_range;
  }

  fn get_scope(&self) -> Option<Scope> {
    self.scopes.get(&self.cur_scope).cloned()
  }

  fn extract_assign_idents(
    &mut self,
    target: &AssignmentTarget,
  ) -> Result<(), ScopeAnalysisError> {
    let mut infos = Vec::new();
    extract_idents_from_assign_target(&mut infos, target);

    let mut idents = Vec::new();
    let mut contains_member_access = false;
    for info in &infos {
      match info {
        AssignTargetIdentInfo::Ident { name, span } => {
          idents.push((name.as_str(), *span));
        }
        AssignTargetIdentInfo::MemberExpr => {
          contains_member_access = true;
        }
      }
    }
    self.process_var_status_by_names(
      idents.into_iter(),
      contains_member_access,
    )?;
    Ok(())
  }

  fn process_var_status_by_names<'b>(
    &mut self,
    idents: impl Iterator<Item = (&'b str, Span)>,
    force_reassigned: bool,
  ) -> Result<(), ScopeAnalysisError> {
    let Some(scope) = self.get_scope() else {
      return Err(ScopeAnalysisError::ScopeNotFound);
    };
    let decls: Vec<DeclInfo> = idents
      .filter_map(|(name, _span)| {
        get_decl_by_ident(Rc::clone(&scope), name)
      })
      .collect();

    match decls.as_slice() {
      [] => {}
      [decl] => {
        self
          .var_groups
          .proceed_status(decl.range, force_reassigned || decl.in_other_scope);
      }
      [first, others @ ..] => {
        self.var_groups.proceed_status(
          first.range,
          force_reassigned || first.in_other_scope,
        );
        for s in others {
          self
            .var_groups
            .proceed_status(s.range, force_reassigned || s.in_other_scope);
          self.var_groups.unite(first.range, s.range);
        }
      }
    }

    Ok(())
  }

  fn process_var_status_single(
    &mut self,
    name: &str,
    force_reassigned: bool,
  ) -> Result<(), ScopeAnalysisError> {
    let Some(scope) = self.get_scope() else {
      return Err(ScopeAnalysisError::ScopeNotFound);
    };
    if let Some(decl) = get_decl_by_ident(scope, name) {
      self
        .var_groups
        .proceed_status(decl.range, force_reassigned || decl.in_other_scope);
    }
    Ok(())
  }
}

impl<'a> Visit<'a> for PreferConstVisitor<'_, '_> {
  fn visit_program(&mut self, program: &Program<'a>) {
    walk::walk_program(self, program);
    // After visiting all nodes, reports errors.
    for range in self.var_groups.dump() {
      self.report(range);
    }
  }

  fn visit_assignment_expression(
    &mut self,
    assign_expr: &AssignmentExpression<'a>,
  ) {
    // This only handles _nested_ `AssignmentExpression` since not nested `AssignExpression` (i.e. the direct child of
    // `ExpressionStatement`) is already handled by `visit_expression_statement`.
    walk::walk_assignment_expression(self, assign_expr);

    match &assign_expr.left {
      AssignmentTarget::AssignmentTargetIdentifier(ident) => {
        if self
          .process_var_status_single(ident.name.as_str(), true)
          .is_err()
        {
          self.scope_analysis_error_occurred = true;
        }
      }
      target => {
        let mut infos = Vec::new();
        extract_idents_from_assign_target(&mut infos, target);
        let idents: Vec<_> = infos
          .iter()
          .filter_map(|info| match info {
            AssignTargetIdentInfo::Ident { name, span } => {
              Some((name.as_str(), *span))
            }
            _ => None,
          })
          .collect();
        if self
          .process_var_status_by_names(idents.into_iter(), true)
          .is_err()
        {
          self.scope_analysis_error_occurred = true;
        }
      }
    }
  }

  fn visit_expression_statement(
    &mut self,
    expr_stmt: &ExpressionStatement<'a>,
  ) {
    let mut expr = &expr_stmt.expression;

    // Unwrap parentheses
    while let Expression::ParenthesizedExpression(e) = expr {
      expr = &e.expression;
    }

    match expr {
      Expression::AssignmentExpression(assign_expr) => {
        match &assign_expr.left {
          AssignmentTarget::AssignmentTargetIdentifier(ident) => {
            if self
              .process_var_status_single(ident.name.as_str(), false)
              .is_err()
            {
              self.scope_analysis_error_occurred = true;
            }
          }
          target => {
            if self.extract_assign_idents(target).is_err() {
              self.scope_analysis_error_occurred = true;
            }
          }
        }
        walk::walk_assignment_expression(self, assign_expr);
      }
      _ => walk::walk_expression_statement(self, expr_stmt),
    }
  }

  fn visit_update_expression(
    &mut self,
    update_expr: &UpdateExpression<'a>,
  ) {
    if let SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) =
      &update_expr.argument
    {
      if self
        .process_var_status_single(ident.name.as_str(), false)
        .is_err()
      {
        self.scope_analysis_error_occurred = true;
      }
    }
    // For member expressions etc., no need to walk further - they don't affect variable status
  }

  fn visit_function(
    &mut self,
    function: &Function<'a>,
    _flags: ScopeFlags,
  ) {
    for param in &function.params.items {
      if let BindingPattern::AssignmentPattern(assign_pat) = &param.pattern {
        self.visit_assignment_pattern(assign_pat);
      }
      // In OXC, non-destructuring params with defaults use FormalParameter.initializer
      // (e.g. `function f(x = expr)` — `expr` is in param.initializer, not AssignmentPattern).
      if let Some(initializer) = &param.initializer {
        walk::walk_expression(self, initializer);
      }
    }
    self.with_child_scope(function.span, |a| {
      if let Some(body) = &function.body {
        walk::walk_function_body(a, body);
      }
    });
  }

  fn visit_arrow_function_expression(
    &mut self,
    arrow: &ArrowFunctionExpression<'a>,
  ) {
    for param in &arrow.params.items {
      if let BindingPattern::AssignmentPattern(assign_pat) = &param.pattern {
        self.visit_assignment_pattern(assign_pat);
      }
      // In OXC, non-destructuring params with defaults use FormalParameter.initializer.
      if let Some(initializer) = &param.initializer {
        walk::walk_expression(self, initializer);
      }
    }
    self.with_child_scope(arrow.span, |a| {
      walk::walk_function_body(a, &arrow.body);
    });
  }

  fn visit_block_statement(
    &mut self,
    block: &deno_ast::oxc::ast::ast::BlockStatement<'a>,
  ) {
    self.with_child_scope(block.span, |a| {
      walk::walk_block_statement(a, block);
    });
  }

  fn visit_for_statement(&mut self, for_stmt: &ForStatement<'a>) {
    self.with_child_scope(for_stmt.span, |a| {
      if let Some(init) = &for_stmt.init {
        walk::walk_for_statement_init(a, init);
      }
      if let Some(test) = &for_stmt.test {
        walk::walk_expression(a, test);
      }
      if let Some(update) = &for_stmt.update {
        walk::walk_expression(a, update);
      }

      if let Statement::BlockStatement(block_stmt) = &for_stmt.body {
        walk::walk_block_statement(a, block_stmt);
      } else {
        walk::walk_statement(a, &for_stmt.body);
      }
    });
  }

  fn visit_for_of_statement(&mut self, for_of_stmt: &ForOfStatement<'a>) {
    self.with_child_scope(for_of_stmt.span, |a| {
      match &for_of_stmt.left {
        ForStatementLeft::VariableDeclaration(var_decl) => {
          a.visit_variable_declaration(var_decl);
        }
        ForStatementLeft::AssignmentTargetIdentifier(ident) => {
          if a
            .process_var_status_single(ident.name.as_str(), false)
            .is_err()
          {
            a.scope_analysis_error_occurred = true;
          }
        }
        left => {
          let mut infos = Vec::new();
          extract_idents_from_for_left(&mut infos, left);
          let mut ident_pairs = Vec::new();
          let mut contains_member = false;
          for info in &infos {
            match info {
              AssignTargetIdentInfo::Ident { name, span } => {
                ident_pairs.push((name.as_str(), *span));
              }
              AssignTargetIdentInfo::MemberExpr => {
                contains_member = true;
              }
            }
          }
          if a
            .process_var_status_by_names(
              ident_pairs.into_iter(),
              contains_member,
            )
            .is_err()
          {
            a.scope_analysis_error_occurred = true;
          }
        }
      }

      walk::walk_expression(a, &for_of_stmt.right);

      if let Statement::BlockStatement(block_stmt) = &for_of_stmt.body {
        walk::walk_block_statement(a, block_stmt);
      } else {
        walk::walk_statement(a, &for_of_stmt.body);
      }
    });
  }

  fn visit_for_in_statement(&mut self, for_in_stmt: &ForInStatement<'a>) {
    self.with_child_scope(for_in_stmt.span, |a| {
      match &for_in_stmt.left {
        ForStatementLeft::VariableDeclaration(var_decl) => {
          a.visit_variable_declaration(var_decl);
        }
        ForStatementLeft::AssignmentTargetIdentifier(ident) => {
          if a
            .process_var_status_single(ident.name.as_str(), false)
            .is_err()
          {
            a.scope_analysis_error_occurred = true;
          }
        }
        left => {
          let mut infos = Vec::new();
          extract_idents_from_for_left(&mut infos, left);
          let mut ident_pairs = Vec::new();
          let mut contains_member = false;
          for info in &infos {
            match info {
              AssignTargetIdentInfo::Ident { name, span } => {
                ident_pairs.push((name.as_str(), *span));
              }
              AssignTargetIdentInfo::MemberExpr => {
                contains_member = true;
              }
            }
          }
          if a
            .process_var_status_by_names(
              ident_pairs.into_iter(),
              contains_member,
            )
            .is_err()
          {
            a.scope_analysis_error_occurred = true;
          }
        }
      }

      walk::walk_expression(a, &for_in_stmt.right);

      if let Statement::BlockStatement(block_stmt) = &for_in_stmt.body {
        walk::walk_block_statement(a, block_stmt);
      } else {
        walk::walk_statement(a, &for_in_stmt.body);
      }
    });
  }

  fn visit_if_statement(&mut self, if_stmt: &IfStatement<'a>) {
    self.with_child_scope(if_stmt.span, |a| {
      walk::walk_expression(a, &if_stmt.test);
      if let Statement::BlockStatement(body) = &if_stmt.consequent {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &if_stmt.consequent);
      }
    });

    if let Some(alt) = &if_stmt.alternate {
      self.with_child_scope(alt.span(), |a| {
        walk::walk_statement(a, alt);
      });
    }
  }

  fn visit_switch_statement(&mut self, switch_stmt: &SwitchStatement<'a>) {
    self.with_child_scope(switch_stmt.span, |a| {
      walk::walk_expression(a, &switch_stmt.discriminant);
      for case in &switch_stmt.cases {
        walk::walk_switch_case(a, case);
      }
    });
  }

  fn visit_while_statement(&mut self, while_stmt: &WhileStatement<'a>) {
    self.with_child_scope(while_stmt.span, |a| {
      walk::walk_expression(a, &while_stmt.test);
      if let Statement::BlockStatement(body) = &while_stmt.body {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &while_stmt.body);
      }
    });
  }

  fn visit_do_while_statement(
    &mut self,
    do_while_stmt: &DoWhileStatement<'a>,
  ) {
    self.with_child_scope(do_while_stmt.span, |a| {
      if let Statement::BlockStatement(body) = &do_while_stmt.body {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &do_while_stmt.body);
      }
      walk::walk_expression(a, &do_while_stmt.test);
    });
  }

  fn visit_with_statement(&mut self, with_stmt: &WithStatement<'a>) {
    self.with_child_scope(with_stmt.span, |a| {
      walk::walk_expression(a, &with_stmt.object);
      if let Statement::BlockStatement(body) = &with_stmt.body {
        walk::walk_block_statement(a, body);
      } else {
        walk::walk_statement(a, &with_stmt.body);
      }
    });
  }

  fn visit_catch_clause(&mut self, catch_clause: &CatchClause<'a>) {
    self.with_child_scope(catch_clause.span, |a| {
      if let Some(param) = &catch_clause.param {
        walk::walk_catch_parameter(a, param);
      }
      walk::walk_block_statement(a, &catch_clause.body);
    });
  }

  fn visit_class(&mut self, class: &Class<'a>) {
    for decorator in &class.decorators {
      walk::walk_decorator(self, decorator);
    }
    if let Some(super_class) = &class.super_class {
      walk::walk_expression(self, super_class);
    }
    self.with_child_scope(class.span, |a| {
      for member in &class.body.body {
        walk::walk_class_element(a, member);
      }
    });
  }

  fn visit_method_definition(
    &mut self,
    method: &MethodDefinition<'a>,
  ) {
    if method.kind == deno_ast::oxc::ast::ast::MethodDefinitionKind::Constructor
    {
      self.with_child_scope(method.span, |a| {
        for param in &method.value.params.items {
          walk::walk_formal_parameter(a, param);
        }
        if let Some(body) = &method.value.body {
          walk::walk_function_body(a, body);
        }
      });
    } else {
      walk::walk_method_definition(self, method);
    }
  }
}

/// Extract ident info from a ForStatementLeft for assignment tracking
fn extract_idents_from_for_left(
  idents: &mut Vec<AssignTargetIdentInfo>,
  left: &ForStatementLeft,
) {
  match left {
    ForStatementLeft::VariableDeclaration(_)
    | ForStatementLeft::AssignmentTargetIdentifier(_) => {
      // Handled separately
    }
    ForStatementLeft::ObjectAssignmentTarget(obj) => {
      // Build a temporary AssignmentTarget and extract
      for prop in &obj.properties {
        match prop {
          AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
            ident_prop,
          ) => {
            idents.push(AssignTargetIdentInfo::Ident {
              name: ident_prop.binding.name.to_compact_str(),
              span: ident_prop.binding.span,
            });
          }
          AssignmentTargetProperty::AssignmentTargetPropertyProperty(
            kv_prop,
          ) => {
            extract_idents_from_assign_target_maybe_default(
              idents,
              &kv_prop.binding,
            );
          }
        }
      }
      if let Some(rest) = &obj.rest {
        extract_idents_from_assign_target(idents, &rest.target);
      }
    }
    ForStatementLeft::ArrayAssignmentTarget(arr) => {
      for elem in arr.elements.iter().flatten() {
        extract_idents_from_assign_target_maybe_default(idents, elem);
      }
      if let Some(rest) = &arr.rest {
        extract_idents_from_assign_target(idents, &rest.target);
      }
    }
    // Member expressions, TS type assertions, etc.
    _ => {
      idents.push(AssignTargetIdentInfo::MemberExpr);
    }
  }
}

#[cfg(test)]
mod variable_collector_tests {
  use super::*;
  use crate::test_util;

  fn collect(src: &str) -> VariableCollector {
    let mut v = VariableCollector::new();
    test_util::parse_and_then(src, |parsed| {
      v.visit_program(parsed.program());
    });
    v
  }

  fn variables(scope: &Scope) -> Vec<String> {
    scope
      .borrow()
      .variables
      .keys()
      .map(|k| k.to_string())
      .collect()
  }

  #[test]
  fn collector_works_function() {
    let src = r#"
let global1;
function foo({ param1, key: param2 }) {
  let inner1 = 2;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let foo_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "param1", "param2"], foo_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_arrow_function() {
    let src = r#"
let global1;
const arrow = (param1, ...param2) => {
  let inner1;
};
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let foo_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "param1"], foo_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_block() {
    let src = r#"
let global1;
{
  let inner1 = 1;
  let inner2;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let inner_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "inner2"], inner_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_1() {
    let src = r#"
let global1;
if (true) {
  let inner1 = 1;
  let inner2;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let inner_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner1", "inner2"], inner_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_2() {
    let src = r#"
let global1;
if (true) {
  let cons = 1;
} else {
  let alt = 3;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let cons_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons"], cons_vars);

    let alt_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["alt"], alt_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_3() {
    let src = r#"
let global1;
if (true) {
  let cons1 = 1;
} else if (false) {
  let cons2 = 2;
} else {
  let alt;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let cons1_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons1"], cons1_vars);

    let cons2_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons2"], cons2_vars);

    let alt_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["alt"], alt_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_if_4() {
    let src = r#"
let global1;
if (true) foo();
else if (false) bar();
else baz();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let cons1_vars = variables(scope_iter.next().unwrap());
    assert!(cons1_vars.is_empty());

    let cons2_vars = variables(scope_iter.next().unwrap());
    assert!(cons2_vars.is_empty());

    let alt_vars = variables(scope_iter.next().unwrap());
    assert!(alt_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }
  #[test]
  fn collector_works_switch_1() {
    let src = r#"
let global1;
switch (foo) {
  case 0:
    global1 = 0;
}
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1"], global_vars);

    let switch_vars = variables(scope_iter.next().unwrap());
    assert!(switch_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_switch_2() {
    let src = r#"
let global1;
switch (foo) {
  case 0:
    global1 = 0;
  case 1: {
    let case1;
  }
  default:
    console.log("default");
}
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1"], global_vars);

    let switch_vars = variables(scope_iter.next().unwrap());
    assert!(switch_vars.is_empty());

    let case1_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["case1"], case1_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_switch_3() {
    let src = r#"
let global1;
switch (foo) {
  case 0: {
    let case0;
  }
  case 1: {
    let case1;
  }
  default: {
    let case2 = 42;
    case2++;
    console.log(case2);
  }
}
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1"], global_vars);

    let switch_vars = variables(scope_iter.next().unwrap());
    assert!(switch_vars.is_empty());

    let case0_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["case0"], case0_vars);

    let case1_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["case1"], case1_vars);

    let case2_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["case2"], case2_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_1() {
    let src = r#"
let global1;
for (let i = 0, j = 10; i < 10; i++) {
  let inner;
  j--;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "inner", "j"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_2() {
    let src = r#"
let global1;
for (let i = 0, j = 10; i < 10; i++) i += 2;
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "j"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_3() {
    let src = r#"
let global1 = 0;
for (global1 = 0; global1 < 10; ++global1) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert!(for_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_of_1() {
    let src = r#"
let global1;
for (let i of [1, 2, 3]) {
  let inner;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "inner"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_of_2() {
    let src = r#"
let global1;
for (let { i, j } of [{ i: 1, j: 2 }, { i : 3, j: 4 }]) i += 2;
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "j"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_of_3() {
    let src = r#"
let global1 = 0;
for (global1 of [1, 2, 3]) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert!(for_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_in_1() {
    let src = r#"
let global1;
for (let i in [1, 2, 3]) {
  let inner;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i", "inner"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_in_2() {
    let src = r#"
let global1;
for (let i in [1, 2, 3]) i += 2;
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["i"], for_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_for_in_3() {
    let src = r#"
let global1 = 0;
for (global1 in [1, 2, 3]) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let for_vars = variables(scope_iter.next().unwrap());
    assert!(for_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_while_1() {
    let src = r#"
let global1;
while (true) {
  let inner;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner"], while_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_while_2() {
    let src = r#"
let global1;
while (true) foo();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert!(while_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_do_while_1() {
    let src = r#"
let global1;
do {
  let inner;
} while (true)
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner"], while_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_do_while_2() {
    let src = r#"
let global1;
do foo(); while (true)
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert!(while_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_with_1() {
    let src = r#"
let global1;
with (foo) {
  let inner = 1;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let with_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["inner"], with_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_with_2() {
    let src = r#"
let global1;
with (foo) bar();
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let with_vars = variables(scope_iter.next().unwrap());
    assert!(with_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_try_catch() {
    let src = r#"
let global1;
try {
  let tryVar;
  foo();
} catch(e) {
  let catchVar;
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let try_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["tryVar"], try_vars);

    let catch_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["catchVar", "e"], catch_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_class() {
    let src = r#"
let global1;
class Foo {
  field;
  #privateField;
  constructor(consParam) {
    let cons;
  }
  get getter() {
    let g;
  }
  set setter() {
    let s;
  }
  static staticMethod(staticMethodParam) {
    let sm;
  }
  method(methodParam) {
    let m;
  }
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let class_vars = variables(scope_iter.next().unwrap());
    assert!(class_vars.is_empty()); // collector doesn't collect class fields

    let constructor_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["cons", "consParam"], constructor_vars);

    let getter_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["g"], getter_vars);

    let setter_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["s"], setter_vars);

    let static_method_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["sm", "staticMethodParam"], static_method_vars);

    let method_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["m", "methodParam"], method_vars);

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn collector_works_complex_1() {
    let src = r#"
let global1;
function foo({ p1 = 0 }) {
  while (cond) {
    let while1 = true;
    if (while1) {
      break;
    }
    let [while2, { while3, key: while4 = 4 }] = bar();
  }
}
let global2 = 42;
    "#;
    let v = collect(src);
    let mut scope_iter = v.scopes.values();

    let global_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["global1", "global2"], global_vars);

    let function_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["p1"], function_vars);

    let while_vars = variables(scope_iter.next().unwrap());
    assert_eq!(vec!["while1", "while2", "while3", "while4"], while_vars);

    let if_vars = variables(scope_iter.next().unwrap());
    assert!(if_vars.is_empty());

    assert!(scope_iter.next().is_none());
  }

  #[test]
  fn var_groups_1() {
    let src = r#"
let { foo, bar } = obj;
let baz = 42;
    "#;
    let mut v = collect(src);
    assert_eq!(v.var_groups.roots.len(), 2);
    assert_eq!(v.var_groups.dump().len(), 3);
  }

  #[test]
  fn var_groups_2() {
    let src = r#"
let { foo, bar: { bar, baz: x = 42 } } = obj;
    "#;
    let mut v = collect(src);
    assert_eq!(v.var_groups.roots.len(), 1);
    assert_eq!(
      v.var_groups.roots.values().next().unwrap().0,
      VarStatus::Initialized
    );
    assert_eq!(v.var_groups.dump().len(), 3);
  }

  #[test]
  fn var_groups_3() {
    let src = r#"
function f(x: number, y: string = 42) {}
"#;
    let mut v = collect(src);
    assert_eq!(v.var_groups.roots.len(), 2);
    for &(s, _) in v.var_groups.roots.values() {
      assert_eq!(s, VarStatus::Reassigned);
    }
    assert_eq!(v.var_groups.dump().len(), 0);
  }

  #[test]
  fn var_groups_4() {
    let src = r#"
try {} catch (e) {}
"#;
    let mut v = collect(src);
    assert_eq!(v.var_groups.roots.len(), 1);
    assert_eq!(
      v.var_groups.roots.values().next().unwrap().0,
      VarStatus::Reassigned
    );
    assert_eq!(v.var_groups.dump().len(), 0);
  }

  #[test]
  fn var_groups_5() {
    let src = r#"
for (let {a, b} of obj) {}
"#;
    let mut v = collect(src);
    assert_eq!(v.var_groups.roots.len(), 1);
    assert_eq!(
      v.var_groups.roots.values().next().unwrap().0,
      VarStatus::Initialized
    );
    assert_eq!(v.var_groups.dump().len(), 2);
  }
}

#[cfg(test)]
mod prefer_const_tests {
  use crate::test_util;

  use super::*;

  // Some tests are derived from
  // https://github.com/eslint/eslint/blob/v7.10.0/tests/lib/rules/prefer-const.js
  // MIT Licensed.

  #[test]
  fn prefer_const_valid() {
    assert_lint_ok! {
      PreferConst,
      r#"var x = 0;"#,
      r#"let x;"#,
      r#"let x = 0; x += 1;"#,
      r#"let x = 0; x -= 1;"#,
      r#"let x = 0; x++;"#,
      r#"let x = 0; ++x;"#,
      r#"let x = 0; x--;"#,
      r#"let x = 0; --x;"#,
      r#"let x; { x = 0; } foo(x);"#,
      r#"let x = 0; x = 1;"#,
      r#"const x = 0;"#,
      r#"for (let i = 0, end = 10; i < end; ++i) {}"#,
      r#"for (let i = 0, end = 10; i < end; i += 2) {}"#,
      r#"for (let i in [1,2,3]) { i = 0; }"#,
      r#"for (let x of [1,2,3]) { x = 0; }"#,
      r#"(function() { var x = 0; })();"#,
      r#"(function() { let x; })();"#,
      r#"(function() { let x; { x = 0; } foo(x); })();"#,
      r#"(function() { let x = 0; x = 1; })();"#,
      r#"(function() { const x = 0; })();"#,
      r#"(function() { for (let i = 0, end = 10; i < end; ++i) {} })();"#,
      r#"(function() { for (let i in [1,2,3]) { i = 0; } })();"#,
      r#"(function() { for (let x of [1,2,3]) { x = 0; } })();"#,
      r#"(function(x = 0) { })();"#,
      r#"let a; while (a = foo());"#,
      r#"let a; do {} while (a = foo());"#,
      r#"let a; for (; a = foo(); );"#,
      r#"let a; for (;; ++a);"#,
      r#"let a; for (const {b = ++a} in foo());"#,
      r#"let a; for (const {b = ++a} of foo());"#,
      r#"let a; for (const x of [1,2,3]) { if (a) {} a = foo(); }"#,
      r#"let a; for (const x of [1,2,3]) { a = a || foo(); bar(a); }"#,
      r#"let a; for (const x of [1,2,3]) { foo(++a); }"#,
      r#"let a; function foo() { if (a) {} a = bar(); }"#,
      r#"let a; function foo() { a = a || bar(); baz(a); }"#,
      r#"let a; function foo() { bar(++a); }"#,
      r#"
      let id;
      function foo() {
          if (typeof id !== 'undefined') {
              return;
          }
          id = setInterval(() => {}, 250);
      }
      foo();
      "#,
      r#"let a; function init() { a = foo(); }"#,
      r#"let a; if (true) a = 0; foo(a);"#,
      r#"
        (function (a) {
            let b;
            ({ a, b } = obj);
        })();
        "#,
      r#"
        (function (a) {
            let b;
            ([ a, b ] = obj);
        })();
        "#,
      r#"var a; { var b; ({ a, b } = obj); }"#,
      r#"let a; { let b; ({ a, b } = obj); }"#,
      r#"var a; { var b; ([ a, b ] = obj); }"#,
      r#"let a; { let b; ([ a, b ] = obj); }"#,
      r#"let x; { x = 0; foo(x); }"#,
      r#"(function() { let x; { x = 0; foo(x); } })();"#,
      r#"let x; for (const a of [1,2,3]) { x = foo(); bar(x); }"#,
      r#"(function() { let x; for (const a of [1,2,3]) { x = foo(); bar(x); } })();"#,
      r#"let x; for (x of array) { x; }"#,

      // if destructuring assignment pattern contains member access (e.g. `typeNode.returnType` in
      // the above cases) then `predicate` should be treated as "reassigned".
      r#"let predicate; [typeNode.returnType, predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType, ...predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType,, predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType=5, predicate] = foo();"#,
      r#"let predicate; [[typeNode.returnType=5], predicate] = foo();"#,
      r#"let predicate; [[typeNode.returnType, predicate]] = foo();"#,
      r#"let predicate; [typeNode.returnType, [predicate]] = foo();"#,
      r#"let predicate; [, [typeNode.returnType, predicate]] = foo();"#,
      r#"let predicate; [, {foo:typeNode.returnType, predicate}] = foo();"#,
      r#"let predicate; [, {foo:typeNode.returnType, ...predicate}] = foo();"#,
      r#"let a; const b = {}; ({ a, c: b.c } = func());"#,
      r#"const x = [1,2]; let y; [,y] = x; y = 0;"#,
      r#"const x = [1,2,3]; let y, z; [y,,z] = x; y = 0; z = 0;"#,
      r#"try { foo(); } catch (e) {}"#,
      r#"let a; try { foo(); a = 1; } catch (e) {}"#,
      r#"let a; try { foo(); } catch (e) { a = 1; }"#,

      // https://github.com/denoland/deno_lint/issues/358
      r#"let a; const b = (a = foo === bar || baz === qux);"#,
      r#"let i = 0; b[i++] = 0;"#,

      // hoisting
      r#"
      function foo() {
        a += 1;
      }
      let a = 1;
      foo();
      "#,
      r#"
      let a = 1;
      function foo() {
        function bar() {
          a++;
        }
        let a = 9999;
        bar();
        console.log(a); // 10000
      }
      foo();
      a *= 2;
      console.log(a); // 2
      "#,

      // https://github.com/denoland/deno_lint/issues/358#issuecomment-703587510
      r#"let a = 0; for (a in [1, 2, 3]) foo(a);"#,
      r#"let a = 0; for (a of [1, 2, 3]) foo(a);"#,
      r#"let a = 0; for (a = 0; a < 10; a++) foo(a);"#,

      // https://github.com/denoland/deno_lint/issues/522
      // imitates `{ "destructuring": "all" }` in ESLint
      r#"let {a, b} = obj; b = 0;"#,
      r#"let a, b; ({a, b} = obj); b++;"#,
      r#"let a, b, c; ({a, b} = obj1); ({b, c} = obj2); b++;"#,
      r#"let {a = 0, b} = obj; b = 0; foo(a, b);"#,
      r#"let {a: {b, c}} = {a: {b: 1, c: 2}}; b = 3;"#,
      r#"let a, b; ({a = 0, b} = obj); b = 0; foo(a, b);"#,
      r#"let { name, ...otherStuff } = obj; otherStuff = {};"#,
      r#"(function() { let {a: x = -1, b: y} = {a:1,b:2}; y = 0; })();"#,
      r#"(function() { let [x = -1, y] = [1,2]; y = 0; })();"#,
      r#"let {a: x = -1, b: y} = {a:1,b:2}; y = 0;"#,
      r#"let [x = -1, y] = [1,2]; y = 0;"#,

      // https://github.com/denoland/deno_lint/issues/665
      r#"let a; switch (foo) { case 0: a = 0; break; }"#,
      r#"let a; switch (foo) { case 0: bar(); break; default: a = "default"; break; }"#,
      r#"let a; switch (foo) { case 0: { a = 0; break; } }"#,

      // https://github.com/denoland/deno_lint/issues/1065
      r#"let x = 0; const funcA = (someNumber: number = x++) => { return someNumber; };"#,
      r#"let x = 0; const funcA = (someNumber: number = x = x + 1) => { return someNumber; };"#,
      r#"let x = 0; const funcA = (someNumber: number = x += 1) => { return someNumber; };"#,
      r#"let x = 0; function foo(someNumber: number = x++) { return someNumber; };"#,
      r#"let x = 0; function foo(someNumber: number = x = x + 1) { return someNumber; };"#,
      r#"let x = 0; function foo(someNumber: number = x += 1) { return someNumber; };"#,

    };
  }

  #[test]
  fn prefer_const_invalid() {
    assert_lint_err! {
      PreferConst,
      r#"let x = 1;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 1; foo(x);"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let i in [1,2,3]) { foo(i); }"#: [
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "i"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let x of [1,2,3]) { foo(x); }"#: [
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { let x = 1; foo(x); })();"#: [
        {
          col: 18,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { for (let i in [1,2,3]) { foo(i); } })();"#: [
        {
          col: 23,
          message: variant!(PreferConstMessage, NeverReassigned, "i"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { for (let x of [1,2,3]) { foo(x); } })();"#: [
        {
          col: 23,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let f = (function() { let g = x; })(); f = 1;"#: [
        {
          col: 26,
          message: variant!(PreferConstMessage, NeverReassigned, "g"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 0; { let x = 1; foo(x); } x = 0;"#: [
        {
          col: 17,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let i = 0; i < 10; ++i) { let x = 1; foo(x); }"#: [
        {
          col: 35,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"for (let i in [1,2,3]) { let x = 1; foo(x); }"#: [
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "i"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 29,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       a = 1;
   }
};
    "#: [
        {
          line: 4,
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       ({a} = 1);
   }
};
    "#: [
        {
          line: 4,
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x; x = 0;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"switch (a) { case 0: let x; x = 0; }"#: [
        {
          col: 25,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"(function() { let x; x = 1; })();"#: [
        {
          col: 18,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let [a] = [1]"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a} = obj"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a = 0, b} = obj, c = a; b = a;"#: [
        {
          col: 22,
          message: variant!(PreferConstMessage, NeverReassigned, "c"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x; function foo() { bar(x); } x = 0;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"/*eslint use-x:error*/ let x = 1"#: [
        {
          col: 27,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"/*eslint use-x:error*/ { let x = 1 }"#: [
        {
          col: 29,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let { foo, bar } = baz;"#: [
        {
          col: 6,
          message: variant!(PreferConstMessage, NeverReassigned, "foo"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "bar"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"const x = [1,2]; let [,y] = x;"#: [
        {
          col: 23,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"const x = [1,2,3]; let [y,,z] = x;"#: [
        {
          col: 24,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 27,
          message: variant!(PreferConstMessage, NeverReassigned, "z"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let predicate; [, {foo:returnType, predicate}] = foo();"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "predicate"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let predicate; [, {foo:returnType, predicate}, ...bar ] = foo();"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "predicate"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let predicate; [, {foo:returnType, ...predicate} ] = foo();"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "predicate"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 'x', y = 'y';"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 13,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 'x', y = 'y'; x = 1"#: [
        {
          col: 13,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let x = 1, y = 'y'; let z = 1;"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 24,
          message: variant!(PreferConstMessage, NeverReassigned, "z"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let { a, b, c } = obj; let { x, y, z } = anotherObj; x = 2;"#: [
        {
          col: 6,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 9,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 12,
          message: variant!(PreferConstMessage, NeverReassigned, "c"),
          hint: PreferConstHint::UseConst,
        },
      ],
      r#"let x = 'x', y = 'y'; function someFunc() { let a = 1, b = 2; foo(a, b) }"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "x"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 13,
          message: variant!(PreferConstMessage, NeverReassigned, "y"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 48,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 55,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let someFunc = () => { let a = 1, b = 2; foo(a, b) }"#: [
        {
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "someFunc"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 27,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 34,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a, b} = c, d;"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 8,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"let {a, b, c} = {}, e, f;"#: [
        {
          col: 5,
          message: variant!(PreferConstMessage, NeverReassigned, "a"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 8,
          message: variant!(PreferConstMessage, NeverReassigned, "b"),
          hint: PreferConstHint::UseConst,
        },
        {
          col: 11,
          message: variant!(PreferConstMessage, NeverReassigned, "c"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
function a() {
  let foo = 0,
  bar = 1;
  foo = 1;
}
function b() {
  let foo = 0,
  bar = 2;
  foo = 2;
}
    "#: [
        {
          line: 4,
          col: 2,
          message: variant!(PreferConstMessage, NeverReassigned, "bar"),
          hint: PreferConstHint::UseConst,
        },
        {
          line: 9,
          col: 2,
          message: variant!(PreferConstMessage, NeverReassigned, "bar"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
let foo = function(a, b) {
  let c, d, e;
  ({ x: a, y: c } = bar());
  function inner() {
    d = 'd';
  }
  e = 'e';
};
if (true) foo = 'foo';
    "#: [
        {
          line: 3,
          col: 12,
          message: variant!(PreferConstMessage, NeverReassigned, "e"),
          hint: PreferConstHint::UseConst,
        }
      ],
      r#"
let e;
try {
  foo();
} catch (e) {
  e = 1;
  e++;
}
e = 2;
    "#: [
        {
          line: 2,
          col: 4,
          message: variant!(PreferConstMessage, NeverReassigned, "e"),
          hint: PreferConstHint::UseConst,
        }
      ]
    };
  }

  #[test]
  fn issue1145_panic_while_scope_analysis() {
    test_util::assert_lint_not_panic(
      Box::new(PreferConst),
      r#"
for await (let [[...x] = function() { initCount += 1; }()] of [[values]]) {
  assert(Array.isArray(x));
  assert.sameValue(x[0], 2);
}
      "#,
    );
  }
}
