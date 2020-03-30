// Copyright 2020 the Deno authors. All rights reserved. MIT license.
#![allow(unused)]

use swc_ecma_ast::*;

pub trait AstTraverser {
  fn walk_program(&self, program: Program) {
    match program {
      Program::Module(module) => self.walk_module(module),
      Program::Script(script) => self.walk_script(script),
    }
  }

  fn walk_module(&self, module: Module) {
    self.walk_module_items(module.body)
  }

  fn walk_script(&self, script: Script) {
    self.walk_statements(script.body)
  }

  fn walk_module_items(&self, module_items: Vec<ModuleItem>) {
    for module_item in module_items {
      self.walk_module_item(module_item)
    }
  }

  fn walk_module_item(&self, module_item: ModuleItem) {
    match module_item {
      ModuleItem::ModuleDecl(module_decl) => self.walk_module_decl(module_decl),
      ModuleItem::Stmt(stmt) => self.walk_statement(stmt),
    }
  }

  fn walk_module_decl(&self, module_decl: ModuleDecl) {
    match module_decl {
      ModuleDecl::Import(import_decl) => self.walk_import_decl(import_decl),
      ModuleDecl::ExportDecl(export_decl) => self.walk_export_decl(export_decl),
      ModuleDecl::ExportNamed(named_export) => {
        self.walk_named_export(named_export)
      }
      ModuleDecl::ExportDefaultDecl(export_default_decl) => {
        self.walk_export_default_decl(export_default_decl)
      }
      ModuleDecl::ExportDefaultExpr(export_default_expr) => {
        self.walk_export_default_expr(export_default_expr)
      }
      ModuleDecl::ExportAll(export_all) => self.walk_export_all(export_all),
      ModuleDecl::TsImportEquals(ts_import_equals_decl) => {
        self.walk_ts_import_equals_decl(ts_import_equals_decl)
      }
      ModuleDecl::TsExportAssignment(ts_export_assignment) => {
        self.walk_ts_export_assignment(ts_export_assignment)
      }
      ModuleDecl::TsNamespaceExport(ts_namespace_export_decl) => {
        self.walk_ts_namespace_export_decl(ts_namespace_export_decl)
      }
    }
  }

  fn walk_import_decl(&self, import_decl: ImportDecl) {
    self.walk_string_literal(import_decl.src);
    self.walk_import_specifiers(import_decl.specifiers);
  }

  fn walk_import_specifiers(&self, import_specifiers: Vec<ImportSpecifier>) {
    for specifier in import_specifiers {
      self.walk_import_specifier(specifier);
    }
  }

  fn walk_import_specifier(&self, import_specifier: ImportSpecifier) {
    match import_specifier {
      ImportSpecifier::Specific(import_specific) => {
        self.walk_named_import_specifier(import_specific)
      }
      ImportSpecifier::Default(import_default) => {
        self.walk_import_default_specifier(import_default)
      }
      ImportSpecifier::Namespace(import_as) => {
        self.walk_import_namespace_specifier(import_as)
      }
    }
  }

  fn walk_named_import_specifier(&self, named_import: ImportSpecific) {
    self.walk_binding_identifier(named_import.local);
    if let Some(imp) = named_import.imported {
      self.walk_identifier_reference(imp);
    }
  }

  fn walk_import_namespace_specifier(&self, import_as: ImportStarAs) {
    self.walk_binding_identifier(import_as.local);
  }

  fn walk_import_default_specifier(&self, import_default: ImportDefault) {
    self.walk_binding_identifier(import_default.local);
  }

  fn walk_export_decl(&self, export_decl: ExportDecl) {
    self.walk_decl(export_decl.decl)
  }

  fn walk_named_export(&self, named_export: NamedExport) {
    self.walk_export_specifiers(named_export.specifiers);
    self.walk_optional_string_literal(named_export.src);
  }

  fn walk_export_specifiers(&self, export_specifiers: Vec<ExportSpecifier>) {
    for specifier in export_specifiers {
      self.walk_export_specifier(specifier);
    }
  }

