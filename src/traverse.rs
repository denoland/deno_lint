#![allow(unused)]

use swc_ecma_ast::*;

pub struct AstTraverser {}

impl AstTraverser {
  pub fn walk_program(&self, program: Program) {
    match program {
      Program::Module(module) => self.walk_module(module),
      Program::Script(script) => self.walk_script(script),
    }
  }

  pub fn walk_module(&self, module: Module) {
    self.walk_module_items(module.body)
  }

  pub fn walk_script(&self, script: Script) {
    self.walk_statements(script.body)
  }

  pub fn walk_module_items(&self, module_items: Vec<ModuleItem>) {
    for module_item in module_items {
      self.walk_module_item(module_item)
    }
  }

  pub fn walk_module_item(&self, module_item: ModuleItem) {
    match module_item {
      ModuleItem::ModuleDecl(module_decl) => self.walk_module_decl(module_decl),
      ModuleItem::Stmt(stmt) => self.walk_statement(stmt),
    }
  }

  pub fn walk_module_decl(&self, module_decl: ModuleDecl) {
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

  pub fn walk_import_decl(&self, import_decl: ImportDecl) {
    self.walk_string_literal(import_decl.src);
    self.walk_import_specifiers(import_decl.specifiers);
  }

  pub fn walk_import_specifiers(
    &self,
    import_specifiers: Vec<ImportSpecifier>,
  ) {
    for specifier in import_specifiers {
      self.walk_import_specifier(specifier);
    }
  }

  pub fn walk_import_specifier(&self, import_specifier: ImportSpecifier) {
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

  pub fn walk_named_import_specifier(&self, named_import: ImportSpecific) {
    self.walk_binding_identifier(named_import.local);
    if let Some(imp) = named_import.imported {
      self.walk_identifier_reference(imp);
    }
  }

  pub fn walk_import_namespace_specifier(&self, import_as: ImportStarAs) {
    self.walk_binding_identifier(import_as.local);
  }

  pub fn walk_import_default_specifier(&self, import_default: ImportDefault) {
    self.walk_binding_identifier(import_default.local);
  }

  pub fn walk_export_decl(&self, export_decl: ExportDecl) {
    self.walk_decl(export_decl.decl)
  }

  pub fn walk_named_export(&self, named_export: NamedExport) {
    self.walk_export_specifiers(named_export.specifiers);
    self.walk_optional_string_literal(named_export.src);
  }

  pub fn walk_export_specifiers(
    &self,
    export_specifiers: Vec<ExportSpecifier>,
  ) {
    for specifier in export_specifiers {
      self.walk_export_specifier(specifier);
    }
  }

  pub fn walk_export_specifier(&self, export_specifier: ExportSpecifier) {
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

  pub fn walk_export_default_specifier(
    &self,
    default_export_specifier: DefaultExportSpecifier,
  ) {
    self.walk_binding_identifier(default_export_specifier.exported)
  }

  pub fn walk_export_namespace_specifier(
    &self,
    ns_export_specifier: NamespaceExportSpecifier,
  ) {
    self.walk_binding_identifier(ns_export_specifier.name);
  }

  pub fn walk_named_export_specifier(
    &self,
    named_export_specifier: NamedExportSpecifier,
  ) {
    if let Some(exp) = named_export_specifier.exported {
      self.walk_binding_identifier(exp);
    }
    self.walk_identifier_reference(named_export_specifier.orig)
  }

  pub fn walk_optional_string_literal(&self, optional_literal: Option<Str>) {
    if let Some(literal) = optional_literal {
      self.walk_string_literal(literal)
    }
  }

  pub fn walk_binding_identifier(&self, ident: Ident) {
    self.walk_identifier(ident)
  }

  pub fn walk_identifier_reference(&self, ident: Ident) {
    self.walk_identifier(ident)
  }

  pub fn walk_identifier(&self, ident: Ident) {}

  pub fn walk_string_literal(&self, str_literal: Str) {}
  pub fn walk_export_default_decl(
    &self,
    export_default_decl: ExportDefaultDecl,
  ) {
  }
  pub fn walk_export_default_expr(
    &self,
    export_default_expr: ExportDefaultExpr,
  ) {
  }
  pub fn walk_export_all(&self, export_all: ExportAll) {}

  pub fn walk_ts_import_equals_decl(
    &self,
    ts_import_equals_decl: TsImportEqualsDecl,
  ) {
    self.walk_binding_identifier(ts_import_equals_decl.id);
    self.walk_ts_module_reference(ts_import_equals_decl.module_ref);
  }

  pub fn walk_ts_module_reference(&self, ts_module_ref: TsModuleRef) {}

  pub fn walk_ts_export_assignment(
    &self,
    ts_export_assignment: TsExportAssignment,
  ) {
    self.walk_expression(ts_export_assignment.expr);
  }

  pub fn walk_ts_namespace_export_decl(
    &self,
    ts_namespace_export_decl: TsNamespaceExportDecl,
  ) {
    self.walk_binding_identifier(ts_namespace_export_decl.id);
  }

  pub fn walk_statements(&self, stmts: Vec<Stmt>) {
    for stmt in stmts {
      self.walk_statement(stmt);
    }
  }

  pub fn walk_expression(&self, expr: Box<Expr>) {
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

  pub fn walk_array_lit(&self, array_lit: ArrayLit) {}
  pub fn walk_arrow_expr(&self, arrow_expr: ArrowExpr) {}
  pub fn walk_assign_expr(&self, assign_expr: AssignExpr) {}
  pub fn walk_await_expr(&self, await_expr: AwaitExpr) {}
  pub fn walk_bin_expr(&self, bin_expr: BinExpr) {}
  pub fn walk_call_expr(&self, call_expr: CallExpr) {}
  pub fn walk_class_expr(&self, class_expr: ClassExpr) {}
  pub fn walk_cond_expr(&self, cond_expr: CondExpr) {}
  pub fn walk_fn_expr(&self, fn_expr: FnExpr) {}
  pub fn walk_jsx_member_expr(&self, jsx_member_expr: JSXMemberExpr) {}
  pub fn walk_jsx_namespaced_name(
    &self,
    js_namespaced_name: JSXNamespacedName,
  ) {
  }
  pub fn walk_jsx_empty(&self, jsx_empty: JSXEmptyExpr) {}
  pub fn walk_jsx_element(&self, jsx_element: Box<JSXElement>) {}
  pub fn walk_jsx_fragment(&self, jsx_fragment: JSXFragment) {}
  pub fn walk_member_expr(&self, member_expr: MemberExpr) {}
  pub fn walk_meta_prop_expr(&self, meta_prop_expr: MetaPropExpr) {}
  pub fn walk_new_expr(&self, new_expr: NewExpr) {}
  pub fn walk_lit(&self, lit: Lit) {}
  pub fn walk_object_lit(&self, object_lit: ObjectLit) {}
  pub fn walk_opt_chain_expr(&self, opt_chain_expr: OptChainExpr) {}
  pub fn walk_paren_expr(&self, parent_expr: ParenExpr) {}
  pub fn walk_private_name(&self, private_name: PrivateName) {}
  pub fn walk_seq_expr(&self, seq_expr: SeqExpr) {}
  pub fn walk_tagged_tpl(&self, tagged_tpl: TaggedTpl) {}
  pub fn walk_this_expr(&self, this_expr: ThisExpr) {}
  pub fn walk_tpl(&self, tpl: Tpl) {}
  pub fn walk_ts_type_assertion(&self, type_assertion: TsTypeAssertion) {}
  pub fn walk_ts_const_assertion(&self, const_assertion: TsConstAssertion) {}
  pub fn walk_ts_non_null_expr(&self, non_null_expr: TsNonNullExpr) {}
  pub fn walk_ts_type_cast_expr(&self, type_cast_expr: TsTypeCastExpr) {}
  pub fn walk_ts_as_expr(&self, as_expr: TsAsExpr) {}
  pub fn walk_unary_expr(&self, unary_expr: UnaryExpr) {}
  pub fn walk_update_expr(&self, update_expr: UpdateExpr) {}
  pub fn walk_yield_expr(&self, yield_expr: YieldExpr) {}

  pub fn walk_statement(&self, stmt: Stmt) {
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

  pub fn walk_block_stmt(&self, block_stmt: BlockStmt) {}
  pub fn walk_empty_stmt(&self, empty_stmt: EmptyStmt) {}
  pub fn walk_debugger_stmt(&self, debugger_stmt: DebuggerStmt) {}
  pub fn walk_with_stmt(&self, with_stmt: WithStmt) {}
  pub fn walk_return_stmt(&self, return_stmt: ReturnStmt) {}
  pub fn walk_labeled_stmt(&self, labeled_stmt: LabeledStmt) {}
  pub fn walk_break_stmt(&self, break_stmt: BreakStmt) {}
  pub fn walk_continue_stmt(&self, continue_stmt: ContinueStmt) {}
  pub fn walk_if_stmt(&self, if_stmt: IfStmt) {}
  pub fn walk_switch_stmt(&self, switch_stmt: SwitchStmt) {}
  pub fn walk_throw_stmt(&self, throw_stmt: ThrowStmt) {}
  pub fn walk_try_stmt(&self, try_stmt: TryStmt) {}
  pub fn walk_while_stmt(&self, while_stmt: WhileStmt) {}
  pub fn walk_do_while_stmt(&self, do_while_stmt: DoWhileStmt) {}
  pub fn walk_for_stmt(&self, for_stmt: ForStmt) {}
  pub fn walk_for_in_stmt(&self, for_in_stmt: ForInStmt) {}
  pub fn walk_for_of_stmt(&self, for_of_stmt: ForOfStmt) {}
  pub fn walk_decl(&self, decl: Decl) {}
  pub fn walk_expr_stmt(&self, expr_stmt: ExprStmt) {}
}
