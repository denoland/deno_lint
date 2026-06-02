// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::context::leftmost_identifier;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::semantic::Scoping;
use deno_ast::oxc::span::Span;
use deno_ast::oxc::syntax::scope::ScopeFlags;
use deno_ast::oxc::syntax::symbol::SymbolId;
use derive_more::Display;
use std::collections::HashSet;

#[derive(Debug)]
pub struct NoUnusedVars;

const CODE: &str = "no-unused-vars";

#[derive(Display)]
enum NoUnusedVarsMessage {
  #[display(fmt = "`{}` is never used", _0)]
  NeverUsed(String),
}

#[derive(Display)]
enum NoUnusedVarsHint {
  #[display(
    fmt = "If this is intentional, prefix it with an underscore like `_{}`",
    _0
  )]
  AddPrefix(String),
  #[display(
    fmt = "If this is intentional, alias it with an underscore like `{} as _{}`",
    _0,
    _0
  )]
  Alias(String),
}

impl LintRule for NoUnusedVars {
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
    let scoping = context.scoping();
    let mut collector = Collector {
      scoping,
      cur_defining: vec![],
      used_types: Default::default(),
      used_vars: Default::default(),
      jsx_factory: context
        .jsx_factory()
        .map(|s| leftmost_identifier(s).to_string()),
      jsx_fragment_factory: context
        .jsx_fragment_factory()
        .map(|s| leftmost_identifier(s).to_string()),
    };
    collector.visit_program(program);

    let mut visitor = NoUnusedVarVisitor::new(
      context,
      collector.used_vars,
      collector.used_types,
    );
    visitor.visit_program(program);
  }
}

/// Collects information about variable usages.
struct Collector<'s> {
  scoping: &'s Scoping,
  used_vars: HashSet<SymbolId>,
  used_types: HashSet<SymbolId>,
  /// Currently defining symbols. Usages of these are self-referential
  /// and should not count as "used".
  cur_defining: Vec<SymbolId>,
  /// The leftmost identifier of the JSX factory (e.g. "React" from "React.createElement").
  /// Marked as used on the first JSX element encountered, then taken.
  jsx_factory: Option<String>,
  /// The leftmost identifier of the JSX fragment factory (e.g. "React" from "React.Fragment").
  /// Marked as used on the first JSX fragment encountered, then taken.
  jsx_fragment_factory: Option<String>,
}

