// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::linter::Context;
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait};

pub trait Handler {
  fn array_lit(&self, _n: &AstView::ArrayLit, _ctx: &mut Context) {}
  fn array_pat(&self, _n: &AstView::ArrayPat, _ctx: &mut Context) {}
  fn arrow_expr(&self, _n: &AstView::ArrowExpr, _ctx: &mut Context) {}
  fn assign_expr(&self, _n: &AstView::AssignExpr, _ctx: &mut Context) {}
  fn assign_pat(&self, _n: &AstView::AssignPat, _ctx: &mut Context) {}
  fn assign_pat_prop(&self, _n: &AstView::AssignPatProp, _ctx: &mut Context) {}
  fn assign_prop(&self, _n: &AstView::AssignProp, _ctx: &mut Context) {}
  fn await_expr(&self, _n: &AstView::AwaitExpr, _ctx: &mut Context) {}
  fn big_int(&self, _n: &AstView::BigInt, _ctx: &mut Context) {}
  fn bin_expr(&self, _n: &AstView::BinExpr, _ctx: &mut Context) {}
  fn binding_ident(&self, _n: &AstView::BindingIdent, _ctx: &mut Context) {}
  fn block_stmt(&self, _n: &AstView::BlockStmt, _ctx: &mut Context) {}
  fn bool(&self, _n: &AstView::Bool, _ctx: &mut Context) {}
  fn break_stmt(&self, _n: &AstView::BreakStmt, _ctx: &mut Context) {}
  fn call_expr(&self, _n: &AstView::CallExpr, _ctx: &mut Context) {}
  fn catch_clause(&self, _n: &AstView::CatchClause, _ctx: &mut Context) {}
  fn class(&self, _n: &AstView::Class, _ctx: &mut Context) {}
  fn class_decl(&self, _n: &AstView::ClassDecl, _ctx: &mut Context) {}
  fn class_expr(&self, _n: &AstView::ClassExpr, _ctx: &mut Context) {}
  fn class_method(&self, _n: &AstView::ClassMethod, _ctx: &mut Context) {}
  fn class_prop(&self, _n: &AstView::ClassProp, _ctx: &mut Context) {}
  fn computed_prop_name(
    &self,
    _n: &AstView::ComputedPropName,
    _ctx: &mut Context,
  ) {
  }
  fn cond_expr(&self, _n: &AstView::CondExpr, _ctx: &mut Context) {}
  fn constructor(&self, _n: &AstView::Constructor, _ctx: &mut Context) {}
  fn continue_stmt(&self, _n: &AstView::ContinueStmt, _ctx: &mut Context) {}
  fn debugger_stmt(&self, _n: &AstView::DebuggerStmt, _ctx: &mut Context) {}
  fn decorator(&self, _n: &AstView::Decorator, _ctx: &mut Context) {}
  fn do_while_stmt(&self, _n: &AstView::DoWhileStmt, _ctx: &mut Context) {}
  fn empty_stmt(&self, _n: &AstView::EmptyStmt, _ctx: &mut Context) {}
  fn export_all(&self, _n: &AstView::ExportAll, _ctx: &mut Context) {}
  fn export_decl(&self, _n: &AstView::ExportDecl, _ctx: &mut Context) {}
  fn export_default_decl(
    &self,
    _n: &AstView::ExportDefaultDecl,
    _ctx: &mut Context,
  ) {
  }
  fn export_default_expr(
    &self,
    _n: &AstView::ExportDefaultExpr,
    _ctx: &mut Context,
  ) {
  }
  fn export_default_specifier(
    &self,
    _n: &AstView::ExportDefaultSpecifier,
    _ctx: &mut Context,
  ) {
  }
  fn export_named_specifier(
    &self,
    _n: &AstView::ExportNamedSpecifier,
    _ctx: &mut Context,
  ) {
  }
  fn export_namespace_specifier(
    &self,
    _n: &AstView::ExportNamespaceSpecifier,
    _ctx: &mut Context,
  ) {
  }
  fn expr_or_spread(&self, _n: &AstView::ExprOrSpread, _ctx: &mut Context) {}
  fn expr_stmt(&self, _n: &AstView::ExprStmt, _ctx: &mut Context) {}
  fn fn_decl(&self, _n: &AstView::FnDecl, _ctx: &mut Context) {}
  fn fn_expr(&self, _n: &AstView::FnExpr, _ctx: &mut Context) {}
  fn for_in_stmt(&self, _n: &AstView::ForInStmt, _ctx: &mut Context) {}
  fn for_of_stmt(&self, _n: &AstView::ForOfStmt, _ctx: &mut Context) {}
  fn for_stmt(&self, _n: &AstView::ForStmt, _ctx: &mut Context) {}
  fn function(&self, _n: &AstView::Function, _ctx: &mut Context) {}
  fn getter_prop(&self, _n: &AstView::GetterProp, _ctx: &mut Context) {}
  fn ident(&self, _n: &AstView::Ident, _ctx: &mut Context) {}
  fn if_stmt(&self, _n: &AstView::IfStmt, _ctx: &mut Context) {}
  fn import_decl(&self, _n: &AstView::ImportDecl, _ctx: &mut Context) {}
  fn import_default_specifier(
    &self,
    _n: &AstView::ImportDefaultSpecifier,
    _ctx: &mut Context,
  ) {
  }
  fn import_named_specifier(
    &self,
    _n: &AstView::ImportNamedSpecifier,
    _ctx: &mut Context,
  ) {
  }
  fn import_star_as_specifier(
    &self,
    _n: &AstView::ImportStarAsSpecifier,
    _ctx: &mut Context,
  ) {
  }
  fn invalid(&self, _n: &AstView::Invalid, _ctx: &mut Context) {}
  fn jsx_attr(&self, _n: &AstView::JSXAttr, _ctx: &mut Context) {}
  fn jsx_closing_element(
    &self,
    _n: &AstView::JSXClosingElement,
    _ctx: &mut Context,
  ) {
  }
  fn jsx_closing_fragment(
    &self,
    _n: &AstView::JSXClosingFragment,
    _ctx: &mut Context,
  ) {
  }
  fn jsx_element(&self, _n: &AstView::JSXElement, _ctx: &mut Context) {}
  fn jsx_empty_expr(&self, _n: &AstView::JSXEmptyExpr, _ctx: &mut Context) {}
  fn jsx_expr_container(
    &self,
    _n: &AstView::JSXExprContainer,
    _ctx: &mut Context,
  ) {
  }
  fn jsx_fragment(&self, _n: &AstView::JSXFragment, _ctx: &mut Context) {}
  fn jsx_member_expr(&self, _n: &AstView::JSXMemberExpr, _ctx: &mut Context) {}
  fn jsx_namespaced_name(
    &self,
    _n: &AstView::JSXNamespacedName,
    _ctx: &mut Context,
  ) {
  }
  fn jsx_opening_element(
    &self,
    _n: &AstView::JSXOpeningElement,
    _ctx: &mut Context,
  ) {
  }
  fn jsx_opening_fragment(
    &self,
    _n: &AstView::JSXOpeningFragment,
    _ctx: &mut Context,
  ) {
  }
  fn jsx_spread_child(&self, _n: &AstView::JSXSpreadChild, _ctx: &mut Context) {
  }
  fn jsx_text(&self, _n: &AstView::JSXText, _ctx: &mut Context) {}
  fn key_value_pat_prop(
    &self,
    _n: &AstView::KeyValuePatProp,
    _ctx: &mut Context,
  ) {
  }
  fn key_value_prop(&self, _n: &AstView::KeyValueProp, _ctx: &mut Context) {}
  fn labeled_stmt(&self, _n: &AstView::LabeledStmt, _ctx: &mut Context) {}
  fn member_expr(&self, _n: &AstView::MemberExpr, _ctx: &mut Context) {}
  fn meta_prop_expr(&self, _n: &AstView::MetaPropExpr, _ctx: &mut Context) {}
  fn method_prop(&self, _n: &AstView::MethodProp, _ctx: &mut Context) {}
  fn module(&self, _n: &AstView::Module, _ctx: &mut Context) {}
  fn named_export(&self, _n: &AstView::NamedExport, _ctx: &mut Context) {}
  fn new_expr(&self, _n: &AstView::NewExpr, _ctx: &mut Context) {}
  fn null(&self, _n: &AstView::Null, _ctx: &mut Context) {}
  fn number(&self, _n: &AstView::Number, _ctx: &mut Context) {}
  fn object_lit(&self, _n: &AstView::ObjectLit, _ctx: &mut Context) {}
  fn object_pat(&self, _n: &AstView::ObjectPat, _ctx: &mut Context) {}
  fn opt_chain_expr(&self, _n: &AstView::OptChainExpr, _ctx: &mut Context) {}
  fn param(&self, _n: &AstView::Param, _ctx: &mut Context) {}
  fn paren_expr(&self, _n: &AstView::ParenExpr, _ctx: &mut Context) {}
  fn private_method(&self, _n: &AstView::PrivateMethod, _ctx: &mut Context) {}
  fn private_name(&self, _n: &AstView::PrivateName, _ctx: &mut Context) {}
  fn private_prop(&self, _n: &AstView::PrivateProp, _ctx: &mut Context) {}
  fn regex(&self, _n: &AstView::Regex, _ctx: &mut Context) {}
  fn rest_pat(&self, _n: &AstView::RestPat, _ctx: &mut Context) {}
  fn return_stmt(&self, _n: &AstView::ReturnStmt, _ctx: &mut Context) {}
  fn script(&self, _n: &AstView::Script, _ctx: &mut Context) {}
  fn seq_expr(&self, _n: &AstView::SeqExpr, _ctx: &mut Context) {}
  fn setter_prop(&self, _n: &AstView::SetterProp, _ctx: &mut Context) {}
  fn spread_element(&self, _n: &AstView::SpreadElement, _ctx: &mut Context) {}
  fn str(&self, _n: &AstView::Str, _ctx: &mut Context) {}
  // Neither `super` or `r#super` can be used here, so we use `super_` reluctantly
  fn super_(&self, _n: &AstView::Super, _ctx: &mut Context) {}
  fn switch_case(&self, _n: &AstView::SwitchCase, _ctx: &mut Context) {}
  fn switch_stmt(&self, _n: &AstView::SwitchStmt, _ctx: &mut Context) {}
  fn tagged_tpl(&self, _n: &AstView::TaggedTpl, _ctx: &mut Context) {}
  fn this_expr(&self, _n: &AstView::ThisExpr, _ctx: &mut Context) {}
  fn throw_stmt(&self, _n: &AstView::ThrowStmt, _ctx: &mut Context) {}
  fn tpl(&self, _n: &AstView::Tpl, _ctx: &mut Context) {}
  fn tpl_element(&self, _n: &AstView::TplElement, _ctx: &mut Context) {}
  fn try_stmt(&self, _n: &AstView::TryStmt, _ctx: &mut Context) {}
  fn ts_array_type(&self, _n: &AstView::TsArrayType, _ctx: &mut Context) {}
  fn ts_as_expr(&self, _n: &AstView::TsAsExpr, _ctx: &mut Context) {}
  fn ts_call_signature_decl(
    &self,
    _n: &AstView::TsCallSignatureDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_conditional_type(
    &self,
    _n: &AstView::TsConditionalType,
    _ctx: &mut Context,
  ) {
  }
  fn ts_const_assertion(
    &self,
    _n: &AstView::TsConstAssertion,
    _ctx: &mut Context,
  ) {
  }
  fn ts_construct_signature_decl(
    &self,
    _n: &AstView::TsConstructSignatureDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_constructor_type(
    &self,
    _n: &AstView::TsConstructorType,
    _ctx: &mut Context,
  ) {
  }
  fn ts_enum_decl(&self, _n: &AstView::TsEnumDecl, _ctx: &mut Context) {}
  fn ts_enum_member(&self, _n: &AstView::TsEnumMember, _ctx: &mut Context) {}
  fn ts_export_assignment(
    &self,
    _n: &AstView::TsExportAssignment,
    _ctx: &mut Context,
  ) {
  }
  fn ts_expr_with_type_args(
    &self,
    _n: &AstView::TsExprWithTypeArgs,
    _ctx: &mut Context,
  ) {
  }
  fn ts_external_module_ref(
    &self,
    _n: &AstView::TsExternalModuleRef,
    _ctx: &mut Context,
  ) {
  }
  fn ts_fn_type(&self, _n: &AstView::TsFnType, _ctx: &mut Context) {}
  fn ts_import_equal_decl(
    &self,
    _n: &AstView::TsImportEqualsDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_import_type(&self, _n: &AstView::TsImportType, _ctx: &mut Context) {}
  fn ts_index_signature(
    &self,
    _n: &AstView::TsIndexSignature,
    _ctx: &mut Context,
  ) {
  }
  fn ts_indexed_access_type(
    &self,
    _n: &AstView::TsIndexedAccessType,
    _ctx: &mut Context,
  ) {
  }
  fn ts_infer_type(&self, _n: &AstView::TsInferType, _ctx: &mut Context) {}
  fn ts_interface_body(
    &self,
    _n: &AstView::TsInterfaceBody,
    _ctx: &mut Context,
  ) {
  }
  fn ts_interface_decl(
    &self,
    _n: &AstView::TsInterfaceDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_intersection_type(
    &self,
    _n: &AstView::TsIntersectionType,
    _ctx: &mut Context,
  ) {
  }
  fn ts_keyword_type(&self, _n: &AstView::TsKeywordType, _ctx: &mut Context) {}
  fn ts_lit_type(&self, _n: &AstView::TsLitType, _ctx: &mut Context) {}
  fn ts_mapped_type(&self, _n: &AstView::TsMappedType, _ctx: &mut Context) {}
  fn ts_method_signature(
    &self,
    _n: &AstView::TsMethodSignature,
    _ctx: &mut Context,
  ) {
  }
  fn ts_module_block(&self, _n: &AstView::TsModuleBlock, _ctx: &mut Context) {}
  fn ts_module_decl(&self, _n: &AstView::TsModuleDecl, _ctx: &mut Context) {}
  fn ts_namespace_decl(
    &self,
    _n: &AstView::TsNamespaceDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_namespace_export_decl(
    &self,
    _n: &AstView::TsNamespaceExportDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_non_null_expr(&self, _n: &AstView::TsNonNullExpr, _ctx: &mut Context) {}
  fn ts_optional_type(&self, _n: &AstView::TsOptionalType, _ctx: &mut Context) {
  }
  fn ts_param_prop(&self, _n: &AstView::TsParamProp, _ctx: &mut Context) {}
  fn ts_parenthesized_type(
    &self,
    _n: &AstView::TsParenthesizedType,
    _ctx: &mut Context,
  ) {
  }
  fn ts_property_signature(
    &self,
    _n: &AstView::TsPropertySignature,
    _ctx: &mut Context,
  ) {
  }
  fn ts_qualified_name(
    &self,
    _n: &AstView::TsQualifiedName,
    _ctx: &mut Context,
  ) {
  }
  fn ts_rest_type(&self, _n: &AstView::TsRestType, _ctx: &mut Context) {}
  fn ts_this_type(&self, _n: &AstView::TsThisType, _ctx: &mut Context) {}
  fn ts_tpl_lit_type(&self, _n: &AstView::TsTplLitType, _ctx: &mut Context) {}
  fn ts_tuple_element(&self, _n: &AstView::TsTupleElement, _ctx: &mut Context) {
  }
  fn ts_tuple_type(&self, _n: &AstView::TsTupleType, _ctx: &mut Context) {}
  fn ts_type_alias_decl(
    &self,
    _n: &AstView::TsTypeAliasDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_type_ann(&self, _n: &AstView::TsTypeAnn, _ctx: &mut Context) {}
  fn ts_type_assertion(
    &self,
    _n: &AstView::TsTypeAssertion,
    _ctx: &mut Context,
  ) {
  }
  fn ts_type_lit(&self, _n: &AstView::TsTypeLit, _ctx: &mut Context) {}
  fn ts_type_operator(&self, _n: &AstView::TsTypeOperator, _ctx: &mut Context) {
  }
  fn ts_type_param(&self, _n: &AstView::TsTypeParam, _ctx: &mut Context) {}
  fn ts_type_param_decl(
    &self,
    _n: &AstView::TsTypeParamDecl,
    _ctx: &mut Context,
  ) {
  }
  fn ts_type_param_instantiation(
    &self,
    _n: &AstView::TsTypeParamInstantiation,
    _ctx: &mut Context,
  ) {
  }
  fn ts_type_predicate(
    &self,
    _n: &AstView::TsTypePredicate,
    _ctx: &mut Context,
  ) {
  }
  fn ts_type_query(&self, _n: &AstView::TsTypeQuery, _ctx: &mut Context) {}
  fn ts_type_ref(&self, _n: &AstView::TsTypeRef, _ctx: &mut Context) {}
  fn ts_union_type(&self, _n: &AstView::TsUnionType, _ctx: &mut Context) {}
  fn unary_expr(&self, _n: &AstView::UnaryExpr, _ctx: &mut Context) {}
  fn update_expr(&self, _n: &AstView::UpdateExpr, _ctx: &mut Context) {}
  fn var_decl(&self, _n: &AstView::VarDecl, _ctx: &mut Context) {}
  fn var_declarator(&self, _n: &AstView::VarDeclarator, _ctx: &mut Context) {}
  fn while_stmt(&self, _n: &AstView::WhileStmt, _ctx: &mut Context) {}
  fn with_stmt(&self, _n: &AstView::WithStmt, _ctx: &mut Context) {}
  fn yield_expr(&self, _n: &AstView::YieldExpr, _ctx: &mut Context) {}
}

pub trait Traverse: Handler {
  fn traverse<'a, N>(&self, node: N, ctx: &mut Context)
  where
    N: NodeTrait<'a>,
  {
    use AstView::Node::*;
    match node.into_node() {
      ArrayLit(n) => {
        self.array_lit(n, ctx);
      }
      ArrayPat(n) => {
        self.array_pat(n, ctx);
      }
      ArrowExpr(n) => {
        self.arrow_expr(n, ctx);
      }
      AssignExpr(n) => {
        self.assign_expr(n, ctx);
      }
      AssignPat(n) => {
        self.assign_pat(n, ctx);
      }
      AssignPatProp(n) => {
        self.assign_pat_prop(n, ctx);
      }
      AssignProp(n) => {
        self.assign_prop(n, ctx);
      }
      AwaitExpr(n) => {
        self.await_expr(n, ctx);
      }
      BigInt(n) => {
        self.big_int(n, ctx);
      }
      BinExpr(n) => {
        self.bin_expr(n, ctx);
      }
      BindingIdent(n) => {
        self.binding_ident(n, ctx);
      }
      BlockStmt(n) => {
        self.block_stmt(n, ctx);
      }
      Bool(n) => {
        self.bool(n, ctx);
      }
      BreakStmt(n) => {
        self.break_stmt(n, ctx);
      }
      CallExpr(n) => {
        self.call_expr(n, ctx);
      }
      CatchClause(n) => {
        self.catch_clause(n, ctx);
      }
      Class(n) => {
        self.class(n, ctx);
      }
      ClassDecl(n) => {
        self.class_decl(n, ctx);
      }
      ClassExpr(n) => {
        self.class_expr(n, ctx);
      }
      ClassMethod(n) => {
        self.class_method(n, ctx);
      }
      ClassProp(n) => {
        self.class_prop(n, ctx);
      }
      ComputedPropName(n) => {
        self.computed_prop_name(n, ctx);
      }
      CondExpr(n) => {
        self.cond_expr(n, ctx);
      }
      Constructor(n) => {
        self.constructor(n, ctx);
      }
      ContinueStmt(n) => {
        self.continue_stmt(n, ctx);
      }
      DebuggerStmt(n) => {
        self.debugger_stmt(n, ctx);
      }
      Decorator(n) => {
        self.decorator(n, ctx);
      }
      DoWhileStmt(n) => {
        self.do_while_stmt(n, ctx);
      }
      EmptyStmt(n) => {
        self.empty_stmt(n, ctx);
      }
      ExportAll(n) => {
        self.export_all(n, ctx);
      }
      ExportDecl(n) => {
        self.export_decl(n, ctx);
      }
      ExportDefaultDecl(n) => {
        self.export_default_decl(n, ctx);
      }
      ExportDefaultExpr(n) => {
        self.export_default_expr(n, ctx);
      }
      ExportDefaultSpecifier(n) => {
        self.export_default_specifier(n, ctx);
      }
      ExportNamedSpecifier(n) => {
        self.export_named_specifier(n, ctx);
      }
      ExportNamespaceSpecifier(n) => {
        self.export_namespace_specifier(n, ctx);
      }
      ExprOrSpread(n) => {
        self.expr_or_spread(n, ctx);
      }
      ExprStmt(n) => {
        self.expr_stmt(n, ctx);
      }
      FnDecl(n) => {
        self.fn_decl(n, ctx);
      }
      FnExpr(n) => {
        self.fn_expr(n, ctx);
      }
      ForInStmt(n) => {
        self.for_in_stmt(n, ctx);
      }
      ForOfStmt(n) => {
        self.for_of_stmt(n, ctx);
      }
      ForStmt(n) => {
        self.for_stmt(n, ctx);
      }
      Function(n) => {
        self.function(n, ctx);
      }
      GetterProp(n) => {
        self.getter_prop(n, ctx);
      }
      Ident(n) => {
        self.ident(n, ctx);
      }
      IfStmt(n) => {
        self.if_stmt(n, ctx);
      }
      ImportDecl(n) => {
        self.import_decl(n, ctx);
      }
      ImportDefaultSpecifier(n) => {
        self.import_default_specifier(n, ctx);
      }
      ImportNamedSpecifier(n) => {
        self.import_named_specifier(n, ctx);
      }
      ImportStarAsSpecifier(n) => {
        self.import_star_as_specifier(n, ctx);
      }
      Invalid(n) => {
        self.invalid(n, ctx);
      }
      JSXAttr(n) => {
        self.jsx_attr(n, ctx);
      }
      JSXClosingElement(n) => {
        self.jsx_closing_element(n, ctx);
      }
      JSXClosingFragment(n) => {
        self.jsx_closing_fragment(n, ctx);
      }
      JSXElement(n) => {
        self.jsx_element(n, ctx);
      }
      JSXEmptyExpr(n) => {
        self.jsx_empty_expr(n, ctx);
      }
      JSXExprContainer(n) => {
        self.jsx_expr_container(n, ctx);
      }
      JSXFragment(n) => {
        self.jsx_fragment(n, ctx);
      }
      JSXMemberExpr(n) => {
        self.jsx_member_expr(n, ctx);
      }
      JSXNamespacedName(n) => {
        self.jsx_namespaced_name(n, ctx);
      }
      JSXOpeningElement(n) => {
        self.jsx_opening_element(n, ctx);
      }
      JSXOpeningFragment(n) => {
        self.jsx_opening_fragment(n, ctx);
      }
      JSXSpreadChild(n) => {
        self.jsx_spread_child(n, ctx);
      }
      JSXText(n) => {
        self.jsx_text(n, ctx);
      }
      KeyValuePatProp(n) => {
        self.key_value_pat_prop(n, ctx);
      }
      KeyValueProp(n) => {
        self.key_value_prop(n, ctx);
      }
      LabeledStmt(n) => {
        self.labeled_stmt(n, ctx);
      }
      MemberExpr(n) => {
        self.member_expr(n, ctx);
      }
      MetaPropExpr(n) => {
        self.meta_prop_expr(n, ctx);
      }
      MethodProp(n) => {
        self.method_prop(n, ctx);
      }
      Module(n) => {
        self.module(n, ctx);
      }
      NamedExport(n) => {
        self.named_export(n, ctx);
      }
      NewExpr(n) => {
        self.new_expr(n, ctx);
      }
      Null(n) => {
        self.null(n, ctx);
      }
      Number(n) => {
        self.number(n, ctx);
      }
      ObjectLit(n) => {
        self.object_lit(n, ctx);
      }
      ObjectPat(n) => {
        self.object_pat(n, ctx);
      }
      OptChainExpr(n) => {
        self.opt_chain_expr(n, ctx);
      }
      Param(n) => {
        self.param(n, ctx);
      }
      ParenExpr(n) => {
        self.paren_expr(n, ctx);
      }
      PrivateMethod(n) => {
        self.private_method(n, ctx);
      }
      PrivateName(n) => {
        self.private_name(n, ctx);
      }
      PrivateProp(n) => {
        self.private_prop(n, ctx);
      }
      Regex(n) => {
        self.regex(n, ctx);
      }
      RestPat(n) => {
        self.rest_pat(n, ctx);
      }
      ReturnStmt(n) => {
        self.return_stmt(n, ctx);
      }
      Script(n) => {
        self.script(n, ctx);
      }
      SeqExpr(n) => {
        self.seq_expr(n, ctx);
      }
      SetterProp(n) => {
        self.setter_prop(n, ctx);
      }
      SpreadElement(n) => {
        self.spread_element(n, ctx);
      }
      Str(n) => {
        self.str(n, ctx);
      }
      Super(n) => {
        self.super_(n, ctx);
      }
      SwitchCase(n) => {
        self.switch_case(n, ctx);
      }
      SwitchStmt(n) => {
        self.switch_stmt(n, ctx);
      }
      TaggedTpl(n) => {
        self.tagged_tpl(n, ctx);
      }
      ThisExpr(n) => {
        self.this_expr(n, ctx);
      }
      ThrowStmt(n) => {
        self.throw_stmt(n, ctx);
      }
      Tpl(n) => {
        self.tpl(n, ctx);
      }
      TplElement(n) => {
        self.tpl_element(n, ctx);
      }
      TryStmt(n) => {
        self.try_stmt(n, ctx);
      }
      TsArrayType(n) => {
        self.ts_array_type(n, ctx);
      }
      TsAsExpr(n) => {
        self.ts_as_expr(n, ctx);
      }
      TsCallSignatureDecl(n) => {
        self.ts_call_signature_decl(n, ctx);
      }
      TsConditionalType(n) => {
        self.ts_conditional_type(n, ctx);
      }
      TsConstAssertion(n) => {
        self.ts_const_assertion(n, ctx);
      }
      TsConstructSignatureDecl(n) => {
        self.ts_construct_signature_decl(n, ctx);
      }
      TsConstructorType(n) => {
        self.ts_constructor_type(n, ctx);
      }
      TsEnumDecl(n) => {
        self.ts_enum_decl(n, ctx);
      }
      TsEnumMember(n) => {
        self.ts_enum_member(n, ctx);
      }
      TsExportAssignment(n) => {
        self.ts_export_assignment(n, ctx);
      }
      TsExprWithTypeArgs(n) => {
        self.ts_expr_with_type_args(n, ctx);
      }
      TsExternalModuleRef(n) => {
        self.ts_external_module_ref(n, ctx);
      }
      TsFnType(n) => {
        self.ts_fn_type(n, ctx);
      }
      TsImportEqualsDecl(n) => {
        self.ts_import_equal_decl(n, ctx);
      }
      TsImportType(n) => {
        self.ts_import_type(n, ctx);
      }
      TsIndexSignature(n) => {
        self.ts_index_signature(n, ctx);
      }
      TsIndexedAccessType(n) => {
        self.ts_indexed_access_type(n, ctx);
      }
      TsInferType(n) => {
        self.ts_infer_type(n, ctx);
      }
      TsInterfaceBody(n) => {
        self.ts_interface_body(n, ctx);
      }
      TsInterfaceDecl(n) => {
        self.ts_interface_decl(n, ctx);
      }
      TsIntersectionType(n) => {
        self.ts_intersection_type(n, ctx);
      }
      TsKeywordType(n) => {
        self.ts_keyword_type(n, ctx);
      }
      TsLitType(n) => {
        self.ts_lit_type(n, ctx);
      }
      TsMappedType(n) => {
        self.ts_mapped_type(n, ctx);
      }
      TsMethodSignature(n) => {
        self.ts_method_signature(n, ctx);
      }
      TsModuleBlock(n) => {
        self.ts_module_block(n, ctx);
      }
      TsModuleDecl(n) => {
        self.ts_module_decl(n, ctx);
      }
      TsNamespaceDecl(n) => {
        self.ts_namespace_decl(n, ctx);
      }
      TsNamespaceExportDecl(n) => {
        self.ts_namespace_export_decl(n, ctx);
      }
      TsNonNullExpr(n) => {
        self.ts_non_null_expr(n, ctx);
      }
      TsOptionalType(n) => {
        self.ts_optional_type(n, ctx);
      }
      TsParamProp(n) => {
        self.ts_param_prop(n, ctx);
      }
      TsParenthesizedType(n) => {
        self.ts_parenthesized_type(n, ctx);
      }
      TsPropertySignature(n) => {
        self.ts_property_signature(n, ctx);
      }
      TsQualifiedName(n) => {
        self.ts_qualified_name(n, ctx);
      }
      TsRestType(n) => {
        self.ts_rest_type(n, ctx);
      }
      TsThisType(n) => {
        self.ts_this_type(n, ctx);
      }
      TsTplLitType(n) => {
        self.ts_tpl_lit_type(n, ctx);
      }
      TsTupleElement(n) => {
        self.ts_tuple_element(n, ctx);
      }
      TsTupleType(n) => {
        self.ts_tuple_type(n, ctx);
      }
      TsTypeAliasDecl(n) => {
        self.ts_type_alias_decl(n, ctx);
      }
      TsTypeAnn(n) => {
        self.ts_type_ann(n, ctx);
      }
      TsTypeAssertion(n) => {
        self.ts_type_assertion(n, ctx);
      }
      TsTypeLit(n) => {
        self.ts_type_lit(n, ctx);
      }
      TsTypeOperator(n) => {
        self.ts_type_operator(n, ctx);
      }
      TsTypeParam(n) => {
        self.ts_type_param(n, ctx);
      }
      TsTypeParamDecl(n) => {
        self.ts_type_param_decl(n, ctx);
      }
      TsTypeParamInstantiation(n) => {
        self.ts_type_param_instantiation(n, ctx);
      }
      TsTypePredicate(n) => {
        self.ts_type_predicate(n, ctx);
      }
      TsTypeQuery(n) => {
        self.ts_type_query(n, ctx);
      }
      TsTypeRef(n) => {
        self.ts_type_ref(n, ctx);
      }
      TsUnionType(n) => {
        self.ts_union_type(n, ctx);
      }
      UnaryExpr(n) => {
        self.unary_expr(n, ctx);
      }
      UpdateExpr(n) => {
        self.update_expr(n, ctx);
      }
      VarDecl(n) => {
        self.var_decl(n, ctx);
      }
      VarDeclarator(n) => {
        self.var_declarator(n, ctx);
      }
      WhileStmt(n) => {
        self.while_stmt(n, ctx);
      }
      WithStmt(n) => {
        self.with_stmt(n, ctx);
      }
      YieldExpr(n) => {
        self.yield_expr(n, ctx);
      }
    }

    for child in node.children() {
      self.traverse(child, ctx);
    }
  }
}

impl<H: Handler> Traverse for H {}
