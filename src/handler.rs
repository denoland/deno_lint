use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait};

pub trait Handler {
  fn array_lit(&mut self, _n: &AstView::ArrayLit) {}
  fn array_pat(&mut self, _n: &AstView::ArrayPat) {}
  fn arrow_expr(&mut self, _n: &AstView::ArrowExpr) {}
  fn assign_expr(&mut self, _n: &AstView::AssignExpr) {}
  fn assign_pat(&mut self, _n: &AstView::AssignPat) {}
  fn assign_pat_prop(&mut self, _n: &AstView::AssignPatProp) {}
  fn assign_prop(&mut self, _n: &AstView::AssignProp) {}
  fn await_expr(&mut self, _n: &AstView::AwaitExpr) {}
  fn big_int(&mut self, _n: &AstView::BigInt) {}
  fn bin_expr(&mut self, _n: &AstView::BinExpr) {}
  fn block_stmt(&mut self, _n: &AstView::BlockStmt) {}
  fn bool(&mut self, _n: &AstView::Bool) {}
  fn break_stmt(&mut self, _n: &AstView::BreakStmt) {}
  fn call_expr(&mut self, _n: &AstView::CallExpr) {}
  fn catch_clause(&mut self, _n: &AstView::CatchClause) {}
  fn class(&mut self, _n: &AstView::Class) {}
  fn class_decl(&mut self, _n: &AstView::ClassDecl) {}
  fn class_expr(&mut self, _n: &AstView::ClassExpr) {}
  fn class_method(&mut self, _n: &AstView::ClassMethod) {}
  fn class_prop(&mut self, _n: &AstView::ClassProp) {}
  fn computed_prop_name(&mut self, _n: &AstView::ComputedPropName) {}
  fn cond_expr(&mut self, _n: &AstView::CondExpr) {}
  fn constructor(&mut self, _n: &AstView::Constructor) {}
  fn continue_stmt(&mut self, _n: &AstView::ContinueStmt) {}
  fn debugger_stmt(&mut self, _n: &AstView::DebuggerStmt) {}
  fn decorator(&mut self, _n: &AstView::Decorator) {}
  fn do_while_stmt(&mut self, _n: &AstView::DoWhileStmt) {}
  fn empty_stmt(&mut self, _n: &AstView::EmptyStmt) {}
  fn export_all(&mut self, _n: &AstView::ExportAll) {}
  fn export_decl(&mut self, _n: &AstView::ExportDecl) {}
  fn export_default_decl(&mut self, _n: &AstView::ExportDefaultDecl) {}
  fn export_default_expr(&mut self, _n: &AstView::ExportDefaultExpr) {}
  fn export_default_specifier(&mut self, _n: &AstView::ExportDefaultSpecifier) {
  }
  fn export_named_specifier(&mut self, _n: &AstView::ExportNamedSpecifier) {}
  fn export_namespace_specifier(
    &mut self,
    _n: &AstView::ExportNamespaceSpecifier,
  ) {
  }
  fn expr_or_spread(&mut self, _n: &AstView::ExprOrSpread) {}
  fn expr_stmt(&mut self, _n: &AstView::ExprStmt) {}
  fn fn_decl(&mut self, _n: &AstView::FnDecl) {}
  fn fn_expr(&mut self, _n: &AstView::FnExpr) {}
  fn for_in_stmt(&mut self, _n: &AstView::ForInStmt) {}
  fn for_of_stmt(&mut self, _n: &AstView::ForOfStmt) {}
  fn for_stmt(&mut self, _n: &AstView::ForStmt) {}
  fn function(&mut self, _n: &AstView::Function) {}
  fn getter_prop(&mut self, _n: &AstView::GetterProp) {}
  fn ident(&mut self, _n: &AstView::Ident) {}
  fn if_stmt(&mut self, _n: &AstView::IfStmt) {}
  fn import_decl(&mut self, _n: &AstView::ImportDecl) {}
  fn import_default_specifier(&mut self, _n: &AstView::ImportDefaultSpecifier) {
  }
  fn import_named_specifier(&mut self, _n: &AstView::ImportNamedSpecifier) {}
  fn import_star_as_specifier(&mut self, _n: &AstView::ImportStarAsSpecifier) {}
  fn invalid(&mut self, _n: &AstView::Invalid) {}
  fn jsx_attr(&mut self, _n: &AstView::JSXAttr) {}
  fn jsx_closing_element(&mut self, _n: &AstView::JSXClosingElement) {}
  fn jsx_closing_fragment(&mut self, _n: &AstView::JSXClosingFragment) {}
  fn jsx_element(&mut self, _n: &AstView::JSXElement) {}
  fn jsx_empty_expr(&mut self, _n: &AstView::JSXEmptyExpr) {}
  fn jsx_expr_container(&mut self, _n: &AstView::JSXExprContainer) {}
  fn jsx_fragment(&mut self, _n: &AstView::JSXFragment) {}
  fn jsx_member_expr(&mut self, _n: &AstView::JSXMemberExpr) {}
  fn jsx_namespaced_name(&mut self, _n: &AstView::JSXNamespacedName) {}
  fn jsx_opening_element(&mut self, _n: &AstView::JSXOpeningElement) {}
  fn jsx_opening_fragment(&mut self, _n: &AstView::JSXOpeningFragment) {}
  fn jsx_spread_child(&mut self, _n: &AstView::JSXSpreadChild) {}
  fn jsx_text(&mut self, _n: &AstView::JSXText) {}
  fn key_value_pat_prop(&mut self, _n: &AstView::KeyValuePatProp) {}
  fn key_value_prop(&mut self, _n: &AstView::KeyValueProp) {}
  fn labeled_stmt(&mut self, _n: &AstView::LabeledStmt) {}
  fn member_expr(&mut self, _n: &AstView::MemberExpr) {}
  fn meta_prop_expr(&mut self, _n: &AstView::MetaPropExpr) {}
  fn method_prop(&mut self, _n: &AstView::MethodProp) {}
  fn module(&mut self, _n: &AstView::Module) {}
  fn named_export(&mut self, _n: &AstView::NamedExport) {}
  fn new_expr(&mut self, _n: &AstView::NewExpr) {}
  fn null(&mut self, _n: &AstView::Null) {}
  fn number(&mut self, _n: &AstView::Number) {}
  fn object_lit(&mut self, _n: &AstView::ObjectLit) {}
  fn object_pat(&mut self, _n: &AstView::ObjectPat) {}
  fn opt_chain_expr(&mut self, _n: &AstView::OptChainExpr) {}
  fn param(&mut self, _n: &AstView::Param) {}
  fn paren_expr(&mut self, _n: &AstView::ParenExpr) {}
  fn private_method(&mut self, _n: &AstView::PrivateMethod) {}
  fn private_name(&mut self, _n: &AstView::PrivateName) {}
  fn private_prop(&mut self, _n: &AstView::PrivateProp) {}
  fn regex(&mut self, _n: &AstView::Regex) {}
  fn rest_pat(&mut self, _n: &AstView::RestPat) {}
  fn return_stmt(&mut self, _n: &AstView::ReturnStmt) {}
  fn script(&mut self, _n: &AstView::Script) {}
  fn seq_expr(&mut self, _n: &AstView::SeqExpr) {}
  fn setter_prop(&mut self, _n: &AstView::SetterProp) {}
  fn spread_element(&mut self, _n: &AstView::SpreadElement) {}
  fn str(&mut self, _n: &AstView::Str) {}
  // Neither `super` or `r#super` can be used here, so we use `super_` reluctantly
  fn super_(&mut self, _n: &AstView::Super) {}
  fn switch_case(&mut self, _n: &AstView::SwitchCase) {}
  fn switch_stmt(&mut self, _n: &AstView::SwitchStmt) {}
  fn tagged_tpl(&mut self, _n: &AstView::TaggedTpl) {}
  fn this_expr(&mut self, _n: &AstView::ThisExpr) {}
  fn throw_stmt(&mut self, _n: &AstView::ThrowStmt) {}
  fn tpl(&mut self, _n: &AstView::Tpl) {}
  fn tpl_element(&mut self, _n: &AstView::TplElement) {}
  fn try_stmt(&mut self, _n: &AstView::TryStmt) {}
  fn ts_array_type(&mut self, _n: &AstView::TsArrayType) {}
  fn ts_as_expr(&mut self, _n: &AstView::TsAsExpr) {}
  fn ts_call_signature_decl(&mut self, _n: &AstView::TsCallSignatureDecl) {}
  fn ts_conditional_type(&mut self, _n: &AstView::TsConditionalType) {}
  fn ts_const_assertion(&mut self, _n: &AstView::TsConstAssertion) {}
  fn ts_construct_signature_decl(
    &mut self,
    _n: &AstView::TsConstructSignatureDecl,
  ) {
  }
  fn ts_constructor_type(&mut self, _n: &AstView::TsConstructorType) {}
  fn ts_enum_decl(&mut self, _n: &AstView::TsEnumDecl) {}
  fn ts_enum_member(&mut self, _n: &AstView::TsEnumMember) {}
  fn ts_export_assignment(&mut self, _n: &AstView::TsExportAssignment) {}
  fn ts_expr_with_type_args(&mut self, _n: &AstView::TsExprWithTypeArgs) {}
  fn ts_external_module_ref(&mut self, _n: &AstView::TsExternalModuleRef) {}
  fn ts_fn_type(&mut self, _n: &AstView::TsFnType) {}
  fn ts_import_equal_decl(&mut self, _n: &AstView::TsImportEqualsDecl) {}
  fn ts_import_type(&mut self, _n: &AstView::TsImportType) {}
  fn ts_index_signature(&mut self, _n: &AstView::TsIndexSignature) {}
  fn ts_indexed_access_type(&mut self, _n: &AstView::TsIndexedAccessType) {}
  fn ts_infer_type(&mut self, _n: &AstView::TsInferType) {}
  fn ts_interface_body(&mut self, _n: &AstView::TsInterfaceBody) {}
  fn ts_interface_decl(&mut self, _n: &AstView::TsInterfaceDecl) {}
  fn ts_intersection_type(&mut self, _n: &AstView::TsIntersectionType) {}
  fn ts_keyword_type(&mut self, _n: &AstView::TsKeywordType) {}
  fn ts_lit_type(&mut self, _n: &AstView::TsLitType) {}
  fn ts_mapped_type(&mut self, _n: &AstView::TsMappedType) {}
  fn ts_method_signature(&mut self, _n: &AstView::TsMethodSignature) {}
  fn ts_module_block(&mut self, _n: &AstView::TsModuleBlock) {}
  fn ts_module_decl(&mut self, _n: &AstView::TsModuleDecl) {}
  fn ts_namespace_decl(&mut self, _n: &AstView::TsNamespaceDecl) {}
  fn ts_namespace_export_decl(&mut self, _n: &AstView::TsNamespaceExportDecl) {}
  fn ts_non_null_expr(&mut self, _n: &AstView::TsNonNullExpr) {}
  fn ts_optional_type(&mut self, _n: &AstView::TsOptionalType) {}
  fn ts_param_prop(&mut self, _n: &AstView::TsParamProp) {}
  fn ts_parenthesized_type(&mut self, _n: &AstView::TsParenthesizedType) {}
  fn ts_property_signature(&mut self, _n: &AstView::TsPropertySignature) {}
  fn ts_qualified_name(&mut self, _n: &AstView::TsQualifiedName) {}
  fn ts_rest_type(&mut self, _n: &AstView::TsRestType) {}
  fn ts_this_type(&mut self, _n: &AstView::TsThisType) {}
  fn ts_tpl_lit_type(&mut self, _n: &AstView::TsTplLitType) {}
  fn ts_tuple_element(&mut self, _n: &AstView::TsTupleElement) {}
  fn ts_tuple_type(&mut self, _n: &AstView::TsTupleType) {}
  fn ts_type_alias_decl(&mut self, _n: &AstView::TsTypeAliasDecl) {}
  fn ts_type_ann(&mut self, _n: &AstView::TsTypeAnn) {}
  fn ts_type_assertion(&mut self, _n: &AstView::TsTypeAssertion) {}
  fn ts_type_cast_expr(&mut self, _n: &AstView::TsTypeCastExpr) {}
  fn ts_type_lit(&mut self, _n: &AstView::TsTypeLit) {}
  fn ts_type_operator(&mut self, _n: &AstView::TsTypeOperator) {}
  fn ts_type_param(&mut self, _n: &AstView::TsTypeParam) {}
  fn ts_type_param_decl(&mut self, _n: &AstView::TsTypeParamDecl) {}
  fn ts_type_param_instantiation(
    &mut self,
    _n: &AstView::TsTypeParamInstantiation,
  ) {
  }
  fn ts_type_predicate(&mut self, _n: &AstView::TsTypePredicate) {}
  fn ts_type_query(&mut self, _n: &AstView::TsTypeQuery) {}
  fn ts_type_ref(&mut self, _n: &AstView::TsTypeRef) {}
  fn ts_union_type(&mut self, _n: &AstView::TsUnionType) {}
  fn unary_expr(&mut self, _n: &AstView::UnaryExpr) {}
  fn update_expr(&mut self, _n: &AstView::UpdateExpr) {}
  fn var_decl(&mut self, _n: &AstView::VarDecl) {}
  fn var_declarator(&mut self, _n: &AstView::VarDeclarator) {}
  fn while_stmt(&mut self, _n: &AstView::WhileStmt) {}
  fn with_stmt(&mut self, _n: &AstView::WithStmt) {}
  fn yield_expr(&mut self, _n: &AstView::YieldExpr) {}
}