impl Collector<'_> {
  /// Resolve an IdentifierReference to its SymbolId, if it has one.
  fn resolve_reference(
    &self,
    ident: &IdentifierReference<'_>,
  ) -> Option<SymbolId> {
    let ref_id = ident.reference_id.get()?;
    let reference = self.scoping.get_reference(ref_id);
    reference.symbol_id()
  }

  /// Mark a name as used by looking up all symbols with that name.
  /// Used for JSX factory names and other string-based lookups.
  fn mark_name_as_usage(&mut self, name: &str) {
    // Look up all symbols with this name through scoping
    // This is a fallback for cases where we only have a string name
    for symbol_id in self.scoping.symbol_ids() {
      if self.scoping.symbol_name(symbol_id) == name {
        if !self.cur_defining.contains(&symbol_id) {
          self.used_vars.insert(symbol_id);
        }
      }
    }
  }

  /// The variable usage during its declaration should _NOT_ be treated as used.
  /// For example:
  ///
  /// ```typescript
  /// // `a` is called, but effectively nothing occurs until `a` is called from _outside_ of this
  /// // function body.
  /// const a = () => { a(); };
  ///
  /// // Same goes for type or interface definitions.
  /// type JsonValue = number | string | boolean | Array<JsonValue> | {
  ///   [key: string]: JsonValue;
  /// };
  /// interface Foo {
  ///   a: Foo;
  /// }
  /// ```
  ///
  /// To handle it, we need to store the variables that are currently being declared.
  /// This is a helper method, responsible for preserving and then restoring variables data.
  fn with_cur_defining<I, F>(&mut self, ids: I, op: F)
  where
    I: IntoIterator<Item = SymbolId>,
    F: FnOnce(&mut Collector<'_>),
  {
    // Preserve the original state
    let prev_len = self.cur_defining.len();
    self.cur_defining.extend(ids);

    op(self);

    // Restore the original state
    self.cur_defining.drain(prev_len..);
    assert_eq!(self.cur_defining.len(), prev_len);
  }

  /// This is a helper method, responsible for temporarily ignoring `cur_defining` while doing
  /// the given operation (`op`).
  ///
  /// For some context, we need to ignore variables that are being declared (which we call
  /// `cur_defining`).
  /// Take function arguments as an example. If `cur_defining` is used inside the arguments, we
  /// have to think of it as _used_.
  ///
  /// ```typescript
  /// const i = setInterval(() => {
  ///  clearInterval(i);
  /// }, 1000);
  /// ```
  ///
  /// In the above example, when visiting `setInterval`, we have `i` included in `cur_defining`.
  /// `setInterval` is taking a closure as an argument and `i` is used in it.
  /// Naturally we have to treat `i` as used, because this closure is effectively invoked
  /// lazily; not invoked at the time when `i` is being defined.
  fn without_cur_defining<F>(&mut self, op: F)
  where
    F: FnOnce(&mut Collector<'_>),
  {
    let prev = std::mem::take(&mut self.cur_defining);
    op(self);
    self.cur_defining = prev;
  }

  fn mark_as_usage(&mut self, symbol_id: SymbolId) {
    // Recursive calls are not usage
    if self.cur_defining.contains(&symbol_id) {
      return;
    }

    // Mark the variable as used.
    self.used_vars.insert(symbol_id);
  }

  fn mark_as_type_usage(&mut self, symbol_id: SymbolId) {
    if self.cur_defining.contains(&symbol_id) {
      return;
    }
    self.used_types.insert(symbol_id);
  }
}

/// Extract all SymbolIds from binding identifiers in a BindingPattern.
fn collect_binding_symbol_ids(
  pat: &BindingPattern<'_>,
  out: &mut Vec<SymbolId>,
) {
  match pat {
    BindingPattern::BindingIdentifier(ident) => {
      if let Some(sym_id) = ident.symbol_id.get() {
        out.push(sym_id);
      }
    }
    BindingPattern::ObjectPattern(obj) => {
      for prop in &obj.properties {
        collect_binding_symbol_ids(&prop.value, out);
      }
      if let Some(rest) = &obj.rest {
        collect_binding_symbol_ids(&rest.argument, out);
      }
    }
    BindingPattern::ArrayPattern(arr) => {
      for elem in arr.elements.iter().flatten() {
        collect_binding_symbol_ids(elem, out);
      }
      if let Some(rest) = &arr.rest {
        collect_binding_symbol_ids(&rest.argument, out);
      }
    }
    BindingPattern::AssignmentPattern(assign) => {
      collect_binding_symbol_ids(&assign.left, out);
    }
  }
}

/// Extract all binding identifiers (name + span + symbol_id) from a BindingPattern.
fn collect_binding_idents(
  pat: &BindingPattern<'_>,
  out: &mut Vec<(String, Span, Option<SymbolId>)>,
) {
  match pat {
    BindingPattern::BindingIdentifier(ident) => {
      out.push((ident.name.to_string(), ident.span, ident.symbol_id.get()));
    }
    BindingPattern::ObjectPattern(obj) => {
      for prop in &obj.properties {
        collect_binding_idents(&prop.value, out);
      }
      if let Some(rest) = &obj.rest {
        collect_binding_idents(&rest.argument, out);
      }
    }
    BindingPattern::ArrayPattern(arr) => {
      for elem in arr.elements.iter().flatten() {
        collect_binding_idents(elem, out);
      }
      if let Some(rest) = &arr.rest {
        collect_binding_idents(&rest.argument, out);
      }
    }
    BindingPattern::AssignmentPattern(assign) => {
      collect_binding_idents(&assign.left, out);
    }
  }
}

impl<'a> Visit<'a> for Collector<'_> {
  fn visit_property_definition(&mut self, n: &PropertyDefinition<'a>) {
    for decorator in &n.decorators {
      self.visit_decorator(decorator);
    }

    if n.computed {
      self.visit_property_key(&n.key);
    }

    if let Some(value) = &n.value {
      self.visit_expression(value);
    }
    if let Some(type_ann) = &n.type_annotation {
      self.visit_ts_type_annotation(type_ann);
    }
  }

  fn visit_ts_property_signature(&mut self, n: &TSPropertySignature<'a>) {
    if n.computed {
      self.visit_property_key(&n.key);
    }

    if let Some(type_ann) = &n.type_annotation {
      self.visit_ts_type_annotation(type_ann);
    }
  }

  fn visit_ts_type_reference(&mut self, ty: &TSTypeReference<'a>) {
    if let Some(type_params) = &ty.type_arguments {
      self.visit_ts_type_parameter_instantiation(type_params);
    }

    // Resolve the leftmost identifier to its SymbolId for type usage tracking
    if let Some(ident) = get_leftmost_ident_ref_from_ts_type_name(&ty.type_name)
    {
      if let Some(sym_id) = self.resolve_reference(ident) {
        self.mark_as_type_usage(sym_id);
      }
    }
  }

  fn visit_ts_interface_heritage(&mut self, n: &TSInterfaceHeritage<'a>) {
    self.visit_expression(&n.expression);
    if let Some(type_args) = &n.type_arguments {
      self.visit_ts_type_parameter_instantiation(type_args);
    }
  }

  fn visit_ts_type_query(&mut self, n: &TSTypeQuery<'a>) {
    match &n.expr_name {
      TSTypeQueryExprName::TSImportType(_) => {}
      TSTypeQueryExprName::IdentifierReference(ident) => {
        if let Some(sym_id) = self.resolve_reference(ident) {
          self.mark_as_usage(sym_id);
        }
      }
      TSTypeQueryExprName::QualifiedName(qn) => {
        if let Some(ident) = get_leftmost_ident_ref_from_ts_type_name(&qn.left)
        {
          if let Some(sym_id) = self.resolve_reference(ident) {
            self.mark_as_usage(sym_id);
          }
        }
      }
      _ => {}
    }
    walk::walk_ts_type_query(self, n);
  }

  fn visit_object_property(&mut self, n: &ObjectProperty<'a>) {
    if n.shorthand {
      // Shorthand properties like `{ foo }` use the value expression
      // which is an IdentifierReference - let the default walk handle it
    }
    walk::walk_object_property(self, n);
  }

  fn visit_property_key(&mut self, n: &PropertyKey<'a>) {
    match n {
      PropertyKey::StaticIdentifier(_) | PropertyKey::PrivateIdentifier(_) => {
        // Don't visit identifiers used as property names
      }
      _ => {
        // Computed/expression keys - walk to find usages
        walk::walk_property_key(self, n);
      }
    }
  }

  fn visit_identifier_reference(&mut self, ident: &IdentifierReference<'a>) {
    if let Some(sym_id) = self.resolve_reference(ident) {
      self.mark_as_usage(sym_id);
    }
  }

  fn visit_expression(&mut self, expr: &Expression<'a>) {
    match expr {
      Expression::Identifier(i) => {
        if let Some(sym_id) = self.resolve_reference(i) {
          self.mark_as_usage(sym_id);
        }
      }
      _ => walk::walk_expression(self, expr),
    }
  }

  fn visit_jsx_element_name(&mut self, n: &JSXElementName<'a>) {
    if let Some(factory) = self.jsx_factory.take() {
      self.mark_name_as_usage(&factory);
    }
    match n {
      JSXElementName::IdentifierReference(i) => {
        if !i.name.starts_with(|c: char| c.is_ascii_lowercase()) {
          if let Some(sym_id) = self.resolve_reference(i) {
            self.mark_as_usage(sym_id);
          }
        }
      }
      JSXElementName::MemberExpression(n) => {
        self.visit_jsx_member_expression(n);
      }
      JSXElementName::NamespacedName(_)
      | JSXElementName::Identifier(_)
      | JSXElementName::ThisExpression(_) => {}
    }
  }

  fn visit_jsx_fragment(&mut self, n: &JSXFragment<'a>) {
    if let Some(factory) = self.jsx_fragment_factory.take() {
      self.mark_name_as_usage(&factory);
    }
    walk::walk_jsx_fragment(self, n);
  }

  fn visit_jsx_member_expression_object(
    &mut self,
    n: &JSXMemberExpressionObject<'a>,
  ) {
    match n {
      JSXMemberExpressionObject::IdentifierReference(i) => {
        if let Some(sym_id) = self.resolve_reference(i) {
          self.mark_as_usage(sym_id);
        }
      }
      JSXMemberExpressionObject::MemberExpression(n) => {
        self.visit_jsx_member_expression(n);
      }
      JSXMemberExpressionObject::ThisExpression(_) => {}
    }
  }

  fn visit_simple_assignment_target(&mut self, n: &SimpleAssignmentTarget<'a>) {
    match n {
      SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
        if let Some(sym_id) = self.resolve_reference(ident) {
          self.mark_as_usage(sym_id);
        }
      }
      _ => walk::walk_simple_assignment_target(self, n),
    }
  }

  fn visit_assignment_expression(&mut self, n: &AssignmentExpression<'a>) {
    if n.operator == AssignmentOperator::Assign {
      match &n.left {
        AssignmentTarget::AssignmentTargetIdentifier(_) => {
          // ignore LHS ident and only visit the right
          self.visit_expression(&n.right);
        }
        _ => walk::walk_assignment_expression(self, n),
      }

      if let Expression::NewExpression(new_expr) = &n.right {
        self.without_cur_defining(|a| {
          walk::walk_new_expression(a, new_expr);
        });
      }
    } else {
      walk::walk_assignment_expression(self, n)
    }
  }

  fn visit_binding_pattern(&mut self, pat: &BindingPattern<'a>) {
    match pat {
      // Ignore binding identifiers (they're declarations, not usages)
      BindingPattern::BindingIdentifier(_) => {}
      _ => walk::walk_binding_pattern(self, pat),
    }
  }

  fn visit_assignment_pattern(&mut self, n: &AssignmentPattern<'a>) {
    // handle codes like `const { foo, bar = foo } = { foo: 42 };`
    self.without_cur_defining(|a| {
      a.visit_expression(&n.right);
    });
    self.visit_binding_pattern(&n.left);
  }

  fn visit_static_member_expression(
    &mut self,
    member_expr: &StaticMemberExpression<'a>,
  ) {
    self.visit_expression(&member_expr.object);
    // Don't visit the property identifier - it's not a variable usage
  }

  fn visit_computed_member_expression(
    &mut self,
    member_expr: &ComputedMemberExpression<'a>,
  ) {
    self.visit_expression(&member_expr.object);
    self.visit_expression(&member_expr.expression);
  }

  /// export is kind of usage
  fn visit_export_specifier(&mut self, export: &ExportSpecifier<'a>) {
    match &export.local {
      ModuleExportName::IdentifierReference(ident) => {
        if let Some(sym_id) = self.resolve_reference(ident) {
          self.mark_as_usage(sym_id);
        }
      }
      ModuleExportName::IdentifierName(ident) => {
        // IdentifierName doesn't have a reference_id, fall back to name lookup
        self.mark_name_as_usage(ident.name.as_str());
      }
      ModuleExportName::StringLiteral(_) => {}
    }
  }

  fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
    if func.r#type == FunctionType::FunctionDeclaration {
      if let Some(id) = &func.id {
        if let Some(sym_id) = id.symbol_id.get() {
          self.with_cur_defining(std::iter::once(sym_id), |a| {
            a.visit_function_inner(func, flags);
          });
          return;
        }
      }
    }
    self.visit_function_inner(func, flags);
  }

  fn visit_class(&mut self, class: &Class<'a>) {
    if class.r#type == ClassType::ClassDeclaration {
      if let Some(id) = &class.id {
        if let Some(sym_id) = id.symbol_id.get() {
          self.with_cur_defining(std::iter::once(sym_id), |a| {
            walk::walk_class(a, class);
          });
          return;
        }
      }
    }
    walk::walk_class(self, class);
  }

  fn visit_ts_interface_declaration(
    &mut self,
    decl: &TSInterfaceDeclaration<'a>,
  ) {
    if let Some(sym_id) = decl.id.symbol_id.get() {
      self.with_cur_defining(std::iter::once(sym_id), |a| {
        for heritage in &decl.extends {
          a.visit_ts_interface_heritage(heritage);
        }
        a.visit_ts_interface_body(&decl.body);
        if let Some(type_params) = &decl.type_parameters {
          a.visit_ts_type_parameter_declaration(type_params);
        }
      });
    }
  }

  fn visit_ts_type_alias_declaration(
    &mut self,
    decl: &TSTypeAliasDeclaration<'a>,
  ) {
    if let Some(sym_id) = decl.id.symbol_id.get() {
      self.with_cur_defining(std::iter::once(sym_id), |a| {
        a.visit_ts_type(&decl.type_annotation);
        if let Some(type_params) = &decl.type_parameters {
          a.visit_ts_type_parameter_declaration(type_params);
        }
      });
    }
  }

  fn visit_ts_enum_declaration(&mut self, decl: &TSEnumDeclaration<'a>) {
    if let Some(sym_id) = decl.id.symbol_id.get() {
      self.with_cur_defining(std::iter::once(sym_id), |a| {
        a.visit_ts_enum_body(&decl.body);
      });
    }
  }

  fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
    let mut declaring_ids = vec![];
    collect_binding_symbol_ids(&declarator.id, &mut declaring_ids);
    self.with_cur_defining(declaring_ids, |a| {
      a.visit_binding_pattern(&declarator.id);
      if let Some(type_ann) = &declarator.type_annotation {
        a.visit_ts_type_annotation(type_ann);
      }
      if let Some(init) = &declarator.init {
        a.visit_expression(init);
      }
    });
  }

  fn visit_ts_import_equals_declaration(
    &mut self,
    decl: &TSImportEqualsDeclaration<'a>,
  ) {
    if let Some(sym_id) = decl.id.symbol_id.get() {
      self.with_cur_defining(std::iter::once(sym_id), |collector| match &decl
        .module_reference
      {
        TSModuleReference::IdentifierReference(ident) => {
          if let Some(ref_sym) = collector.resolve_reference(ident) {
            collector.mark_as_usage(ref_sym);
          }
        }
        TSModuleReference::QualifiedName(qn) => {
          if let Some(ident) =
            get_leftmost_ident_ref_from_ts_type_name(&qn.left)
          {
            if let Some(ref_sym) = collector.resolve_reference(ident) {
              collector.mark_as_usage(ref_sym);
            }
          }
        }
        TSModuleReference::ExternalModuleReference(_) => {}
      });
    }
  }

  fn visit_call_expression(&mut self, call_expr: &CallExpression<'a>) {
    self.visit_expression(&call_expr.callee);

    for arg in &call_expr.arguments {
      self.without_cur_defining(|a| {
        a.visit_argument(arg);
      });
    }

    if let Some(type_args) = &call_expr.type_arguments {
      self.visit_ts_type_parameter_instantiation(type_args);
    }
  }

  fn visit_for_in_statement(&mut self, n: &ForInStatement<'a>) {
    // The LHS of `for (x in obj)` is an assignment target, not a read.
    // Only visit VariableDeclaration LHS; skip assignment targets (they're writes, not reads).
    if let ForStatementLeft::VariableDeclaration(decl) = &n.left {
      self.visit_variable_declaration(decl);
    }
    // For assignment target LHS (simple ident, array/object destructuring),
    // the identifiers are being assigned to, not read. Skip them.
    self.visit_expression(&n.right);
    self.visit_statement(&n.body);
  }

  fn visit_for_of_statement(&mut self, n: &ForOfStatement<'a>) {
    // The LHS of `for (x of xs)` is an assignment target, not a read.
    if let ForStatementLeft::VariableDeclaration(decl) = &n.left {
      self.visit_variable_declaration(decl);
    }
    // For assignment target LHS, skip - identifiers are being written, not read.
    self.visit_expression(&n.right);
    self.visit_statement(&n.body);
  }
}

