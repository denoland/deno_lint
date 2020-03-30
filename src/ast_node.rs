// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![allow(unused)]

use swc_ecma_ast::*;

// TODO(bartlomieju): copyrights
/// Adopted from dprint
macro_rules! generate_node {
    ($($node_name:ident),*) => {
        #[derive(Clone, PartialEq, Debug)]
        pub enum AstNodeKind {
            $($node_name),*,
        }

        #[derive(Clone, Hash, PartialEq, Debug)]
        pub enum AstNode {
            $($node_name($node_name)),*
        }

        impl Eq for AstNode {}

        impl AstNode {
            pub fn kind(&self) -> AstNodeKind {
                match self {
                    $(AstNode::$node_name(_) => AstNodeKind::$node_name),*
                }
            }
        }
        $(
        impl Into<AstNode> for $node_name {
            fn into(self) -> AstNode {
                AstNode::$node_name(self)
            }
        }
        )*
    };
}

generate_node![
  /* class */
  ClassMethod,
  ClassProp,
  Constructor,
  Decorator,
  PrivateMethod,
  PrivateProp,
  TsParamProp,
  /* clauses */
  CatchClause,
  /* common */
  ComputedPropName,
  Ident,
  Invalid,
  PrivateName,
  TsQualifiedName,
  /* declarations */
  ClassDecl,
  ExportDecl,
  ExportDefaultDecl,
  ExportDefaultExpr,
  FnDecl,
  Function,
  NamedExport,
  ImportDecl,
  TsEnumDecl,
  TsEnumMember,
  TsImportEqualsDecl,
  TsInterfaceDecl,
  TsTypeAliasDecl,
  TsModuleDecl,
  TsModuleBlock,
  TsNamespaceDecl,
  /* exports */
  DefaultExportSpecifier,
  NamespaceExportSpecifier,
  NamedExportSpecifier,
  /* expressions */
  ArrayLit,
  ArrowExpr,
  AssignExpr,
  AssignProp,
  AwaitExpr,
  BinExpr,
  CallExpr,
  ClassExpr,
  CondExpr,
  ExprOrSpread,
  FnExpr,
  GetterProp,
  KeyValueProp,
  MemberExpr,
  MetaPropExpr,
  MethodProp,
  NewExpr,
  ParenExpr,
  ObjectLit,
  OptChainExpr,
  SeqExpr,
  SetterProp,
  SpreadElement,
  Super,
  TaggedTpl,
  ThisExpr,
  Tpl,
  TplElement,
  TsAsExpr,
  TsConstAssertion,
  TsTypeCastExpr,
  TsExprWithTypeArgs,
  TsNonNullExpr,
  TsTypeAssertion,
  UnaryExpr,
  UpdateExpr,
  YieldExpr,
  /* imports */
  ImportDefault,
  ImportSpecific,
  ImportStarAs,
  TsExternalModuleRef,
  /* interface / type element */
  TsInterfaceBody,
  TsCallSignatureDecl,
  TsConstructSignatureDecl,
  TsIndexSignature,
  TsMethodSignature,
  TsPropertySignature,
  TsTypeLit,
  /* jsx */
  JSXAttr,
  JSXClosingElement,
  JSXClosingFragment,
  JSXElement,
  JSXEmptyExpr,
  JSXExprContainer,
  JSXFragment,
  JSXMemberExpr,
  JSXNamespacedName,
  JSXOpeningElement,
  JSXOpeningFragment,
  JSXSpreadChild,
  JSXText,
  /* literals */
  BigInt,
  Bool,
  Null,
  Number,
  Regex,
  Str,
  /* module */
  Module,
  /* patterns */
  ArrayPat,
  AssignPat,
  AssignPatProp,
  KeyValuePatProp,
  ObjectPat,
  RestPat,
  /* statements */
  Stmt,
  BlockStmt,
  BreakStmt,
  ContinueStmt,
  DebuggerStmt,
  DoWhileStmt,
  EmptyStmt,
  ExportAll,
  ExprStmt,
  ForStmt,
  ForInStmt,
  ForOfStmt,
  IfStmt,
  LabeledStmt,
  ReturnStmt,
  SwitchStmt,
  SwitchCase,
  ThrowStmt,
  TryStmt,
  TsExportAssignment,
  TsNamespaceExportDecl,
  VarDecl,
  VarDeclarator,
  WithStmt,
  WhileStmt,
  /* types */
  TsArrayType,
  TsConditionalType,
  TsConstructorType,
  TsKeywordType,
  TsFnType,
  TsImportType,
  TsIndexedAccessType,
  TsInferType,
  TsIntersectionType,
  TsLitType,
  TsMappedType,
  TsOptionalType,
  TsParenthesizedType,
  TsRestType,
  TsThisType,
  TsTupleType,
  TsTypeAnn,
  TsTypeOperator,
  TsTypeParamInstantiation,
  TsTypeParamDecl,
  TsTypeParam,
  TsTypePredicate,
  TsTypeQuery,
  TsTypeRef,
  TsUnionType,
  Decl,
  Pat
];