  fn walk_export_specifier(&self, export_specifier: ExportSpecifier) {
    match export_specifier {
      ExportSpecifier::Default(default_export_specifier) => {
        self.walk_export_default_specifier(default_export_specifier)
      }
      ExportSpecifier::Namespace(ns_export_specifier) => {
        self.walk_export_namespace_specifier(ns_export_specifier)
      }
      ExportSpecifier::Named(named_export_specifier) => {
        self.walk_named_export_specifier(named_export_specifier)
      }
    }
  }

  fn walk_export_default_specifier(
    &self,
    default_export_specifier: DefaultExportSpecifier,
  ) {
    self.walk_binding_identifier(default_export_specifier.exported)
  }

  fn walk_export_namespace_specifier(
    &self,
    ns_export_specifier: NamespaceExportSpecifier,
  ) {
    self.walk_binding_identifier(ns_export_specifier.name);
  }

  fn walk_named_export_specifier(
    &self,
    named_export_specifier: NamedExportSpecifier,
  ) {
    if let Some(exp) = named_export_specifier.exported {
      self.walk_binding_identifier(exp);
    }
    self.walk_identifier_reference(named_export_specifier.orig)
  }

  fn walk_optional_string_literal(&self, optional_literal: Option<Str>) {
    if let Some(literal) = optional_literal {
      self.walk_string_literal(literal)
    }
  }

  fn walk_binding_identifier(&self, ident: Ident) {
    self.walk_identifier(ident)
  }

  fn walk_identifier_reference(&self, ident: Ident) {
    self.walk_identifier(ident)
  }

  fn walk_identifier(&self, ident: Ident) {
    if let Some(type_ann) = ident.type_ann {
      self.walk_ts_type_ann(type_ann);
    }
  }

  fn walk_string_literal(&self, str_literal: Str) {}

  fn walk_num_literal(&self, number_literal: Number) {}

  fn walk_export_default_decl(&self, export_default_decl: ExportDefaultDecl) {
    match export_default_decl.decl {
      DefaultDecl::Class(class_expr) => self.walk_class_expr(class_expr),
      DefaultDecl::Fn(fn_expr) => self.walk_fn_expr(fn_expr),
      DefaultDecl::TsInterfaceDecl(ts_interface_decl) => {
        self.walk_ts_interface_decl(ts_interface_decl);
      }
    }
  }

  fn walk_export_default_expr(&self, export_default_expr: ExportDefaultExpr) {
    self.walk_expression(export_default_expr.expr);
  }

  fn walk_export_all(&self, export_all: ExportAll) {
    self.walk_string_literal(export_all.src);
  }

  fn walk_ts_import_equals_decl(
    &self,
    ts_import_equals_decl: TsImportEqualsDecl,
  ) {
    self.walk_binding_identifier(ts_import_equals_decl.id);
    self.walk_ts_module_reference(ts_import_equals_decl.module_ref);
  }

  fn walk_ts_module_reference(&self, ts_module_ref: TsModuleRef) {}

  fn walk_ts_export_assignment(
    &self,
    ts_export_assignment: TsExportAssignment,
  ) {
    self.walk_expression(ts_export_assignment.expr);
  }

  fn walk_ts_namespace_export_decl(
    &self,
    ts_namespace_export_decl: TsNamespaceExportDecl,
  ) {
    self.walk_binding_identifier(ts_namespace_export_decl.id);
  }

  fn walk_statements(&self, stmts: Vec<Stmt>) {
    for stmt in stmts {
      self.walk_statement(stmt);
    }
  }