/// Get the leftmost IdentifierReference from a TSTypeName.
fn get_leftmost_ident_ref_from_ts_type_name<'a>(
  name: &'a TSTypeName<'a>,
) -> Option<&'a IdentifierReference<'a>> {
  match name {
    TSTypeName::IdentifierReference(ident) => Some(ident),
    TSTypeName::QualifiedName(qn) => {
      get_leftmost_ident_ref_from_ts_type_name(&qn.left)
    }
    TSTypeName::ThisExpression(_) => None,
  }
}

/// Helper for Collector to visit a Function's internals.
impl Collector<'_> {
  fn visit_function_inner(&mut self, func: &Function<'_>, flags: ScopeFlags) {
    // If the first parameter is `this` with a type annotation, it's a fake
    // TypeScript parameter. Mark it as used.
    if let Some(first_param) = func.params.items.first() {
      if let BindingPattern::BindingIdentifier(ident) = &first_param.pattern {
        if ident.name.as_str() == "this"
          && first_param.type_annotation.is_some()
        {
          self.mark_name_as_usage("this");
        }
      }
    }

    walk::walk_function(self, func, flags);
  }
}

struct NoUnusedVarVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
  used_vars: HashSet<SymbolId>,
  used_types: HashSet<SymbolId>,
}

impl<'c, 'view> NoUnusedVarVisitor<'c, 'view> {
  fn new(
    context: &'c mut Context<'view>,
    used_vars: HashSet<SymbolId>,
    used_types: HashSet<SymbolId>,
  ) -> Self {
    Self {
      context,
      used_vars,
      used_types,
    }
  }
}

#[derive(Debug, Clone, Copy)]
enum IdentKind {
  NamedImport,
  DefaultImport,
  StarAsImport,
  Other,
}

impl NoUnusedVarVisitor<'_, '_> {
  fn handle_binding(
    &mut self,
    name: &str,
    span: Span,
    symbol_id: Option<SymbolId>,
    kind: IdentKind,
  ) {
    if name.starts_with('_') {
      return;
    }

    let is_used = if let Some(sym_id) = symbol_id {
      self.used_vars.contains(&sym_id)
    } else {
      // No symbol_id means semantic analysis couldn't resolve it.
      // Be conservative and don't report.
      true
    };

    if !is_used {
      let message = NoUnusedVarsMessage::NeverUsed(name.to_string());
      let hint = match kind {
        IdentKind::NamedImport => NoUnusedVarsHint::Alias(name.to_string()),
        IdentKind::DefaultImport
        | IdentKind::StarAsImport
        | IdentKind::Other => NoUnusedVarsHint::AddPrefix(name.to_string()),
      };
      self
        .context
        .add_diagnostic_with_hint(span, CODE, message, hint);
    }
  }
}

impl<'a> Visit<'a> for NoUnusedVarVisitor<'_, 'a> {
  fn visit_arrow_function_expression(
    &mut self,
    expr: &ArrowFunctionExpression<'a>,
  ) {
    let mut idents = vec![];
    for param in &expr.params.items {
      collect_binding_idents(&param.pattern, &mut idents);
    }

    for (name, span, sym_id) in &idents {
      self.handle_binding(name, *span, *sym_id, IdentKind::Other);
    }
    for stmt in &expr.body.statements {
      self.visit_statement(stmt);
    }
  }