impl AstNode {
  pub fn get_children(&self) -> Vec<AstNode> {
    use AstNode::*;

    match self {
      Module(module) => self.children_module(module),
      Decl(decl) => self.children_decl(decl),
      FnDecl(fn_decl) => self.children_fn_decl(fn_decl),
      Function(function) => self.children_function(function),
      BlockStmt(block_stmt) => self.children_block_stmt(block_stmt),
      _ => vec![],
    }
  }

  // fn walk_program(&self, program: Program) {
  //   match program {
  //     Program::Module(module) => self.walk_module(module),
  //     Program::Script(script) => self.walk_script(script),
  //   }
  // }

  fn children_module(&self, module: &Module) -> Vec<AstNode> {
    module
      .body
      .iter()
      .map(|module_item| match module_item {
        ModuleItem::ModuleDecl(module_decl) => match module_decl {
          ModuleDecl::Import(import_decl) => import_decl.clone().into(),
          ModuleDecl::ExportDecl(export_decl) => export_decl.clone().into(),
          ModuleDecl::ExportNamed(named_export) => named_export.clone().into(),
          ModuleDecl::ExportDefaultDecl(export_default_decl) => {
            export_default_decl.clone().into()
          }
          ModuleDecl::ExportDefaultExpr(export_default_expr) => {
            export_default_expr.clone().into()
          }
          ModuleDecl::ExportAll(export_all) => export_all.clone().into(),
          ModuleDecl::TsImportEquals(ts_import_equals_decl) => {
            ts_import_equals_decl.clone().into()
          }
          ModuleDecl::TsExportAssignment(ts_export_assignment) => {
            ts_export_assignment.clone().into()
          }
          ModuleDecl::TsNamespaceExport(ts_namespace_export_decl) => {
            ts_namespace_export_decl.clone().into()
          }
        },
        ModuleItem::Stmt(stmt) => self.statement_to_node(stmt),
      })
      .collect()
  }

  // fn walk_module_items(&self, module_items: Vec<ModuleItem>) {
  //   for module_item in module_items {
  //     self.walk_module_item(module_item)
  //   }
  // }

  // fn walk_module_decl(&self, module_decl: ModuleDecl) {
  //   match module_decl {
  //     ModuleDecl::Import(import_decl) => self.walk_import_decl(import_decl),
  //     ModuleDecl::ExportDecl(export_decl) => self.walk_export_decl(export_decl),
  //     ModuleDecl::ExportNamed(named_export) => {
  //       self.walk_named_export(named_export)
  //     }
  //     ModuleDecl::ExportDefaultDecl(export_default_decl) => {
  //       self.walk_export_default_decl(export_default_decl)
  //     }
  //     ModuleDecl::ExportDefaultExpr(export_default_expr) => {
  //       self.walk_export_default_expr(export_default_expr)
  //     }
  //     ModuleDecl::ExportAll(export_all) => self.walk_export_all(export_all),
  //     ModuleDecl::TsImportEquals(ts_import_equals_decl) => {
  //       self.walk_ts_import_equals_decl(ts_import_equals_decl)
  //     }
  //     ModuleDecl::TsExportAssignment(ts_export_assignment) => {
  //       self.walk_ts_export_assignment(ts_export_assignment)
  //     }
  //     ModuleDecl::TsNamespaceExport(ts_namespace_export_decl) => {
  //       self.walk_ts_namespace_export_decl(ts_namespace_export_decl)
  //     }
  //   }
  // }

  // fn walk_import_decl(&self, import_decl: ImportDecl) {
  //   self.walk_string_literal(import_decl.src);
  //   self.walk_import_specifiers(import_decl.specifiers);
  // }

  // fn walk_import_specifiers(&self, import_specifiers: Vec<ImportSpecifier>) {
  //   for specifier in import_specifiers {
  //     self.walk_import_specifier(specifier);
  //   }
  // }