  fn walk_expression(&self, expr: Box<Expr>) {
    match *expr {
      Expr::Array(array_lit) => self.walk_array_lit(array_lit),
      Expr::Arrow(arrow_expr) => self.walk_arrow_expr(arrow_expr),
      Expr::Assign(assign_expr) => self.walk_assign_expr(assign_expr),
      Expr::Await(await_expr) => self.walk_await_expr(await_expr),
      Expr::Bin(bin_expr) => self.walk_bin_expr(bin_expr),
      Expr::Call(call_expr) => self.walk_call_expr(call_expr),
      Expr::Class(class_expr) => self.walk_class_expr(class_expr),
      Expr::Cond(cond_expr) => self.walk_cond_expr(cond_expr),
      Expr::Fn(fn_expr) => self.walk_fn_expr(fn_expr),
      Expr::Ident(ident) => self.walk_identifier_reference(ident),
      Expr::Invalid(_) => {}
      Expr::JSXMember(jsx_member_expr) => {
        self.walk_jsx_member_expr(jsx_member_expr)
      }
      Expr::JSXNamespacedName(jsx_namespaced_name) => {
        self.walk_jsx_namespaced_name(jsx_namespaced_name)
      }
      Expr::JSXEmpty(jsx_empty_expr) => self.walk_jsx_empty(jsx_empty_expr),
      Expr::JSXElement(jsx_element) => self.walk_jsx_element(jsx_element),
      Expr::JSXFragment(jsx_fragment) => self.walk_jsx_fragment(jsx_fragment),
      Expr::Member(member_expr) => self.walk_member_expr(member_expr),
      Expr::MetaProp(meta_prop_expr) => {
        self.walk_meta_prop_expr(meta_prop_expr)
      }
      Expr::New(new_expr) => self.walk_new_expr(new_expr),
      Expr::Lit(lit) => self.walk_lit(lit),
      Expr::Object(object_lit) => self.walk_object_lit(object_lit),
      Expr::OptChain(opt_chain_expr) => {
        self.walk_opt_chain_expr(opt_chain_expr)
      }
      Expr::Paren(paren_expr) => self.walk_paren_expr(paren_expr),
      Expr::PrivateName(private_name) => self.walk_private_name(private_name),
      Expr::Seq(seq_expr) => self.walk_seq_expr(seq_expr),
      Expr::TaggedTpl(tagged_tpl) => self.walk_tagged_tpl(tagged_tpl),
      Expr::This(this_expr) => self.walk_this_expr(this_expr),
      Expr::Tpl(tpl) => self.walk_tpl(tpl),
      Expr::TsTypeAssertion(ts_type_assertion) => {
        self.walk_ts_type_assertion(ts_type_assertion)
      }
      Expr::TsConstAssertion(ts_const_assertion) => {
        self.walk_ts_const_assertion(ts_const_assertion)
      }
      Expr::TsNonNull(ts_non_null_expr) => {
        self.walk_ts_non_null_expr(ts_non_null_expr)
      }
      Expr::TsTypeCast(ts_type_cast_expr) => {
        self.walk_ts_type_cast_expr(ts_type_cast_expr)
      }
      Expr::TsAs(ts_as_expr) => self.walk_ts_as_expr(ts_as_expr),
      Expr::Unary(unary_expr) => self.walk_unary_expr(unary_expr),
      Expr::Update(update_expr) => self.walk_update_expr(update_expr),
      Expr::Yield(yield_expr) => self.walk_yield_expr(yield_expr),
    }
  }

  fn walk_array_lit(&self, array_lit: ArrayLit) {
    for expr_or_spread in array_lit.elems {
      if let Some(expr_or_spread) = expr_or_spread {
        self.walk_expression(expr_or_spread.expr);
      }
    }
  }

  // TODO: deal with type_params
  fn walk_arrow_expr(&self, arrow_expr: ArrowExpr) {
    match arrow_expr.body {
      BlockStmtOrExpr::BlockStmt(stmt) => self.walk_block_stmt(stmt),
      BlockStmtOrExpr::Expr(expr) => self.walk_expression(expr),
    }
    self.walk_patterns(arrow_expr.params);
    if let Some(type_ann) = arrow_expr.return_type {
      self.walk_ts_type_ann(type_ann);
    }
  }

  fn walk_assign_expr(&self, assign_expr: AssignExpr) {
    match assign_expr.left {
      PatOrExpr::Pat(pat) => self.walk_pattern(*pat),
      PatOrExpr::Expr(expr) => self.walk_expression(expr),
    }
    self.walk_expression(assign_expr.right);
  }