  fn visit_function(&mut self, func: &Function<'a>, flags: ScopeFlags) {
    match func.r#type {
      FunctionType::FunctionDeclaration => {
        if func.declare {
          return;
        }

        if let Some(id) = &func.id {
          self.handle_binding(
            id.name.as_str(),
            id.span,
            id.symbol_id.get(),
            IdentKind::Other,
          );
        }

        // If function body is not present, it's an overload definition
        if func.body.is_some() {
          walk::walk_function(self, func, flags);
        }
      }
      FunctionType::TSDeclareFunction
      | FunctionType::TSEmptyBodyFunctionExpression => {
        // TypeScript overloads / empty body / declare functions - don't report params
        if !func.declare {
          if let Some(id) = &func.id {
            self.handle_binding(
              id.name.as_str(),
              id.span,
              id.symbol_id.get(),
              IdentKind::Other,
            );
          }
        }
      }
      _ => {
        walk::walk_function(self, func, flags);
      }
    }
  }

  fn visit_variable_declaration(&mut self, n: &VariableDeclaration<'a>) {
    if n.declare {
      return;
    }

    for decl in &n.declarations {
      self.visit_variable_declarator(decl);
    }
  }

  fn visit_variable_declarator(&mut self, declarator: &VariableDeclarator<'a>) {
    let mut idents = vec![];
    collect_binding_idents(&declarator.id, &mut idents);

    for (name, span, sym_id) in &idents {
      self.handle_binding(name, *span, *sym_id, IdentKind::Other);
    }
    self.visit_binding_pattern(&declarator.id);
    if let Some(init) = &declarator.init {
      self.visit_expression(init);
    }
  }

  fn visit_class(&mut self, class: &Class<'a>) {
    if class.r#type == ClassType::ClassDeclaration {
      if class.declare {
        return;
      }

      if let Some(id) = &class.id {
        self.handle_binding(
          id.name.as_str(),
          id.span,
          id.symbol_id.get(),
          IdentKind::Other,
        );
      }
    }
    walk::walk_class(self, class);
  }

  // Interface/type method signatures and TS function types - don't report parameters as unused
  fn visit_ts_method_signature(&mut self, _n: &TSMethodSignature<'a>) {}
  fn visit_ts_call_signature_declaration(
    &mut self,
    _n: &TSCallSignatureDeclaration<'a>,
  ) {
  }
  fn visit_ts_construct_signature_declaration(
    &mut self,
    _n: &TSConstructSignatureDeclaration<'a>,
  ) {
  }
  // TSFunctionType params are type-level only (e.g. `(user: User) => void`)
  // and should not be flagged as unused variables.
  fn visit_ts_function_type(&mut self, _n: &TSFunctionType<'a>) {}

  fn visit_catch_clause(&mut self, clause: &CatchClause<'a>) {
    if let Some(param) = &clause.param {
      let mut idents = vec![];
      collect_binding_idents(&param.pattern, &mut idents);

      for (name, span, sym_id) in &idents {
        self.handle_binding(name, *span, *sym_id, IdentKind::Other);
      }
    }

    self.visit_block_statement(&clause.body);
  }

  fn visit_method_definition(&mut self, method: &MethodDefinition<'a>) {
    for decorator in &method.decorators {
      self.visit_decorator(decorator);
    }
    self.visit_property_key(&method.key);

    match method.kind {
      MethodDefinitionKind::Method => {
        // If method body is not present, it's an overload definition
        if method.value.body.is_some() {
          for param in &method.value.params.items {
            self.visit_formal_parameter(param);
          }
        }
      }
      MethodDefinitionKind::Set => {
        // For setters, visit the key and body but skip parameter reporting
      }
      MethodDefinitionKind::Constructor => {
        // If body is not present, it's an overload definition
        if method.value.body.is_none() {
          return;
        }
        walk::walk_function(
          self,
          &method.value,
          ScopeFlags::Function | ScopeFlags::Constructor,
        );
        return;
      }
      MethodDefinitionKind::Get => {}
    }

    if let Some(body) = &method.value.body {
      self.visit_function_body(body);
    }
  }

  fn visit_object_property(&mut self, n: &ObjectProperty<'a>) {
    if n.kind == PropertyKind::Set {
      // For setter properties, visit key and body but skip parameter reporting
      self.visit_property_key(&n.key);
      if let Expression::FunctionExpression(func) = &n.value {
        if let Some(body) = &func.body {
          self.visit_function_body(body);
        }
      }
      return;
    }
    walk::walk_object_property(self, n);
  }

  fn visit_formal_parameter(&mut self, param: &FormalParameter<'a>) {
    // TypeScript parameter properties (e.g. `constructor(private x: T)`)
    // are automatically class fields and should not be flagged as unused.
    if param.accessibility.is_some() || param.readonly {
      return;
    }

    let mut idents = vec![];
    collect_binding_idents(&param.pattern, &mut idents);

    for (name, span, sym_id) in &idents {
      self.handle_binding(name, *span, *sym_id, IdentKind::Other);
    }
    walk::walk_formal_parameter(self, param);
  }

  fn visit_import_specifier(&mut self, import: &ImportSpecifier<'a>) {
    let sym_id = import.local.symbol_id.get();
    if let Some(s) = sym_id {
      if self.used_types.contains(&s) {
        return;
      }
    }
    self.handle_binding(
      import.local.name.as_str(),
      import.local.span,
      sym_id,
      IdentKind::NamedImport,
    );
  }

  fn visit_import_default_specifier(
    &mut self,
    import: &ImportDefaultSpecifier<'a>,
  ) {
    let sym_id = import.local.symbol_id.get();
    if let Some(s) = sym_id {
      if self.used_types.contains(&s) {
        return;
      }
    }

    self.handle_binding(
      import.local.name.as_str(),
      import.local.span,
      sym_id,
      IdentKind::DefaultImport,
    );
  }

  fn visit_import_namespace_specifier(
    &mut self,
    import: &ImportNamespaceSpecifier<'a>,
  ) {
    let sym_id = import.local.symbol_id.get();
    if let Some(s) = sym_id {
      if self.used_types.contains(&s) {
        return;
      }
    }
    self.handle_binding(
      import.local.name.as_str(),
      import.local.span,
      sym_id,
      IdentKind::StarAsImport,
    );
  }

  fn visit_ts_import_equals_declaration(
    &mut self,
    decl: &TSImportEqualsDeclaration<'a>,
  ) {
    self.handle_binding(
      decl.id.name.as_str(),
      decl.id.span,
      decl.id.symbol_id.get(),
      IdentKind::Other,
    );
  }

  /// No error as export is kind of usage
  fn visit_export_named_declaration(
    &mut self,
    export: &ExportNamedDeclaration<'a>,
  ) {
    if let Some(decl) = &export.declaration {
      match decl {
        Declaration::ClassDeclaration(c) if !c.declare => {
          walk::walk_class(self, c);
        }
        Declaration::FunctionDeclaration(f) if !f.declare => {
          // If function body is not present, it's an overload definition
          if f.body.is_some() {
            walk::walk_function(self, f, ScopeFlags::Function);
          }
        }
        Declaration::VariableDeclaration(v) if !v.declare => {
          for decl in &v.declarations {
            self.visit_binding_pattern(&decl.id);
            if let Some(init) = &decl.init {
              self.visit_expression(init);
            }
          }
        }
        _ => {}
      }
    }
    // Don't walk specifiers - export is usage
  }

  fn visit_export_default_declaration(
    &mut self,
    export: &ExportDefaultDeclaration<'a>,
  ) {
    match &export.declaration {
      ExportDefaultDeclarationKind::ClassDeclaration(c) => {
        walk::walk_class(self, c);
      }
      ExportDefaultDeclarationKind::FunctionDeclaration(f) => {
        // If function body is not present, it's an overload definition
        if f.body.is_some() {
          walk::walk_function(self, f, ScopeFlags::Function);
        }
      }
      ExportDefaultDeclarationKind::TSInterfaceDeclaration(i) => {
        walk::walk_ts_interface_declaration(self, i);
      }
      _ => {
        walk::walk_export_default_declaration(self, export);
      }
    }
  }

  fn visit_formal_parameters(&mut self, params: &FormalParameters<'a>) {
    // Skip the `this` parameter if present
    let skip = if let Some(first) = params.items.first() {
      if let BindingPattern::BindingIdentifier(ident) = &first.pattern {
        ident.name.as_str() == "this"
      } else {
        false
      }
    } else {
      false
    };

    if skip {
      for param in params.items.iter().skip(1) {
        self.visit_formal_parameter(param);
      }
    } else {
      for param in &params.items {
        self.visit_formal_parameter(param);
      }
    }
  }

  fn visit_ts_enum_declaration(&mut self, n: &TSEnumDeclaration<'a>) {
    if n.declare {
      return;
    }

    if let Some(sym_id) = n.id.symbol_id.get() {
      if self.used_types.contains(&sym_id) {
        return;
      }
    }
    self.handle_binding(
      n.id.name.as_str(),
      n.id.span,
      n.id.symbol_id.get(),
      IdentKind::Other,
    );
  }

  fn visit_ts_module_declaration(&mut self, n: &TSModuleDeclaration<'a>) {
    if n.declare {
      return;
    }

    if let Some(body) = &n.body {
      self.visit_ts_module_declaration_body(body);
    }
  }

  /// no-op as export is kind of usage
  fn visit_export_specifier(&mut self, _: &ExportSpecifier<'a>) {}
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unused_vars_valid() {
    assert_lint_ok! {
      NoUnusedVars,
      "var a = 1; console.log(a)",
      "var a = 1; const arrow = () => a; console.log(arrow)",
      "var a = 1; console.log?.(a)",
      "var a = 1; a?.()",
      // Hoisting. This code is wrong, but it's not related with unused-vars
      "console.log(a); var a = 1;",
      "var foo = 5;\n\nlabel: while (true) {\n  console.log(foo);\n  break label;\n}",
      "var foo = 5;\n\nwhile (true) {\n  console.log(foo);\n  break;\n}",
      "for (let prop in box) {\n        box[prop] = parseInt(box[prop]);\n}",
      "var box = {a: 2};\n    for (var prop in box) {\n        box[prop] = parseInt(box[prop]);\n}",
      "f({ set foo(a) { return; } });",
      "a; var a;",
      "var a=10; alert(a);",
      "var a=10; (function() { alert(a); })();",
      "var a=10; (function() { setTimeout(function() { alert(a); }, 0); })();",
      "var a=10; d[a] = 0;",
      "(function() { var a=10; return a; })();",
      "(function g() {})()",
      "function f(a) {alert(a);}; f();",
      "var c = 0; function f(a){ var b = a; return b; }; f(c);",
      "var arr1 = [1, 2]; var arr2 = [3, 4]; for (var i in arr1) { arr1[i] = 5; } for (var i in arr2) { arr2[i] = 10; }",
      "var min = \"min\"; Math[min];",
      "Foo.bar = function(baz) { return baz; };",
      "myFunc(function foo() {}.bind(this))",
      "myFunc(function foo(){}.toString())",
      "(function() { var doSomething = function doSomething() {}; doSomething() }())",
      "/*global a */ a;",
      "var a=10; (function() { alert(a); })();",
      "var a=10; (function() { alert(a); })();",
      "(function z() { z(); })();",
      "var who = \"Paul\";\nmodule.exports = `Hello ${who}!`;",
      "export var foo = 123;",
      "export function foo () {}",
      "let toUpper = (partial) => partial.toUpperCase; export {toUpper}",
      "export class foo {}",
      "class Foo{}; var x = new Foo(); x.foo()",
      "const foo = \"hello!\";function bar(foobar = foo) {  foobar.replace(/!$/, \" world!\");}\nbar();",
      "function Foo(){}; var x = new Foo(); x.foo()",
      "function foo() {var foo = 1; return foo}; foo();",
      "function foo(foo) {return foo}; foo(1);",
      "function foo() {function foo() {return 1;}; return foo()}; foo();",
      "function foo() {var foo = 1; return foo}; foo();",
      "function foo(foo) {return foo}; foo(1);",
      "function foo() {function foo() {return 1;}; return foo()}; foo();",
      "const x = 1; const [y = x] = []; foo(y);",
      "const x = 1; const {y = x} = {}; foo(y);",
      "const x = 1; const {z: [y = x]} = {}; foo(y);",
      "const x = []; const {z: [y] = x} = {}; foo(y);",
      "const x = 1; let y; [y = x] = []; foo(y);",
      "const x = 1; let y; ({z: [y = x]} = {}); foo(y);",
      "const x = []; let y; ({z: [y] = x} = {}); foo(y);",
      "const x = 1; function foo(y = x) { bar(y); } foo();",
      "const x = 1; function foo({y = x} = {}) { bar(y); } foo();",
      "const x = 1; function foo(y = function(z = x) { bar(z); }) { y(); } foo();",
      "const x = 1; function foo(y = function() { bar(x); }) { y(); } foo();",
      "var x = 1; var [y = x] = []; foo(y);",
      "var x = 1; var {y = x} = {}; foo(y);",
      "var x = 1; var {z: [y = x]} = {}; foo(y);",
      "var x = []; var {z: [y] = x} = {}; foo(y);",
      "var x = 1, y; [y = x] = []; foo(y);",
      "var x = 1, y; ({z: [y = x]} = {}); foo(y);",
      "var x = [], y; ({z: [y] = x} = {}); foo(y);",
      "var x = 1; function foo(y = x) { bar(y); } foo();",
      "var x = 1; function foo({y = x} = {}) { bar(y); } foo();",
      "var x = 1; function foo(y = function(z = x) { bar(z); }) { y(); } foo();",
      "var x = 1; function foo(y = function() { bar(x); }) { y(); } foo();",
      "var _a",
      "function foo(_a) { } foo();",
      "function foo(a, _b) { return a; } foo();",
      "try{}catch(err){console.error(err);}",
      "try{}catch(_ignoreErr){}",
      "var a = 0, b; b = a = a + 1; foo(b);",
      "var a = 0, b; b = a += a + 1; foo(b);",
      "var a = 0, b; b = a++; foo(b);",
      "function foo(a) { var b = a = a + 1; bar(b) } foo();",
      "function foo(a) { var b = a += a + 1; bar(b) } foo();",
      "function foo(a) { var b = a++; bar(b) } foo();",
      "function foo(cb) { cb = function() { function something(a) { cb(1 + a); } register(something); }(); } foo();",
      "function* foo(cb) { cb = yield function(a) { cb(1 + a); }; } foo();",
      "function foo(cb) { cb = tag`hello${function(a) { cb(1 + a); }}`; } foo();",
      "function foo(cb) { var b; cb = b = function(a) { cb(1 + a); }; b(); } foo();",
      "(class { set foo(UNUSED) {} })",
      "class Foo { set bar(UNUSED) {} } console.log(Foo)",
      "var a = function () { a(); }; a();",
      "var a = function(){ return function () { a(); } }; a();",
      "const a = () => { a(); }; a();",
      "const a = () => () => { a(); }; a();",
      r#"export * as ns from "source""#,
      "import.meta",
      "
import { ClassDecoratorFactory } from 'decorators';
@ClassDecoratorFactory()
export class Foo {}
      ",
      "
import { ClassDecorator } from 'decorators';
@ClassDecorator
export class Foo {}
      ",
      "
import { AccessorDecoratorFactory } from 'decorators';
export class Foo {
  @AccessorDecoratorFactory(true)
  get bar() {}
}
      ",
      "
import { AccessorDecorator } from 'decorators';
export class Foo {
  @AccessorDecorator
  set bar() {}
}
      ",
      "
import { MethodDecoratorFactory } from 'decorators';
export class Foo {
  @MethodDecoratorFactory(false)
  bar() {}
}
      ",
      "
import { MethodDecorator } from 'decorators';
export class Foo {
  @MethodDecorator
  static bar() {}
}
      ",
      "
import { ConstructorParameterDecoratorFactory } from 'decorators';
export class Service {
  constructor(
    @ConstructorParameterDecoratorFactory(APP_CONFIG) config: AppConfig,
  ) {
    this.title = config.title;
  }
}
      ",
      "
import { ConstructorParameterDecorator } from 'decorators';
export class Foo {
  constructor(@ConstructorParameterDecorator bar) {
    this.bar = bar;
  }
}
      ",
      "
import { ParameterDecoratorFactory } from 'decorators';
export class Qux {
  bar(@ParameterDecoratorFactory(true) baz: number) {
    console.log(baz);
  }
}
      ",
      "
import { ParameterDecorator } from 'decorators';
export class Foo {
  static greet(@ParameterDecorator name: string) {
    return name;
  }
}
      ",
      "
import { Input, Output, EventEmitter } from 'decorators';
export class SomeComponent {
  @Input() data;
  @Output()
  click = new EventEmitter();
}
      ",
      "
import { configurable } from 'decorators';
export class A {
  @configurable(true) static prop1;
  @configurable(false)
  static prop2;
}
      ",
      "
import { foo, bar } from 'decorators';
export class B {
  @foo x;
  @bar
  y;
}
      ",
      "
interface Base {}
class Thing implements Base {}
new Thing();
      ",
      "
interface Base {}
const a: Base = {};
console.log(a);
      ",
      "
import { Foo } from 'foo';
function bar<T>() {}
bar<Foo>();
      ",
      "
import { Foo } from 'foo';
const bar = function <T>() {};
bar<Foo>();
      ",
      "
import { Foo } from 'foo';
const bar = <T>() => {};
bar<Foo>();
      ",
      "
import { Foo } from 'foo';
<Foo>(<T>() => {})();
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable<string> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<SomeOther> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable | undefined = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable & undefined = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<SomeOther[]> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<Array<SomeOther>> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Array<Nullable> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Nullable[] = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Array<Nullable[]> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
const a: Array<Array<Nullable>> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Array<Nullable<SomeOther>> = 'hello';
console.log(a);
      ",
      "
import { Nullable } from 'nullable';
import { Component } from 'react';
class Foo implements Component<Nullable> {}
new Foo();
      ",
      "
import { Nullable } from 'nullable';
import { Component } from 'react';
class Foo extends Component<Nullable, {}> {}
new Foo();
          ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component } from 'react';
class Foo extends Component<Nullable<SomeOther>, {}> {}
new Foo();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component } from 'react';
class Foo implements Component<Nullable<SomeOther>, {}> {}
new Foo();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Component, Component2 } from 'react';
class Foo implements Component<Nullable<SomeOther>, {}>, Component2 {}
new Foo();
      ",
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do = (a: Nullable<Another>) => {
    console.log(a);
  };
}
new A();
      ",
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(a: Nullable<Another>) {
    console.log(a);
  }
}
new A();
      ",
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(): Nullable<Another> {
    return null;
  }
}
new A();
      ",
      "