  // fn walk_import_specifier(&self, import_specifier: ImportSpecifier) {
  //   match import_specifier {
  //     ImportSpecifier::Specific(import_specific) => {
  //       self.walk_named_import_specifier(import_specific)
  //     }
  //     ImportSpecifier::Default(import_default) => {
  //       self.walk_import_default_specifier(import_default)
  //     }
  //     ImportSpecifier::Namespace(import_as) => {
  //       self.walk_import_namespace_specifier(import_as)
  //     }
  //   }
  // }

  // fn walk_named_import_specifier(&self, named_import: ImportSpecific) {
  //   self.walk_binding_identifier(named_import.local);
  //   if let Some(imp) = named_import.imported {
  //     self.walk_identifier_reference(imp);
  //   }
  // }

  // fn walk_import_namespace_specifier(&self, import_as: ImportStarAs) {
  //   self.walk_binding_identifier(import_as.local);
  // }

  // fn walk_import_default_specifier(&self, import_default: ImportDefault) {
  //   self.walk_binding_identifier(import_default.local);
  // }

  // fn walk_export_decl(&self, export_decl: ExportDecl) {
  //   self.walk_decl(export_decl.decl)
  // }

  // fn walk_named_export(&self, named_export: NamedExport) {
  //   self.walk_export_specifiers(named_export.specifiers);
  //   self.walk_optional_string_literal(named_export.src);
  // }

  // fn walk_export_specifiers(&self, export_specifiers: Vec<ExportSpecifier>) {
  //   for specifier in export_specifiers {
  //     self.walk_export_specifier(specifier);
  //   }
  // }

  // fn walk_export_specifier(&self, export_specifier: ExportSpecifier) {
  //   match export_specifier {
  //     ExportSpecifier::Default(default_export_specifier) => {
  //       self.walk_export_default_specifier(default_export_specifier)
  //     }
  //     ExportSpecifier::Namespace(ns_export_specifier) => {
  //       self.walk_export_namespace_specifier(ns_export_specifier)
  //     }
  //     ExportSpecifier::Named(named_export_specifier) => {
  //       self.walk_named_export_specifier(named_export_specifier)
  //     }
  //   }
  // }

  // fn walk_export_default_specifier(
  //   &self,
  //   default_export_specifier: DefaultExportSpecifier,
  // ) {
  //   self.walk_binding_identifier(default_export_specifier.exported)
  // }

  // fn walk_export_namespace_specifier(
  //   &self,
  //   ns_export_specifier: NamespaceExportSpecifier,
  // ) {
  //   self.walk_binding_identifier(ns_export_specifier.name);
  // }

  // fn walk_named_export_specifier(
  //   &self,
  //   named_export_specifier: NamedExportSpecifier,
  // ) {
  //   if let Some(exp) = named_export_specifier.exported {
  //     self.walk_binding_identifier(exp);
  //   }
  //   self.walk_identifier_reference(named_export_specifier.orig)
  // }

  // fn walk_optional_string_literal(&self, optional_literal: Option<Str>) {
  //   if let Some(literal) = optional_literal {
  //     self.walk_string_literal(literal)
  //   }
  // }

  // fn walk_binding_identifier(&self, ident: Ident) {
  //   self.walk_identifier(ident)
  // }

  // fn walk_identifier_reference(&self, ident: Ident) {
  //   self.walk_identifier(ident)
  // }

  // fn walk_identifier(&self, ident: Ident) {}

  // fn walk_string_literal(&self, str_literal: Str) {}
  // fn walk_export_default_decl(&self, export_default_decl: ExportDefaultDecl) {}
  // fn walk_export_default_expr(&self, export_default_expr: ExportDefaultExpr) {}
  // fn walk_export_all(&self, export_all: ExportAll) {}

  // fn walk_ts_import_equals_decl(
  //   &self,
  //   ts_import_equals_decl: TsImportEqualsDecl,
  // ) {
  //   self.walk_binding_identifier(ts_import_equals_decl.id);
  //   self.walk_ts_module_reference(ts_import_equals_decl.module_ref);
  // }

  // fn walk_ts_module_reference(&self, ts_module_ref: TsModuleRef) {}

  // fn walk_ts_export_assignment(
  //   &self,
  //   ts_export_assignment: TsExportAssignment,
  // ) {
  //   self.walk_expression(ts_export_assignment.expr);
  // }

  // fn walk_ts_namespace_export_decl(
  //   &self,
  //   ts_namespace_export_decl: TsNamespaceExportDecl,
  // ) {
  //   self.walk_binding_identifier(ts_namespace_export_decl.id);
  // }