  fn walk_await_expr(&self, await_expr: AwaitExpr) {
    self.walk_expression(await_expr.arg);
  }

  fn walk_bin_expr(&self, bin_expr: BinExpr) {
    self.walk_expression(bin_expr.left);
    self.walk_expression(bin_expr.right);
  }

  // TODO: deal with type_args
  fn walk_call_expr(&self, call_expr: CallExpr) {
    for arg in call_expr.args {
      self.walk_expression(arg.expr);
    }
    match call_expr.callee {
      ExprOrSuper::Expr(expr) => self.walk_expression(expr),
      ExprOrSuper::Super(super_) => {}
    }
  }

  fn walk_class_expr(&self, class_expr: ClassExpr) {
    if let Some(ident) = class_expr.ident {
      self.walk_identifier(ident);
    }
    // TODO: deal with actual class
  }

  fn walk_cond_expr(&self, cond_expr: CondExpr) {
    self.walk_expression(cond_expr.test);
    self.walk_expression(cond_expr.alt);
    self.walk_expression(cond_expr.cons);
  }

  fn walk_fn_expr(&self, fn_expr: FnExpr) {
    if let Some(ident) = fn_expr.ident {
      self.walk_identifier(ident);
    }
    self.walk_function(fn_expr.function);
  }

  fn walk_jsx_member_expr(&self, jsx_member_expr: JSXMemberExpr) {
    self.walk_identifier(jsx_member_expr.prop);
    match jsx_member_expr.obj {
      JSXObject::JSXMemberExpr(jsx_member_expr) => {
        self.walk_jsx_member_expr(*jsx_member_expr)
      }
      JSXObject::Ident(ident) => self.walk_identifier(ident),
    }
  }

  fn walk_jsx_namespaced_name(&self, jsx_namespaced_name: JSXNamespacedName) {
    self.walk_identifier(jsx_namespaced_name.name);
    self.walk_identifier(jsx_namespaced_name.ns);
  }

  fn walk_jsx_empty(&self, jsx_empty: JSXEmptyExpr) {}

  fn walk_jsx_element(&self, jsx_element: Box<JSXElement>) {
    // TODO: deal with this
  }

  fn walk_jsx_fragment(&self, jsx_fragment: JSXFragment) {
    // TODO: deal with this
  }

  fn walk_member_expr(&self, member_expr: MemberExpr) {
    self.walk_expression(member_expr.prop);
    match member_expr.obj {
      ExprOrSuper::Expr(expr) => self.walk_expression(expr),
      ExprOrSuper::Super(super_) => {}
    }
  }

  fn walk_meta_prop_expr(&self, meta_prop_expr: MetaPropExpr) {
    self.walk_identifier(meta_prop_expr.meta);
    self.walk_identifier(meta_prop_expr.prop);
  }

  fn walk_new_expr(&self, new_expr: NewExpr) {
    self.walk_expression(new_expr.callee);
    if let Some(args) = new_expr.args {
      for expr_or_spread in args {
        self.walk_expression(expr_or_spread.expr);
      }
    }
    // TODO: deal with type_args
  }

  fn walk_lit(&self, lit: Lit) {}

  fn walk_object_lit(&self, object_lit: ObjectLit) {
    for prop_or_spread in object_lit.props {
      match prop_or_spread {
        PropOrSpread::Prop(prop) => self.walk_prop(*prop),
        PropOrSpread::Spread(spread) => self.walk_expression(spread.expr),
      };
    }
  }

  fn walk_opt_chain_expr(&self, opt_chain_expr: OptChainExpr) {
    self.walk_expression(opt_chain_expr.expr)
  }

  fn walk_paren_expr(&self, parent_expr: ParenExpr) {
    self.walk_expression(parent_expr.expr)
  }

  fn walk_private_name(&self, private_name: PrivateName) {
    self.walk_identifier(private_name.id);
  }

  fn walk_seq_expr(&self, seq_expr: SeqExpr) {
    for expr in seq_expr.exprs {
      self.walk_expression(expr);
    }
  }