import { Nullable } from 'nullable';
function foo(a: Nullable) {
  console.log(a);
}
foo();
      ",
      "
import { Nullable } from 'nullable';
function foo(): Nullable {
  return null;
}
foo();
      ",
      "
import { Nullable } from 'nullable';
function foo(): Nullable {
  return null;
}
foo();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
class A extends Nullable<SomeOther> {
  do(a: Nullable<Another>) {
    console.log(a);
  }
}
new A();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
interface A extends Nullable<SomeOther> {
  other: Nullable<Another>;
}
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
interface A extends Nullable<SomeOther> {
  do(a: Nullable<Another>);
}
      ",
      "
import { Foo } from './types';
class Bar<T extends Foo> {}
new Bar<number>();
      ",
      "
import { Foo, Bar } from './types';
class Baz<T extends Foo & Bar> {}
new Baz<any>();
      ",
      "
import { Foo } from './types';
class Bar<T = Foo> {}
new Bar<number>();
      ",
      "
import { Foo } from './types';
class Foo<T = any> {}
new Foo();
      ",
      "
import { Foo } from './types';
class Foo<T = {}> {}
new Foo();
      ",
      "
import { Foo } from './types';
class Foo<T extends {} = {}> {}
new Foo();
      ",
      "
type Foo = 'a' | 'b' | 'c';
type Bar = number;
export const map: { [name in Foo]: Bar } = {
  a: 1,
  b: 2,
  c: 3,
};
      ",
      "