  // fn walk_statements(&self, stmts: Vec<Stmt>) {
  //   for stmt in stmts {
  //     self.walk_statement(stmt);
  //   }
  // }

  // fn walk_expression(&self, expr: Box<Expr>) {
  //   match *expr {
  //     Expr::Array(array_lit) => self.walk_array_lit(array_lit),
  //     Expr::Arrow(arrow_expr) => self.walk_arrow_expr(arrow_expr),
  //     Expr::Assign(assign_expr) => self.walk_assign_expr(assign_expr),
  //     Expr::Await(await_expr) => self.walk_await_expr(await_expr),
  //     Expr::Bin(bin_expr) => self.walk_bin_expr(bin_expr),
  //     Expr::Call(call_expr) => self.walk_call_expr(call_expr),
  //     Expr::Class(class_expr) => self.walk_class_expr(class_expr),
  //     Expr::Cond(cond_expr) => self.walk_cond_expr(cond_expr),
  //     Expr::Fn(fn_expr) => self.walk_fn_expr(fn_expr),
  //     Expr::Ident(ident) => self.walk_identifier_reference(ident),
  //     Expr::Invalid(_) => {}
  //     Expr::JSXMember(jsx_member_expr) => {
  //       self.walk_jsx_member_expr(jsx_member_expr)
  //     }
  //     Expr::JSXNamespacedName(jsx_namespaced_name) => {
  //       self.walk_jsx_namespaced_name(jsx_namespaced_name)
  //     }
  //     Expr::JSXEmpty(jsx_empty_expr) => self.walk_jsx_empty(jsx_empty_expr),
  //     Expr::JSXElement(jsx_element) => self.walk_jsx_element(jsx_element),
  //     Expr::JSXFragment(jsx_fragment) => self.walk_jsx_fragment(jsx_fragment),
  //     Expr::Member(member_expr) => self.walk_member_expr(member_expr),
  //     Expr::MetaProp(meta_prop_expr) => {
  //       self.walk_meta_prop_expr(meta_prop_expr)
  //     }
  //     Expr::New(new_expr) => self.walk_new_expr(new_expr),
  //     Expr::Lit(lit) => self.walk_lit(lit),
  //     Expr::Object(object_lit) => self.walk_object_lit(object_lit),
  //     Expr::OptChain(opt_chain_expr) => {
  //       self.walk_opt_chain_expr(opt_chain_expr)
  //     }
  //     Expr::Paren(paren_expr) => self.walk_paren_expr(paren_expr),
  //     Expr::PrivateName(private_name) => self.walk_private_name(private_name),
  //     Expr::Seq(seq_expr) => self.walk_seq_expr(seq_expr),
  //     Expr::TaggedTpl(tagged_tpl) => self.walk_tagged_tpl(tagged_tpl),
  //     Expr::This(this_expr) => self.walk_this_expr(this_expr),
  //     Expr::Tpl(tpl) => self.walk_tpl(tpl),
  //     Expr::TsTypeAssertion(ts_type_assertion) => {
  //       self.walk_ts_type_assertion(ts_type_assertion)
  //     }
  //     Expr::TsConstAssertion(ts_const_assertion) => {
  //       self.walk_ts_const_assertion(ts_const_assertion)
  //     }
  //     Expr::TsNonNull(ts_non_null_expr) => {
  //       self.walk_ts_non_null_expr(ts_non_null_expr)
  //     }
  //     Expr::TsTypeCast(ts_type_cast_expr) => {
  //       self.walk_ts_type_cast_expr(ts_type_cast_expr)
  //     }
  //     Expr::TsAs(ts_as_expr) => self.walk_ts_as_expr(ts_as_expr),
  //     Expr::Unary(unary_expr) => self.walk_unary_expr(unary_expr),
  //     Expr::Update(update_expr) => self.walk_update_expr(update_expr),
  //     Expr::Yield(yield_expr) => self.walk_yield_expr(yield_expr),
  //   }
  // }