  fn walk_tagged_tpl(&self, tagged_tpl: TaggedTpl) {
    self.walk_expression(tagged_tpl.tag);
    for expr in tagged_tpl.exprs {
      self.walk_expression(expr);
    }
    // TODO: deal with quasis?
  }

  fn walk_this_expr(&self, this_expr: ThisExpr) {}

  fn walk_tpl(&self, tpl: Tpl) {
    for expr in tpl.exprs {
      self.walk_expression(expr);
    }
    // TODO: deal with quasis?
  }

  fn walk_ts_type_assertion(&self, type_assertion: TsTypeAssertion) {
    self.walk_expression(type_assertion.expr);
    self.walk_ts_type(*type_assertion.type_ann)
  }

  fn walk_ts_const_assertion(&self, const_assertion: TsConstAssertion) {
    self.walk_expression(const_assertion.expr);
  }

  fn walk_ts_non_null_expr(&self, non_null_expr: TsNonNullExpr) {
    self.walk_expression(non_null_expr.expr);
  }

  fn walk_ts_type_cast_expr(&self, type_cast_expr: TsTypeCastExpr) {
    self.walk_expression(type_cast_expr.expr);
    self.walk_ts_type_ann(type_cast_expr.type_ann);
  }

  fn walk_ts_as_expr(&self, as_expr: TsAsExpr) {
    self.walk_expression(as_expr.expr);
    self.walk_ts_type(*as_expr.type_ann)
  }

  fn walk_unary_expr(&self, unary_expr: UnaryExpr) {
    self.walk_expression(unary_expr.arg);
  }

  fn walk_update_expr(&self, update_expr: UpdateExpr) {
    self.walk_expression(update_expr.arg);
  }

  fn walk_yield_expr(&self, yield_expr: YieldExpr) {
    if let Some(arg) = yield_expr.arg {
      self.walk_expression(arg);
    }
  }

  fn walk_statement(&self, stmt: Stmt) {
    match stmt {
      Stmt::Block(block_stmt) => self.walk_block_stmt(block_stmt),
      Stmt::Empty(empty_stmt) => self.walk_empty_stmt(empty_stmt),
      Stmt::Debugger(debugger_stmt) => self.walk_debugger_stmt(debugger_stmt),
      Stmt::With(with_stmt) => self.walk_with_stmt(with_stmt),
      Stmt::Return(return_stmt) => self.walk_return_stmt(return_stmt),
      Stmt::Labeled(labeled_stmt) => self.walk_labeled_stmt(labeled_stmt),
      Stmt::Break(break_stmt) => self.walk_break_stmt(break_stmt),
      Stmt::Continue(continue_stmt) => self.walk_continue_stmt(continue_stmt),
      Stmt::If(if_stmt) => self.walk_if_stmt(if_stmt),
      Stmt::Switch(switch_stmt) => self.walk_switch_stmt(switch_stmt),
      Stmt::Throw(throw_stmt) => self.walk_throw_stmt(throw_stmt),
      Stmt::Try(try_stmt) => self.walk_try_stmt(try_stmt),
      Stmt::While(while_stmt) => self.walk_while_stmt(while_stmt),
      Stmt::DoWhile(do_while_stmt) => self.walk_do_while_stmt(do_while_stmt),
      Stmt::For(for_stmt) => self.walk_for_stmt(for_stmt),
      Stmt::ForIn(for_in_stmt) => self.walk_for_in_stmt(for_in_stmt),
      Stmt::ForOf(for_of_stmt) => self.walk_for_of_stmt(for_of_stmt),
      Stmt::Decl(decl) => self.walk_decl(decl),
      Stmt::Expr(expr_stmt) => self.walk_expr_stmt(expr_stmt),
    }
  }

  fn walk_block_stmt(&self, block_stmt: BlockStmt) {
    self.walk_statements(block_stmt.stmts);
  }

  fn walk_empty_stmt(&self, empty_stmt: EmptyStmt) {}

  fn walk_debugger_stmt(&self, debugger_stmt: DebuggerStmt) {}