import { Nullable } from 'nullable';
class A<T> {
  bar: T;
}
new A<Nullable>();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
function foo<T extends Nullable>() {}
foo<SomeOther>();
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
class A<T extends Nullable> {
  bar: T;
}
new A<SomeOther>()
      ",
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
interface A<T extends Nullable> {
  bar: T;
}
export const a: A<SomeOther> = {
  foo: 'bar',
};

      ",
      "
import { Component, Vue } from 'vue-property-decorator';
import HelloWorld from './components/HelloWorld.vue';
@Component({
  components: {
    HelloWorld,
  },
})
export default class App extends Vue {}
      ",
      "
import firebase, { User } from 'firebase/app';
// initialize firebase project
firebase.initializeApp({});
export function authenticated(cb: (user: User | null) => void): void {
  firebase.auth().onAuthStateChanged(user => cb(user));
}
      ",
      "
import { Foo } from './types';
export class Bar<T extends Foo> {}
      ",
      "
import webpack from 'webpack';
export default function webpackLoader(this: webpack.loader.LoaderContext) {}
      ",
      "
import execa, { Options as ExecaOptions } from 'execa';
export function foo(options: ExecaOptions): execa {
  options();
}
      ",
      "
import { Foo, Bar } from './types';
export class Baz<F = Foo & Bar> {}
      ",
      "
// warning 'B' is defined but never used
export const a: Array<{ b: B }> = [];
      ",
      "
export enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
      ",
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
interface IFoo {
  fieldName: FormFieldIds;
}
      ",
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
interface IFoo {
  fieldName: FormFieldIds.EMAIL;
}
      ",
      "
import * as fastify from 'fastify';
import { Server, IncomingMessage, ServerResponse } from 'http';
const server: fastify.FastifyInstance<
  Server,
  IncomingMessage,
  ServerResponse
> = fastify({});
server.get('/ping');
      ",
      "
declare function foo();
      ",
      "
declare namespace Foo {
  function bar(line: string, index: number | null, tabSize: number): number;
  var baz: string;
}
      ",
      "
declare var Foo: {
  new (value?: any): Object;
  foo(): string;
};
      ",
      "
declare class Foo {
  constructor(value?: any): Object;
  foo(): string;
}
      ",

      // https://github.com/denoland/deno_lint/issues/670
      "export declare class Foo { constructor(arg: string); }",
      "export declare function foo(): void;",
      "export declare const foo: number;",
      "export declare let foo: number;",
      "export declare var foo: number;",

      "
import foo from 'foo';
export interface Bar extends foo.i18n {}
      ",
      "
import foo from 'foo';
import bar from 'foo';
export interface Bar extends foo.i18n<bar> {}
      ",
      "
import { observable } from 'mobx';
export default class ListModalStore {
  @observable
  orderList: IObservableArray<BizPurchaseOrderTO> = observable([]);
}
      ",
      "
import { Dec, TypeA, Class } from 'test';
export default class Foo {
  constructor(
    @Dec(Class)
    private readonly prop: TypeA<Class>,
  ) {}
}
      ",
      "
import { Dec, TypeA, Class } from 'test';
export default class Foo {
  constructor(
    @Dec(Class)
    ...prop: TypeA<Class>
  ) {
    prop();
  }
}
      ",
      "export function foo(msg: string): void",
      "export default function foo(msg: string): void",
      "function _foo(msg?: string): void",
      "const key = 0; export const obj = { [key]: true };",
      "export class Foo { bar(msg: string): void; }",
      "import { foo } from './foo.ts'; type Bar = typeof foo;",
      "interface Foo {} export interface Bar extends Foo {}",
      "import type Foo from './foo.ts'; export interface Bar extends Foo {}",
      "import type Foo from './foo.ts'; export class Bar implements Foo {}",
      "import type Foo from './foo.ts'; interface _Bar<T extends Foo> {}",
      "import type Foo from './foo.ts'; type _Bar<T extends Foo> = T;",
      "type Foo = { a: number }; function _bar<T extends keyof Foo>() {}",

      // https://github.com/denoland/deno_lint/issues/667#issuecomment-821856328
      // `this` as a fake parameter. See: https://www.typescriptlang.org/docs/handbook/functions.html#this-parameters
      "export function f(this: void) {}",
      "export const foo = { bar(this: Foo) {} };",
      "export interface Foo { bar(this: void): void; }",
      "export interface Foo { bar(baz: (this: void) => void ): void; }",
      "export class Foo { bar(this: Foo) {} }",
      r#"
export abstract class Point4DPartial {
    toString(this: Point4D): string {
      return [this.getPosition(), this.z, this.getTime()].join(", ");
    }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/667
      "const i = setInterval(() => clearInterval(i), 1000);",
      "const i = setInterval(function() { clearInterval(i); }, 1000);",
      "const i = setInterval(function foo() { clearInterval(i); }, 1000);",
      "setTimeout(function foo() { const foo = 42; console.log(foo); });",
      "const fn = function foo() {}; fn();",

      // https://github.com/denoland/deno_lint/issues/687
      "const { foo, bar = foo } = { foo: 42 }; console.log(bar);",
      "const { foo, bar = f(foo) } = makeObj(); console.log(bar);",
      "const { foo, bar = foo.prop } = makeObj(); console.log(bar);",
      "const { foo, bar = await foo } = makeObj(); console.log(bar);",
      "const { foo, bar = foo ? 42 : 7 } = makeObj(); console.log(bar);",
      "const { foo, bar = baz ? foo : 7 } = makeObj(); console.log(bar);",
      "const { foo, bar = `hello ${foo}` } = makeObj(); console.log(bar);",
      "const { foo, bar = [foo] } = makeObj(); console.log(bar);",
      "const { foo, bar = { key: foo } } = makeObj(); console.log(bar);",
      "const { foo, bar = function() { foo(); } } = makeObj(); console.log(bar);",
      "const { foo, bar = () => foo() } = makeObj(); console.log(bar);",