  // fn walk_array_lit(&self, array_lit: ArrayLit) {}
  // fn walk_arrow_expr(&self, arrow_expr: ArrowExpr) {}
  // fn walk_assign_expr(&self, assign_expr: AssignExpr) {}
  // fn walk_await_expr(&self, await_expr: AwaitExpr) {}
  // fn walk_bin_expr(&self, bin_expr: BinExpr) {}
  // fn walk_call_expr(&self, call_expr: CallExpr) {}
  // fn walk_class_expr(&self, class_expr: ClassExpr) {}
  // fn walk_cond_expr(&self, cond_expr: CondExpr) {}
  // fn walk_fn_expr(&self, fn_expr: FnExpr) {}
  // fn walk_jsx_member_expr(&self, jsx_member_expr: JSXMemberExpr) {}
  // fn walk_jsx_namespaced_name(&self, js_namespaced_name: JSXNamespacedName) {}
  // fn walk_jsx_empty(&self, jsx_empty: JSXEmptyExpr) {}
  // fn walk_jsx_element(&self, jsx_element: Box<JSXElement>) {}
  // fn walk_jsx_fragment(&self, jsx_fragment: JSXFragment) {}
  // fn walk_member_expr(&self, member_expr: MemberExpr) {}
  // fn walk_meta_prop_expr(&self, meta_prop_expr: MetaPropExpr) {}
  // fn walk_new_expr(&self, new_expr: NewExpr) {}
  // fn walk_lit(&self, lit: Lit) {}
  // fn walk_object_lit(&self, object_lit: ObjectLit) {}
  // fn walk_opt_chain_expr(&self, opt_chain_expr: OptChainExpr) {}
  // fn walk_paren_expr(&self, parent_expr: ParenExpr) {}
  // fn walk_private_name(&self, private_name: PrivateName) {}
  // fn walk_seq_expr(&self, seq_expr: SeqExpr) {}
  // fn walk_tagged_tpl(&self, tagged_tpl: TaggedTpl) {}
  // fn walk_this_expr(&self, this_expr: ThisExpr) {}
  // fn walk_tpl(&self, tpl: Tpl) {}
  // fn walk_ts_type_assertion(&self, type_assertion: TsTypeAssertion) {}
  // fn walk_ts_const_assertion(&self, const_assertion: TsConstAssertion) {}
  // fn walk_ts_non_null_expr(&self, non_null_expr: TsNonNullExpr) {}
  // fn walk_ts_type_cast_expr(&self, type_cast_expr: TsTypeCastExpr) {}
  // fn walk_ts_as_expr(&self, as_expr: TsAsExpr) {}
  // fn walk_unary_expr(&self, unary_expr: UnaryExpr) {}
  // fn walk_update_expr(&self, update_expr: UpdateExpr) {}
  // fn walk_yield_expr(&self, yield_expr: YieldExpr) {}

  fn statement_to_node(&self, stmt: &Stmt) -> AstNode {
    match stmt {
      Stmt::Block(block_stmt) => block_stmt.clone().into(),
      Stmt::Empty(empty_stmt) => empty_stmt.clone().into(),
      Stmt::Debugger(debugger_stmt) => debugger_stmt.clone().into(),
      Stmt::With(with_stmt) => with_stmt.clone().into(),
      Stmt::Return(return_stmt) => return_stmt.clone().into(),
      Stmt::Labeled(labeled_stmt) => labeled_stmt.clone().into(),
      Stmt::Break(break_stmt) => break_stmt.clone().into(),
      Stmt::Continue(continue_stmt) => continue_stmt.clone().into(),
      Stmt::If(if_stmt) => if_stmt.clone().into(),
      Stmt::Switch(switch_stmt) => switch_stmt.clone().into(),
      Stmt::Throw(throw_stmt) => throw_stmt.clone().into(),
      Stmt::Try(try_stmt) => try_stmt.clone().into(),
      Stmt::While(while_stmt) => while_stmt.clone().into(),
      Stmt::DoWhile(do_while_stmt) => do_while_stmt.clone().into(),
      Stmt::For(for_stmt) => for_stmt.clone().into(),
      Stmt::ForIn(for_in_stmt) => for_in_stmt.clone().into(),
      Stmt::ForOf(for_of_stmt) => for_of_stmt.clone().into(),
      Stmt::Decl(decl) => decl.clone().into(),
      Stmt::Expr(expr_stmt) => expr_stmt.clone().into(),
    }
  }

  fn children_block_stmt(&self, block_stmt: &BlockStmt) -> Vec<AstNode> {
    block_stmt
      .stmts
      .iter()
      .map(|e| self.statement_to_node(e))
      .collect()
  }