  fn walk_with_stmt(&self, with_stmt: WithStmt) {
    self.walk_expression(with_stmt.obj);
    self.walk_statement(*with_stmt.body);
  }

  fn walk_return_stmt(&self, return_stmt: ReturnStmt) {
    if let Some(arg) = return_stmt.arg {
      self.walk_expression(arg);
    }
  }

  fn walk_labeled_stmt(&self, labeled_stmt: LabeledStmt) {
    self.walk_identifier(labeled_stmt.label);
    self.walk_statement(*labeled_stmt.body);
  }

  fn walk_break_stmt(&self, break_stmt: BreakStmt) {
    if let Some(label) = break_stmt.label {
      self.walk_identifier(label);
    }
  }

  fn walk_continue_stmt(&self, continue_stmt: ContinueStmt) {
    if let Some(label) = continue_stmt.label {
      self.walk_identifier(label);
    }
  }

  fn walk_if_stmt(&self, if_stmt: IfStmt) {
    if let Some(alt) = if_stmt.alt {
      self.walk_statement(*alt);
    }
    self.walk_statement(*if_stmt.cons);
    self.walk_expression(if_stmt.test);
  }

  fn walk_switch_stmt(&self, switch_stmt: SwitchStmt) {
    self.walk_expression(switch_stmt.discriminant);
    for case in switch_stmt.cases {
      if let Some(case) = case.test {
        self.walk_expression(case);
      }
      self.walk_statements(case.cons);
    }
  }

  fn walk_throw_stmt(&self, throw_stmt: ThrowStmt) {
    self.walk_expression(throw_stmt.arg);
  }

  fn walk_try_stmt(&self, try_stmt: TryStmt) {
    self.walk_block_stmt(try_stmt.block);
    if let Some(handler) = try_stmt.handler {
      self.walk_block_stmt(handler.body);
      if let Some(pat) = handler.param {
        self.walk_pattern(pat);
      }
    }
    if let Some(finalizer) = try_stmt.finalizer {
      self.walk_block_stmt(finalizer);
    }
  }

  fn walk_while_stmt(&self, while_stmt: WhileStmt) {
    self.walk_expression(while_stmt.test);
    self.walk_statement(*while_stmt.body)
  }

  fn walk_do_while_stmt(&self, do_while_stmt: DoWhileStmt) {
    self.walk_expression(do_while_stmt.test);
    self.walk_statement(*do_while_stmt.body)
  }

  fn walk_for_stmt(&self, for_stmt: ForStmt) {
    self.walk_statement(*for_stmt.body);
    if let Some(init) = for_stmt.init {
      match init {
        VarDeclOrExpr::Expr(expr) => self.walk_expression(expr),
        VarDeclOrExpr::VarDecl(var_decl) => self.walk_var_decl(var_decl),
      }
    }
    if let Some(test) = for_stmt.test {
      self.walk_expression(test);
    }
    if let Some(update) = for_stmt.update {
      self.walk_expression(update);
    }
  }

  fn walk_for_in_stmt(&self, for_in_stmt: ForInStmt) {
    self.walk_statement(*for_in_stmt.body);
    match for_in_stmt.left {
      VarDeclOrPat::Pat(pat) => self.walk_pattern(pat),
      VarDeclOrPat::VarDecl(var_decl) => self.walk_var_decl(var_decl),
    }
    self.walk_expression(for_in_stmt.right);
  }

  fn walk_for_of_stmt(&self, for_of_stmt: ForOfStmt) {
    self.walk_statement(*for_of_stmt.body);
    match for_of_stmt.left {
      VarDeclOrPat::Pat(pat) => self.walk_pattern(pat),
      VarDeclOrPat::VarDecl(var_decl) => self.walk_var_decl(var_decl),
    }
    self.walk_expression(for_of_stmt.right);
  }