      // https://github.com/denoland/deno_lint/issues/690
      r#"
export class Foo {
  constructor(x: number);
  constructor(y: string);
  constructor(xy: number | string) {
    console.log(xy);
  }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/705
      r#"
type Foo = string[];
function _bar(...Foo: Foo): void {
  console.log(Foo);
}
      "#,
      r#"
import type { Foo } from "./foo.ts";
function _bar(...Foo: Foo) {
  console.log(Foo);
}
      "#,
      r#"
import type { Filters } from "./types.ts";
export function audioFilters(...Filters: Filters[]): void {
  for (const filter of Filters) {
    console.log(filter);
    return;
  }
}
      "#,

      // https://github.com/denoland/deno_lint/issues/739
      r#"
export class Test {
  #myFunction(value: string): string;
  #myFunction(value: number): number;
  #myFunction(value: string | number) {
    return value;
  }
}
      "#,
      "
import * as deps from './deps.ts';
import MyTest = deps.SubNamespace.MyTest;
console.log(MyTest);
      ",
      "
import * as deps from './deps.ts';
import MyDeps = deps;
console.log(MyDeps);
      "
    };

    // JSX or TSX
    assert_lint_ok! {
      NoUnusedVars,
      filename: "file:///foo.tsx",
      r#"
import { TypeA } from './interface';
export const a = <GenericComponent<TypeA> />;
      "#,
      r#"
const text = 'text';
export function Foo() {
  return (
    <div id="hoge">
      <input type="search" size={30} placeholder={text} />
    </div>
  );
}
      "#,
      r#"
function Root() { return null; }
function Child() { return null; }
export default <Root><Child>Hello World!</Child></Root>;
      "#,

      // https://github.com/denoland/deno_lint/issues/663
      r#"
import React from "./dummy.ts";
export default <div />;
      "#,
      r#"
import React from "./dummy.ts";
function Component() { return null; }
export default <Component />;
      "#,
      r#"
import React from "./dummy.ts";
const Component = () => { return null; }
export default <Component />;
      "#,
      r#"
import React from "./dummy.ts";
class Component extends React.Component { render() { return null; } }
export default <Component />;
      "#,

      r#"
/** @jsx h */
import { h } from "./dummy.ts";
export default <foo />;
      "#,
      r#"
/** @jsxFrag Fragment */
import { Fragment } from "./dummy.ts";
export default <></>;
      "#,
    };
  }

  #[test]
  fn no_unused_vars_invalid() {
    assert_lint_err! {
      NoUnusedVars,
      "var a = 0": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      // variable shadowing
      "var a = 1; function foo() { var a = 2; console.log(a); }; use(foo);": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foox() { return foox(); }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foox"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foox"),
        }
      ],
      "class Foo {}": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "Foo"),
        }
      ],
      "(function() { function foox() { if (true) { return foox(); } } }())": [
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foox"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foox"),
        }
      ],
      "function f() { var a = 1; return function(){ f(a *= 2); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function f() { var a = 1; return function(){ f(++a); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function f() { var a = { prop: 1 }; return function(){ f(a.prop *= 2); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function foo(first, second) {\ndoStuff(function()\
       {\nconsole.log(second);});};": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        },
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "first"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "first"),
        }
      ],
      "var a=10; a=20;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a=10; (function() { var a = 1; alert(a); })();": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a=10, b=0, c=null; alert(a+b)": [
        {
          col: 15,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "var a=10, b=0, c=null; setTimeout(function() { var b=2; alert(a+b+c); }, 0);": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "var a=10, b=0, c=null; setTimeout(function() \
      { var b=2; var c=2; alert(a+b+c); }, 0);": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        },
        {
          col: 15,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "function f(){var a=[];return a.map(function(){});}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function f(){var a=[];return a.map(function g(){});}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        }
      ],
      "function f(){var x;function a(){x=42;}function b(){alert(x);}}": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        },
        {
          col: 28,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        },
        {
          col: 47,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "function f(a) {}; f();": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function a(x, y, z){ return y; }; a();": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        },
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "z"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "z"),
        }
      ],
      "var min = Math.min": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "min"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "min"),
        }
      ],
      "Foo.bar = function(baz) { return 1; };": [
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "baz"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "baz"),
        }
      ],
      "var min = {min: 1}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "min"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "min"),
        }
      ],
      "function gg(baz, bar) { return baz; }; gg();": [
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "bar"),
        }
      ],
      "(function(foo, baz, bar) { return baz; })();": [
        {
          col: 10,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        },
        {
          col: 20,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "bar"),
        }
      ],
      "(function z(foo) { var bar = 33; })();": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        },
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "bar"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "bar"),
        }
      ],
      "(function z(foo) { z(); })();": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        }
      ],
      "function f() { var a = 1; return function(){ f(a = 2); }; }": [
        {
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "f"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "f"),
        },
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "import x from \"y\";": [
        {
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "export function fn2({ x, y }) {\n console.log(x); \n};": [
        {
          col: 25,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "y"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "y"),
        }
      ],
      "export function fn2( x, y ) {\n console.log(x); \n};": [
        {
          col: 24,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "y"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "y"),
        }
      ],
      "var _a; var b;": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "function foo(a, _b) { } foo()": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a, _b, c) { return a; } foo();": [
        {
          col: 20,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "const data = { type: 'coords', x: 1, y: 2 };\
     const { type, ...coords } = data;\n console.log(coords);": [
        {
          col: 52,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "type"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "type"),
        }
      ],
      "const data = { type: 'coords', x: 3, y: 2 };\
        const { type, ...coords } = data;\n console.log(type)": [
        {
          col: 61,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "coords"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "coords"),
        }
      ],
      "const data = { vars: \
      ['x','y'], x: 1, y: 2 }; const { vars: [x], ...coords } = data;\n\
       console.log(coords)": [
        {
          col: 61,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "const data = { defaults: { x: 0 }, x: 1, y: 2 }; const { defaults: { x }, ...coords } = data;\n console.log(coords)": [
        {
          col: 69,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "export default function(a) {}": [
        {
          col: 24,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "export default function(a, b) { console.log(a); }": [
        {
          col: 27,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "export default (function(a) {});": [
        {
          col: 25,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "export default (function(a, b) { console.log(a); });": [
        {
          col: 28,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "export default (a) => {};": [
        {
          col: 16,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "export default (a, b) => { console.log(a); };": [
        {
          col: 19,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "b"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "b"),
        }
      ],
      "try{}catch(err){};": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "err"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "err"),
        }
      ],
      "(function ({ a }, b ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "(function ({ a }, { b, c } ) { return b; })();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        },
        {
          col: 23,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "var a = function() { a(); };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = function(){ return function() { a(); } };": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "const a = () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "const a = () => () => { a(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "let a = 'a';
    a = 10;
    function foo(){
        a = 11;
        a = () => {
            a = 13
        }
    }": [
        {
          line: 1,
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        },
        {
          line: 3,
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        }
      ],
      "let c = 'c'
    c = 10
    function foo1() {
      c = 11
      c = () => {
        c = 13
      }
    }
    c = foo1": [
        {
          line: 1,
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "c"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "c"),
        }
      ],
      "
import { ClassDecoratorFactory } from 'decorators';
export class Foo {}
      ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "ClassDecoratorFactory"),
          hint: variant!(NoUnusedVarsHint, Alias, "ClassDecoratorFactory"),
        }
      ],
      "
import { Foo, Bar } from 'foo';
function baz<Foo>() {}
baz<Bar>();
      ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Foo"),
          hint: variant!(NoUnusedVarsHint, Alias, "Foo"),
        }
      ],
      "
import { Nullable } from 'nullable';
const a: string = 'hello';
console.log(a);
      ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Nullable"),
          hint: variant!(NoUnusedVarsHint, Alias, "Nullable"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'other';
const a: Nullable<string> = 'hello';
console.log(a);
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "SomeOther"),
          hint: variant!(NoUnusedVarsHint, Alias, "SomeOther"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do = (a: Nullable) => {
    console.log(a);
  };
}
new A();
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(a: Nullable) {
    console.log(a);
  }
}
new A();
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
class A {
  do(): Nullable {
    return null;
  }
}
new A();
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
interface A {
  do(a: Nullable);
}
      ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { Another } from 'some';
interface A {
  other: Nullable;
}
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Another"),
          hint: variant!(NoUnusedVarsHint, Alias, "Another"),
        }
      ],
      "
import { Nullable } from 'nullable';
function foo(a: string) {
  console.log(a);
}
foo();
        ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Nullable"),
          hint: variant!(NoUnusedVarsHint, Alias, "Nullable"),
        }
      ],
      "