  // fn walk_empty_stmt(&self, empty_stmt: EmptyStmt) {}
  // fn walk_debugger_stmt(&self, debugger_stmt: DebuggerStmt) {}
  // fn walk_with_stmt(&self, with_stmt: WithStmt) {}
  // fn walk_return_stmt(&self, return_stmt: ReturnStmt) {}
  // fn walk_labeled_stmt(&self, labeled_stmt: LabeledStmt) {}
  // fn walk_break_stmt(&self, break_stmt: BreakStmt) {}
  // fn walk_continue_stmt(&self, continue_stmt: ContinueStmt) {}
  // fn walk_if_stmt(&self, if_stmt: IfStmt) {}
  // fn walk_switch_stmt(&self, switch_stmt: SwitchStmt) {}
  // fn walk_throw_stmt(&self, throw_stmt: ThrowStmt) {}
  // fn walk_try_stmt(&self, try_stmt: TryStmt) {}
  // fn walk_while_stmt(&self, while_stmt: WhileStmt) {}
  // fn walk_do_while_stmt(&self, do_while_stmt: DoWhileStmt) {}
  // fn walk_for_stmt(&self, for_stmt: ForStmt) {}
  // fn walk_for_in_stmt(&self, for_in_stmt: ForInStmt) {}
  // fn walk_for_of_stmt(&self, for_of_stmt: ForOfStmt) {}
  fn children_decl(&self, decl: &Decl) -> Vec<AstNode> {
    let node = match decl {
      Decl::Class(class_decl) => class_decl.clone().into(),
      Decl::Fn(fn_decl) => fn_decl.clone().into(),
      Decl::Var(var_decl) => var_decl.clone().into(),
      Decl::TsInterface(ts_interface_decl) => ts_interface_decl.clone().into(),
      Decl::TsTypeAlias(ts_type_alias_decl) => {
        ts_type_alias_decl.clone().into()
      }
      Decl::TsEnum(ts_enum_decl) => ts_enum_decl.clone().into(),
      Decl::TsModule(ts_module_decl) => ts_module_decl.clone().into(),
    };

    vec![node]
  }
  // fn walk_expr_stmt(&self, expr_stmt: ExprStmt) {
  //   self.walk_expression(expr_stmt.expr);
  // }

  // fn walk_class_decl(&self, class_decl: ClassDecl) {}
  fn children_fn_decl(&self, fn_decl: &FnDecl) -> Vec<AstNode> {
    vec![
      fn_decl.ident.clone().into(),
      fn_decl.function.clone().into(),
    ]
  }
  // fn walk_var_decl(&self, var_decl: VarDecl) {}
  // fn walk_ts_interface_decl(&self, ts_interface_decl: TsInterfaceDecl) {}
  // fn walk_ts_type_alias_decl(&self, ts_type_alias_decl: TsTypeAliasDecl) {}
  // fn walk_ts_enum_decl(&self, ts_enum_decl: TsEnumDecl) {}
  // fn walk_ts_module_decl(&self, ts_module_decl: TsModuleDecl) {}

  fn children_function(&self, function: &Function) -> Vec<AstNode> {
    let mut nodes = vec![];

    if let Some(body) = function.body.clone() {
      nodes.push(body.into());
    }

    for param in &function.params {
      nodes.push(param.clone().into());
    }

    if let Some(type_ann) = function.return_type.clone() {
      nodes.push(type_ann.into());
    }
    nodes
  }

  // fn walk_patterns(&self, patterns: Vec<Pat>) {
  //   for pat in patterns {
  //     self.walk_pattern(pat)
  //   }
  // }

  // fn walk_pattern(&self, pattern: Pat) {
  //   match pattern {
  //     Pat::Ident(ident) => self.walk_binding_identifier(ident),
  //     Pat::Array(array_pat) => self.walk_array_pattern(array_pat),
  //     Pat::Rest(rest_pat) => self.walk_rest_pattern(rest_pat),
  //     Pat::Object(object_pat) => self.walk_object_pattern(object_pat),
  //     Pat::Assign(assign_pat) => self.walk_assign_pattern(assign_pat),
  //     Pat::Invalid(_) => unreachable!(),
  //     Pat::Expr(boxed_expr) => self.walk_expression(boxed_expr),
  //   }
  // }

  // fn walk_array_pattern(&self, array_pat: ArrayPat) {}
  // fn walk_rest_pattern(&self, rest_pat: RestPat) {}
  // fn walk_object_pattern(&self, object_pat: ObjectPat) {}
  // fn walk_assign_pattern(&self, assign_pat: AssignPat) {}

  // fn walk_ts_type_ann(&self, ts_type_ann: TsTypeAnn) {
  //   self.walk_ts_type(*ts_type_ann.type_ann);
  // }

  // fn walk_ts_type(&self, ts_type: TsType) {}
}