  fn walk_decl(&self, decl: Decl) {
    match decl {
      Decl::Class(class_decl) => self.walk_class_decl(class_decl),
      Decl::Fn(fn_decl) => self.walk_fn_decl(fn_decl),
      Decl::Var(var_decl) => self.walk_var_decl(var_decl),
      Decl::TsInterface(ts_interface_decl) => {
        self.walk_ts_interface_decl(ts_interface_decl)
      }
      Decl::TsTypeAlias(ts_type_alias_decl) => {
        self.walk_ts_type_alias_decl(ts_type_alias_decl)
      }
      Decl::TsEnum(ts_enum_decl) => self.walk_ts_enum_decl(ts_enum_decl),
      Decl::TsModule(ts_module_decl) => {
        self.walk_ts_module_decl(ts_module_decl)
      }
    }
  }

  fn walk_expr_stmt(&self, expr_stmt: ExprStmt) {
    self.walk_expression(expr_stmt.expr);
  }

  fn walk_class_decl(&self, class_decl: ClassDecl) {
    self.walk_identifier(class_decl.ident);
    // TODO: handle class
  }

  fn walk_fn_decl(&self, fn_decl: FnDecl) {
    self.walk_identifier(fn_decl.ident);
    self.walk_function(fn_decl.function)
  }

  fn walk_var_decl(&self, var_decl: VarDecl) {
    for decl in var_decl.decls {
      self.walk_pattern(decl.name);
      if let Some(init) = decl.init {
        self.walk_expression(init);
      }
    }
  }

  fn walk_ts_interface_decl(&self, ts_interface_decl: TsInterfaceDecl) {
    self.walk_identifier(ts_interface_decl.id);
    for expr in ts_interface_decl.extends {
      // TODO: handle expr
      // TODO: handle type args
    }
  }

  fn walk_ts_type_alias_decl(&self, ts_type_alias_decl: TsTypeAliasDecl) {
    self.walk_identifier(ts_type_alias_decl.id);
    self.walk_ts_type(*ts_type_alias_decl.type_ann);
    // TODO: handle type_params
  }

  fn walk_ts_enum_decl(&self, ts_enum_decl: TsEnumDecl) {
    self.walk_identifier(ts_enum_decl.id);
    for member in ts_enum_decl.members {
      match member.id {
        TsEnumMemberId::Ident(ident) => self.walk_identifier(ident),
        TsEnumMemberId::Str(str_) => self.walk_string_literal(str_),
      }
    }
  }

  fn walk_ts_module_decl(&self, ts_module_decl: TsModuleDecl) {
    match ts_module_decl.id {
      TsModuleName::Ident(ident) => self.walk_identifier(ident),
      TsModuleName::Str(str_) => self.walk_string_literal(str_),
    }
    if let Some(body) = ts_module_decl.body {
      match body {
        TsNamespaceBody::TsModuleBlock(module_block) => {
          self.walk_module_items(module_block.body)
        }
        TsNamespaceBody::TsNamespaceDecl(namespace_decl) => {
          self.walk_ts_namespace_decl(namespace_decl)
        }
      }
    }
  }

  fn walk_ts_namespace_decl(&self, ts_namespace_decl: TsNamespaceDecl) {
    self.walk_identifier(ts_namespace_decl.id);
    // TODO: handle body
  }

  fn walk_function(&self, function: Function) {
    if let Some(body) = function.body {
      self.walk_block_stmt(body);
    }
    self.walk_patterns(function.params);
    if let Some(type_ann) = function.return_type {
      self.walk_ts_type_ann(type_ann);
    }
    // TODO: deal with type_params
  }

  fn walk_prop(&self, prop: Prop) {
    match prop {
      Prop::Assign(assign_prop) => self.walk_assign_prop(assign_prop),
      Prop::Getter(getter_prop) => self.walk_getter_prop(getter_prop),
      Prop::KeyValue(key_value_prop) => {
        self.walk_key_value_prop(key_value_prop)
      }
      Prop::Method(method_prop) => self.walk_method_prop(method_prop),
      Prop::Setter(setter_prop) => self.walk_setter_prop(setter_prop),
      Prop::Shorthand(ident) => self.walk_identifier_reference(ident),
    }
  }

  fn walk_assign_prop(&self, assign_prop: AssignProp) {
    self.walk_binding_identifier(assign_prop.key);
    self.walk_expression(assign_prop.value);
  }