import { Nullable } from 'nullable';
function foo(): string | null {
  return null;
}
foo();
        ": [
        {
          line: 2,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "Nullable"),
          hint: variant!(NoUnusedVarsHint, Alias, "Nullable"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
class A extends Nullable {
  other: Nullable<Another>;
}
new A();
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "SomeOther"),
          hint: variant!(NoUnusedVarsHint, Alias, "SomeOther"),
        }
      ],
      "
import { Nullable } from 'nullable';
import { SomeOther } from 'some';
import { Another } from 'some';
abstract class A extends Nullable {
  other: Nullable<Another>;
}
new A();
        ": [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "SomeOther"),
          hint: variant!(NoUnusedVarsHint, Alias, "SomeOther"),
        }
      ],
      "
enum FormFieldIds {
  PHONE = 'phone',
  EMAIL = 'email',
}
        ": [
        {
          line: 2,
          col: 5,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "FormFieldIds"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "FormFieldIds"),
        }
      ],
      "
import test from 'test';
import baz from 'baz';
export interface Bar extends baz.test {}
        ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
        }
      ],
      "
import React from './dummy.ts';
const a = 42;
foo(a);
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "React"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "React"),
        }
      ],

      // https://github.com/denoland/deno_lint/issues/730
      r#"import * as foo from "./foo.ts";"#: [
        {
          line: 1,
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        }
      ],

      // FnExpr
      "const fn = function foo() { foo(); };": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "fn"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "fn"),
        }
      ],

      // https://github.com/denoland/deno_lint/issues/697
      "for (const x of xs) {}": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "let x; for (x of xs) {}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "for await (const x of xs) {}": [
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "let x; for await (x of xs) {}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "for (const x in xs) {}": [
        {
          col: 11,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "let x; for (x in xs) {}": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x"),
        }
      ],
      "for (const [x1, x2] of xs) {}": [
        {
          col: 12,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x1"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x1"),
        },
        {
          col: 16,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x2"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x2"),
        }
      ],
      "let x1, x2; for ([x1, x2] of xs) { console.log(x1); }": [
        {
          col: 8,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x2"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x2"),
        },
      ],
      "for (const { x1, x2 } of xs) {}": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x1"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x1"),
        },
        {
          col: 17,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x2"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x2"),
        }
      ],
      "let x1, x2; for ({ x1, x2 } of xs) { console.log(x2); }": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "x1"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "x1"),
        },
      ],

        r#"
export class Foo {
  #myFunction(value: string): string;
  #myFunction(value: number): number;
  #myFunction(value: string | number) {
    return 42;
  }
}
        "#: [
          {
            col: 14,
            line:5,
            message: variant!(NoUnusedVarsMessage, NeverUsed, "value"),
            hint: variant!(NoUnusedVarsHint, AddPrefix, "value"),
          },
        ],
        "import * as deps from './test.js';\nimport Test = deps.test;": [
          {
            col: 7,
            line: 2,
            message: variant!(NoUnusedVarsMessage, NeverUsed, "Test"),
            hint: variant!(NoUnusedVarsHint, AddPrefix, "Test"),
          },
        ],
    };

    // jsx/tsx
    assert_lint_err! {
      NoUnusedVars,
      filename: "file:///foo.tsx",
      r#"
import React from 'react';
export const Foo = () => {
  return "string";
}"#: [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "React"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "React"),
        }
      ],
      r#"
/** @jsx h */ /** @jsxFrag Fragment */
import { h, Fragment } from "preact";
export const Foo = () => {
  return <></>;
}"#: [
        {
          line: 3,
          col: 9,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "h"),
          hint: variant!(NoUnusedVarsHint, Alias, "h"),
        }
      ]
    }
  }

  // TODO(magurotuna): deals with this using ControlFlow
  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_for_loop_control_flow() {
    assert_lint_err! {
      NoUnusedVars,
      "(function(obj) { var name; for ( name in obj ) { i(); return; } })({});": [
        {
          col: 21,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "name"),
        }
      ],
      "(function(obj) { var name; for ( name in obj ) { } })({});": [
        {
          col: 21,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "name"),
        }
      ],
      "(function(obj) { for ( var name in obj ) { } })({});": [
        {
          col: 37,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "name"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "name"),
        }
      ]
    };
  }

  #[test]
  fn no_unused_vars_static_init_is_usage() {
    assert_lint_err! {
      NoUnusedVars,
      "let foo: Foo;
class Foo {
  instance;
  static {
    foo = new Foo();
  }
}
": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "foo"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "foo"),
        }
      ],
    };
  }

  // TODO(magurotuna): deals with this using ControlFlow
  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_assign_expr() {
    assert_lint_err! {
      NoUnusedVars,
      "var a = 0; a = a + 1;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 0; a = a + a;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 0; a += a + 1": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 0; a++;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a) { a = a + 1 } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a) { a += a + 1 } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "function foo(a) { a++ } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 3; a = a * 5 + 6;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "var a = 2, b = 4; a = a * 2 + b;": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ],
      "const a = 1; a += 1;": [
        {
          col: 6,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "a"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "a"),
        }
      ]
    };
  }

  // TODO(magurotuna): deals with this using ControlFlow
  #[test]
  #[ignore = "control flow analysis is not implemented yet"]
  fn no_unused_vars_err_assign_to_self() {
    assert_lint_err! {
      NoUnusedVars,
      "function foo(cb) { cb = function(a) { cb(1 + a); }; bar(not_cb); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
        }
      ],
      "function foo(cb) { cb = function(a) { return cb(1 + a); }(); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
        }
      ],
      "function foo(cb) { cb = (function(a) { cb(1 + a); }, cb); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
        }
      ],
      "function foo(cb) { cb = (0, function(a) { cb(1 + a); }); } foo();": [
        {
          col: 13,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "cb"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "cb"),
        }
      ]
    };
  }

  #[test]
  #[ignore = "pure method analysis is not implemented yet"]
  fn no_unused_vars_err_array_methods() {
    assert_lint_err! {
      NoUnusedVars,
      "let myArray = [1,2,3,4].filter((x) => x == 0); myArray = myArray.filter((x) => x == 1);": [
        {
          col: 4,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "myArray"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "myArray"),
        }
      ]
    };
  }

  #[test]
  #[ignore = "swc cannot parse this at the moment"]
  fn no_unused_vars_ts_err_06() {
    assert_lint_err! {
      NoUnusedVars,
      "
import test from 'test';
import baz from 'baz';
export interface Bar extends baz().test {}
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
        }
      ],
      "
import test from 'test';
import baz from 'baz';
export class Bar implements baz.test {}
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
        }
      ],
      "
import test from 'test';
import baz from 'baz';
export class Bar implements baz().test {}
      ": [
        {
          line: 2,
          col: 7,
          message: variant!(NoUnusedVarsMessage, NeverUsed, "test"),
          hint: variant!(NoUnusedVarsHint, AddPrefix, "test"),
        }
      ],
    };
  }

  #[test]
  #[ignore = "typescript property analysis is not implemented yet"]
  fn no_unused_vars_ts_ok_12() {
    assert_lint_ok! {
      NoUnusedVars,
      "
export class App {
  constructor(private logger: Logger) {
    console.log(this.logger);
  }
}
      ",
      "
export class App {
  constructor(bar: string);
  constructor(private logger: Logger) {
    console.log(this.logger);
  }
}
      ",
      "
export class App {
  constructor(baz: string, private logger: Logger) {
    console.log(baz);
    console.log(this.logger);
  }
}
      ",
      "
export class App {
  constructor(baz: string, private logger: Logger, private bar: () => void) {
    console.log(this.logger);
    this.bar();
  }
}
      ",
      "
export class App {
  constructor(private logger: Logger) {}
  meth() {
    console.log(this.logger);
  }
}
      ",
    };
  }
}