pub trait Traverse: Handler {
  fn traverse<'a, N>(&mut self, node: N)
  where
    N: NodeTrait<'a>,
  {
    use AstView::Node::*;
    match node.into_node() {
      ArrayLit(n) => {
        self.array_lit(n);
      }
      ArrayPat(n) => {
        self.array_pat(n);
      }
      ArrowExpr(n) => {
        self.arrow_expr(n);
      }
      AssignExpr(n) => {
        self.assign_expr(n);
      }
      AssignPat(n) => {
        self.assign_pat(n);
      }
      AssignPatProp(n) => {
        self.assign_pat_prop(n);
      }
      AssignProp(n) => {
        self.assign_prop(n);
      }
      AwaitExpr(n) => {
        self.await_expr(n);
      }
      BigInt(n) => {
        self.big_int(n);
      }
      BinExpr(n) => {
        self.bin_expr(n);
      }
      BlockStmt(n) => {
        self.block_stmt(n);
      }
      Bool(n) => {
        self.bool(n);
      }
      BreakStmt(n) => {
        self.break_stmt(n);
      }
      CallExpr(n) => {
        self.call_expr(n);
      }
      CatchClause(n) => {
        self.catch_clause(n);
      }
      Class(n) => {
        self.class(n);
      }
      ClassDecl(n) => {
        self.class_decl(n);
      }
      ClassExpr(n) => {
        self.class_expr(n);
      }
      ClassMethod(n) => {
        self.class_method(n);
      }
      ClassProp(n) => {
        self.class_prop(n);
      }
      ComputedPropName(n) => {
        self.computed_prop_name(n);
      }
      CondExpr(n) => {
        self.cond_expr(n);
      }
      Constructor(n) => {
        self.constructor(n);
      }
      ContinueStmt(n) => {
        self.continue_stmt(n);
      }
      DebuggerStmt(n) => {
        self.debugger_stmt(n);
      }
      Decorator(n) => {
        self.decorator(n);
      }
      DoWhileStmt(n) => {
        self.do_while_stmt(n);
      }
      EmptyStmt(n) => {
        self.empty_stmt(n);
      }
      ExportAll(n) => {
        self.export_all(n);
      }
      ExportDecl(n) => {
        self.export_decl(n);
      }
      ExportDefaultDecl(n) => {
        self.export_default_decl(n);
      }
      ExportDefaultExpr(n) => {
        self.export_default_expr(n);
      }
      ExportDefaultSpecifier(n) => {
        self.export_default_specifier(n);
      }
      ExportNamedSpecifier(n) => {
        self.export_named_specifier(n);
      }
      ExportNamespaceSpecifier(n) => {
        self.export_namespace_specifier(n);
      }
      ExprOrSpread(n) => {
        self.expr_or_spread(n);
      }
      ExprStmt(n) => {
        self.expr_stmt(n);
      }
      FnDecl(n) => {
        self.fn_decl(n);
      }
      FnExpr(n) => {
        self.fn_expr(n);
      }
      ForInStmt(n) => {
        self.for_in_stmt(n);
      }
      ForOfStmt(n) => {
        self.for_of_stmt(n);
      }
      ForStmt(n) => {
        self.for_stmt(n);
      }
      Function(n) => {
        self.function(n);
      }
      GetterProp(n) => {
        self.getter_prop(n);
      }
      Ident(n) => {
        self.ident(n);
      }
      IfStmt(n) => {
        self.if_stmt(n);
      }
      ImportDecl(n) => {
        self.import_decl(n);
      }
      ImportDefaultSpecifier(n) => {
        self.import_default_specifier(n);
      }
      ImportNamedSpecifier(n) => {
        self.import_named_specifier(n);
      }
      ImportStarAsSpecifier(n) => {
        self.import_star_as_specifier(n);
      }
      Invalid(n) => {
        self.invalid(n);
      }
      JSXAttr(n) => {
        self.jsx_attr(n);
      }
      JSXClosingElement(n) => {
        self.jsx_closing_element(n);
      }
      JSXClosingFragment(n) => {
        self.jsx_closing_fragment(n);
      }
      JSXElement(n) => {
        self.jsx_element(n);
      }
      JSXEmptyExpr(n) => {
        self.jsx_empty_expr(n);
      }
      JSXExprContainer(n) => {
        self.jsx_expr_container(n);
      }
      JSXFragment(n) => {
        self.jsx_fragment(n);
      }
      JSXMemberExpr(n) => {
        self.jsx_member_expr(n);
      }
      JSXNamespacedName(n) => {
        self.jsx_namespaced_name(n);
      }
      JSXOpeningElement(n) => {
        self.jsx_opening_element(n);
      }
      JSXOpeningFragment(n) => {
        self.jsx_opening_fragment(n);
      }
      JSXSpreadChild(n) => {
        self.jsx_spread_child(n);
      }
      JSXText(n) => {
        self.jsx_text(n);
      }
      KeyValuePatProp(n) => {
        self.key_value_pat_prop(n);
      }
      KeyValueProp(n) => {
        self.key_value_prop(n);
      }
      LabeledStmt(n) => {
        self.labeled_stmt(n);
      }
      MemberExpr(n) => {
        self.member_expr(n);
      }
      MetaPropExpr(n) => {
        self.meta_prop_expr(n);
      }
      MethodProp(n) => {
        self.method_prop(n);
      }
      Module(n) => {
        self.module(n);
      }
      NamedExport(n) => {
        self.named_export(n);
      }
      NewExpr(n) => {
        self.new_expr(n);
      }
      Null(n) => {
        self.null(n);
      }
      Number(n) => {
        self.number(n);
      }
      ObjectLit(n) => {
        self.object_lit(n);
      }
      ObjectPat(n) => {
        self.object_pat(n);
      }
      OptChainExpr(n) => {
        self.opt_chain_expr(n);
      }
      Param(n) => {
        self.param(n);
      }
      ParenExpr(n) => {
        self.paren_expr(n);
      }
      PrivateMethod(n) => {
        self.private_method(n);
      }
      PrivateName(n) => {
        self.private_name(n);
      }
      PrivateProp(n) => {
        self.private_prop(n);
      }
      Regex(n) => {
        self.regex(n);
      }
      RestPat(n) => {
        self.rest_pat(n);
      }
      ReturnStmt(n) => {
        self.return_stmt(n);
      }
      Script(n) => {
        self.script(n);
      }
      SeqExpr(n) => {
        self.seq_expr(n);
      }
      SetterProp(n) => {
        self.setter_prop(n);
      }
      SpreadElement(n) => {
        self.spread_element(n);
      }
      Str(n) => {
        self.str(n);
      }
      Super(n) => {
        self.super_(n);
      }
      SwitchCase(n) => {
        self.switch_case(n);
      }
      SwitchStmt(n) => {
        self.switch_stmt(n);
      }
      TaggedTpl(n) => {
        self.tagged_tpl(n);
      }
      ThisExpr(n) => {
        self.this_expr(n);
      }
      ThrowStmt(n) => {
        self.throw_stmt(n);
      }
      Tpl(n) => {
        self.tpl(n);
      }
      TplElement(n) => {
        self.tpl_element(n);
      }
      TryStmt(n) => {
        self.try_stmt(n);
      }
      TsArrayType(n) => {
        self.ts_array_type(n);
      }
      TsAsExpr(n) => {
        self.ts_as_expr(n);
      }
      TsCallSignatureDecl(n) => {
        self.ts_call_signature_decl(n);
      }
      TsConditionalType(n) => {
        self.ts_conditional_type(n);
      }
      TsConstAssertion(n) => {
        self.ts_const_assertion(n);
      }
      TsConstructSignatureDecl(n) => {
        self.ts_construct_signature_decl(n);
      }
      TsConstructorType(n) => {
        self.ts_constructor_type(n);
      }
      TsEnumDecl(n) => {
        self.ts_enum_decl(n);
      }
      TsEnumMember(n) => {
        self.ts_enum_member(n);
      }
      TsExportAssignment(n) => {
        self.ts_export_assignment(n);
      }
      TsExprWithTypeArgs(n) => {
        self.ts_expr_with_type_args(n);
      }
      TsExternalModuleRef(n) => {
        self.ts_external_module_ref(n);
      }
      TsFnType(n) => {
        self.ts_fn_type(n);
      }
      TsImportEqualsDecl(n) => {
        self.ts_import_equal_decl(n);
      }
      TsImportType(n) => {
        self.ts_import_type(n);
      }
      TsIndexSignature(n) => {
        self.ts_index_signature(n);
      }
      TsIndexedAccessType(n) => {
        self.ts_indexed_access_type(n);
      }
      TsInferType(n) => {
        self.ts_infer_type(n);
      }
      TsInterfaceBody(n) => {
        self.ts_interface_body(n);
      }
      TsInterfaceDecl(n) => {
        self.ts_interface_decl(n);
      }
      TsIntersectionType(n) => {
        self.ts_intersection_type(n);
      }
      TsKeywordType(n) => {
        self.ts_keyword_type(n);
      }
      TsLitType(n) => {
        self.ts_lit_type(n);
      }
      TsMappedType(n) => {
        self.ts_mapped_type(n);
      }
      TsMethodSignature(n) => {
        self.ts_method_signature(n);
      }
      TsModuleBlock(n) => {
        self.ts_module_block(n);
      }
      TsModuleDecl(n) => {
        self.ts_module_decl(n);
      }
      TsNamespaceDecl(n) => {
        self.ts_namespace_decl(n);
      }
      TsNamespaceExportDecl(n) => {
        self.ts_namespace_export_decl(n);
      }
      TsNonNullExpr(n) => {
        self.ts_non_null_expr(n);
      }
      TsOptionalType(n) => {
        self.ts_optional_type(n);
      }
      TsParamProp(n) => {
        self.ts_param_prop(n);
      }
      TsParenthesizedType(n) => {
        self.ts_parenthesized_type(n);
      }
      TsPropertySignature(n) => {
        self.ts_property_signature(n);
      }
      TsQualifiedName(n) => {
        self.ts_qualified_name(n);
      }
      TsRestType(n) => {
        self.ts_rest_type(n);
      }
      TsThisType(n) => {
        self.ts_this_type(n);
      }
      TsTplLitType(n) => {
        self.ts_tpl_lit_type(n);
      }
      TsTupleElement(n) => {
        self.ts_tuple_element(n);
      }
      TsTupleType(n) => {
        self.ts_tuple_type(n);
      }
      TsTypeAliasDecl(n) => {
        self.ts_type_alias_decl(n);
      }
      TsTypeAnn(n) => {
        self.ts_type_ann(n);
      }
      TsTypeAssertion(n) => {
        self.ts_type_assertion(n);
      }
      TsTypeCastExpr(n) => {
        self.ts_type_cast_expr(n);
      }
      TsTypeLit(n) => {
        self.ts_type_lit(n);
      }
      TsTypeOperator(n) => {
        self.ts_type_operator(n);
      }
      TsTypeParam(n) => {
        self.ts_type_param(n);
      }
      TsTypeParamDecl(n) => {
        self.ts_type_param_decl(n);
      }
      TsTypeParamInstantiation(n) => {
        self.ts_type_param_instantiation(n);
      }
      TsTypePredicate(n) => {
        self.ts_type_predicate(n);
      }
      TsTypeQuery(n) => {
        self.ts_type_query(n);
      }
      TsTypeRef(n) => {
        self.ts_type_ref(n);
      }
      TsUnionType(n) => {
        self.ts_union_type(n);
      }
      UnaryExpr(n) => {
        self.unary_expr(n);
      }
      UpdateExpr(n) => {
        self.update_expr(n);
      }
      VarDecl(n) => {
        self.var_decl(n);
      }
      VarDeclarator(n) => {
        self.var_declarator(n);
      }
      WhileStmt(n) => {
        self.while_stmt(n);
      }
      WithStmt(n) => {
        self.with_stmt(n);
      }
      YieldExpr(n) => {
        self.yield_expr(n);
      }
    }

    for child in node.children() {
      self.traverse(child);
    }
  }
}

impl<H: Handler> Traverse for H {}