  fn walk_getter_prop(&self, getter_prop: GetterProp) {
    self.walk_prop_name(getter_prop.key);
    if let Some(body) = getter_prop.body {
      self.walk_block_stmt(body);
    }
    if let Some(type_ann) = getter_prop.type_ann {
      self.walk_ts_type_ann(type_ann);
    }
  }

  fn walk_key_value_prop(&self, key_value_prop: KeyValueProp) {
    self.walk_prop_name(key_value_prop.key);
    self.walk_expression(key_value_prop.value);
  }

  fn walk_method_prop(&self, method_prop: MethodProp) {
    self.walk_prop_name(method_prop.key);
    self.walk_function(method_prop.function)
  }

  fn walk_setter_prop(&self, setter_prop: SetterProp) {
    self.walk_prop_name(setter_prop.key);
    if let Some(body) = setter_prop.body {
      self.walk_block_stmt(body);
    }
    self.walk_pattern(setter_prop.param);
  }

  fn walk_prop_name(&self, prop_name: PropName) {
    match prop_name {
      PropName::Ident(ident) => self.walk_binding_identifier(ident),
      PropName::Num(num) => self.walk_num_literal(num),
      PropName::Str(str_) => self.walk_string_literal(str_),
      PropName::Computed(computed) => self.walk_expression(computed.expr),
    }
  }

  fn walk_patterns(&self, patterns: Vec<Pat>) {
    for pat in patterns {
      self.walk_pattern(pat)
    }
  }

  fn walk_pattern(&self, pattern: Pat) {
    match pattern {
      Pat::Ident(ident) => self.walk_binding_identifier(ident),
      Pat::Array(array_pat) => self.walk_array_pattern(array_pat),
      Pat::Rest(rest_pat) => self.walk_rest_pattern(rest_pat),
      Pat::Object(object_pat) => self.walk_object_pattern(object_pat),
      Pat::Assign(assign_pat) => self.walk_assign_pattern(assign_pat),
      Pat::Invalid(_) => unreachable!(),
      Pat::Expr(boxed_expr) => self.walk_expression(boxed_expr),
    }
  }

  fn walk_array_pattern(&self, array_pat: ArrayPat) {
    for pat in array_pat.elems {
      if let Some(pat) = pat {
        self.walk_pattern(pat);
      }
    }
    if let Some(type_ann) = array_pat.type_ann {
      self.walk_ts_type_ann(type_ann);
    }
  }

  fn walk_rest_pattern(&self, rest_pat: RestPat) {
    self.walk_pattern(*rest_pat.arg);
    if let Some(type_ann) = rest_pat.type_ann {
      self.walk_ts_type_ann(type_ann);
    }
  }

  fn walk_object_pattern(&self, object_pat: ObjectPat) {
    for prop in object_pat.props {
      match prop {
        ObjectPatProp::Assign(assign_pat_props) => {
          self.walk_binding_identifier(assign_pat_props.key);
          if let Some(expr) = assign_pat_props.value {
            self.walk_expression(expr);
          }
        }
        ObjectPatProp::KeyValue(key_value_pat_props) => {
          self.walk_prop_name(key_value_pat_props.key);
          self.walk_pattern(*key_value_pat_props.value);
        }
        ObjectPatProp::Rest(rest_pat) => self.walk_rest_pattern(rest_pat),
      }
    }
    if let Some(type_ann) = object_pat.type_ann {
      self.walk_ts_type_ann(type_ann);
    }
  }

  fn walk_assign_pattern(&self, assign_pat: AssignPat) {
    self.walk_pattern(*assign_pat.left);
    self.walk_expression(assign_pat.right);
    if let Some(type_ann) = assign_pat.type_ann {
      self.walk_ts_type_ann(type_ann);
    }
  }

  fn walk_ts_type_ann(&self, ts_type_ann: TsTypeAnn) {
    self.walk_ts_type(*ts_type_ann.type_ann);
  }

  fn walk_ts_type(&self, ts_type: TsType) {}
}
