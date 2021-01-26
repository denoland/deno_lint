use dprint_swc_ecma_ast_view::{self as AstView, NodeKind, NodeTrait};

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
    let current_node = node.into_node();

    match node.kind() {
      NodeKind::ArrayLit => {
        let n = current_node.expect::<AstView::ArrayLit<'a>>();
        self.array_lit(n);
      }
      NodeKind::ArrayPat => {
        let n = current_node.expect::<AstView::ArrayPat<'a>>();
        self.array_pat(n);
      }
      NodeKind::ArrowExpr => {
        let n = current_node.expect::<AstView::ArrowExpr<'a>>();
        self.arrow_expr(n);
      }
      NodeKind::AssignExpr => {
        let n = current_node.expect::<AstView::AssignExpr<'a>>();
        self.assign_expr(n);
      }
      NodeKind::AssignPat => {
        let n = current_node.expect::<AstView::AssignPat<'a>>();
        self.assign_pat(n);
      }
      NodeKind::AssignPatProp => {
        let n = current_node.expect::<AstView::AssignPatProp<'a>>();
        self.assign_pat_prop(n);
      }
      NodeKind::AssignProp => {
        let n = current_node.expect::<AstView::AssignProp<'a>>();
        self.assign_prop(n);
      }
      NodeKind::AwaitExpr => {
        let n = current_node.expect::<AstView::AwaitExpr<'a>>();
        self.await_expr(n);
      }
      NodeKind::BigInt => {
        let n = current_node.expect::<AstView::BigInt<'a>>();
        self.big_int(n);
      }
      NodeKind::BinExpr => {
        let n = current_node.expect::<AstView::BinExpr<'a>>();
        self.bin_expr(n);
      }
      NodeKind::BlockStmt => {
        let n = current_node.expect::<AstView::BlockStmt<'a>>();
        self.block_stmt(n);
      }
      NodeKind::Bool => {
        let n = current_node.expect::<AstView::Bool<'a>>();
        self.bool(n);
      }
      NodeKind::BreakStmt => {
        let n = current_node.expect::<AstView::BreakStmt<'a>>();
        self.break_stmt(n);
      }
      NodeKind::CallExpr => {
        let n = current_node.expect::<AstView::CallExpr<'a>>();
        self.call_expr(n);
      }
      NodeKind::CatchClause => {
        let n = current_node.expect::<AstView::CatchClause<'a>>();
        self.catch_clause(n);
      }
      NodeKind::Class => {
        let n = current_node.expect::<AstView::Class<'a>>();
        self.class(n);
      }
      NodeKind::ClassDecl => {
        let n = current_node.expect::<AstView::ClassDecl<'a>>();
        self.class_decl(n);
      }
      NodeKind::ClassExpr => {
        let n = current_node.expect::<AstView::ClassExpr<'a>>();
        self.class_expr(n);
      }
      NodeKind::ClassMethod => {
        let n = current_node.expect::<AstView::ClassMethod<'a>>();
        self.class_method(n);
      }
      NodeKind::ClassProp => {
        let n = current_node.expect::<AstView::ClassProp<'a>>();
        self.class_prop(n);
      }
      NodeKind::ComputedPropName => {
        let n = current_node.expect::<AstView::ComputedPropName<'a>>();
        self.computed_prop_name(n);
      }
      NodeKind::CondExpr => {
        let n = current_node.expect::<AstView::CondExpr<'a>>();
        self.cond_expr(n);
      }
      NodeKind::Constructor => {
        let n = current_node.expect::<AstView::Constructor<'a>>();
        self.constructor(n);
      }
      NodeKind::ContinueStmt => {
        let n = current_node.expect::<AstView::ContinueStmt<'a>>();
        self.continue_stmt(n);
      }
      NodeKind::DebuggerStmt => {
        let n = current_node.expect::<AstView::DebuggerStmt<'a>>();
        self.debugger_stmt(n);
      }
      NodeKind::Decorator => {
        let n = current_node.expect::<AstView::Decorator<'a>>();
        self.decorator(n);
      }
      NodeKind::DoWhileStmt => {
        let n = current_node.expect::<AstView::DoWhileStmt<'a>>();
        self.do_while_stmt(n);
      }
      NodeKind::EmptyStmt => {
        let n = current_node.expect::<AstView::EmptyStmt<'a>>();
        self.empty_stmt(n);
      }
      NodeKind::ExportAll => {
        let n = current_node.expect::<AstView::ExportAll<'a>>();
        self.export_all(n);
      }
      NodeKind::ExportDecl => {
        let n = current_node.expect::<AstView::ExportDecl<'a>>();
        self.export_decl(n);
      }
      NodeKind::ExportDefaultDecl => {
        let n = current_node.expect::<AstView::ExportDefaultDecl<'a>>();
        self.export_default_decl(n);
      }
      NodeKind::ExportDefaultExpr => {
        let n = current_node.expect::<AstView::ExportDefaultExpr<'a>>();
        self.export_default_expr(n);
      }
      NodeKind::ExportDefaultSpecifier => {
        let n = current_node.expect::<AstView::ExportDefaultSpecifier<'a>>();
        self.export_default_specifier(n);
      }
      NodeKind::ExportNamedSpecifier => {
        let n = current_node.expect::<AstView::ExportNamedSpecifier<'a>>();
        self.export_named_specifier(n);
      }
      NodeKind::ExportNamespaceSpecifier => {
        let n = current_node.expect::<AstView::ExportNamespaceSpecifier<'a>>();
        self.export_namespace_specifier(n);
      }
      NodeKind::ExprOrSpread => {
        let n = current_node.expect::<AstView::ExprOrSpread<'a>>();
        self.expr_or_spread(n);
      }
      NodeKind::ExprStmt => {
        let n = current_node.expect::<AstView::ExprStmt<'a>>();
        self.expr_stmt(n);
      }
      NodeKind::FnDecl => {
        let n = current_node.expect::<AstView::FnDecl<'a>>();
        self.fn_decl(n);
      }
      NodeKind::FnExpr => {
        let n = current_node.expect::<AstView::FnExpr<'a>>();
        self.fn_expr(n);
      }
      NodeKind::ForInStmt => {
        let n = current_node.expect::<AstView::ForInStmt<'a>>();
        self.for_in_stmt(n);
      }
      NodeKind::ForOfStmt => {
        let n = current_node.expect::<AstView::ForOfStmt<'a>>();
        self.for_of_stmt(n);
      }
      NodeKind::ForStmt => {
        let n = current_node.expect::<AstView::ForStmt<'a>>();
        self.for_stmt(n);
      }
      NodeKind::Function => {
        let n = current_node.expect::<AstView::Function<'a>>();
        self.function(n);
      }
      NodeKind::GetterProp => {
        let n = current_node.expect::<AstView::GetterProp<'a>>();
        self.getter_prop(n);
      }
      NodeKind::Ident => {
        let n = current_node.expect::<AstView::Ident<'a>>();
        self.ident(n);
      }
      NodeKind::IfStmt => {
        let n = current_node.expect::<AstView::IfStmt<'a>>();
        self.if_stmt(n);
      }
      NodeKind::ImportDecl => {
        let n = current_node.expect::<AstView::ImportDecl<'a>>();
        self.import_decl(n);
      }
      NodeKind::ImportDefaultSpecifier => {
        let n = current_node.expect::<AstView::ImportDefaultSpecifier<'a>>();
        self.import_default_specifier(n);
      }
      NodeKind::ImportNamedSpecifier => {
        let n = current_node.expect::<AstView::ImportNamedSpecifier<'a>>();
        self.import_named_specifier(n);
      }
      NodeKind::ImportStarAsSpecifier => {
        let n = current_node.expect::<AstView::ImportStarAsSpecifier<'a>>();
        self.import_star_as_specifier(n);
      }
      NodeKind::Invalid => {
        let n = current_node.expect::<AstView::Invalid<'a>>();
        self.invalid(n);
      }
      NodeKind::JSXAttr => {
        let n = current_node.expect::<AstView::JSXAttr<'a>>();
        self.jsx_attr(n);
      }
      NodeKind::JSXClosingElement => {
        let n = current_node.expect::<AstView::JSXClosingElement<'a>>();
        self.jsx_closing_element(n);
      }
      NodeKind::JSXClosingFragment => {
        let n = current_node.expect::<AstView::JSXClosingFragment<'a>>();
        self.jsx_closing_fragment(n);
      }
      NodeKind::JSXElement => {
        let n = current_node.expect::<AstView::JSXElement<'a>>();
        self.jsx_element(n);
      }
      NodeKind::JSXEmptyExpr => {
        let n = current_node.expect::<AstView::JSXEmptyExpr<'a>>();
        self.jsx_empty_expr(n);
      }
      NodeKind::JSXExprContainer => {
        let n = current_node.expect::<AstView::JSXExprContainer<'a>>();
        self.jsx_expr_container(n);
      }
      NodeKind::JSXFragment => {
        let n = current_node.expect::<AstView::JSXFragment<'a>>();
        self.jsx_fragment(n);
      }
      NodeKind::JSXMemberExpr => {
        let n = current_node.expect::<AstView::JSXMemberExpr<'a>>();
        self.jsx_member_expr(n);
      }
      NodeKind::JSXNamespacedName => {
        let n = current_node.expect::<AstView::JSXNamespacedName<'a>>();
        self.jsx_namespaced_name(n);
      }
      NodeKind::JSXOpeningElement => {
        let n = current_node.expect::<AstView::JSXOpeningElement<'a>>();
        self.jsx_opening_element(n);
      }
      NodeKind::JSXOpeningFragment => {
        let n = current_node.expect::<AstView::JSXOpeningFragment<'a>>();
        self.jsx_opening_fragment(n);
      }
      NodeKind::JSXSpreadChild => {
        let n = current_node.expect::<AstView::JSXSpreadChild<'a>>();
        self.jsx_spread_child(n);
      }
      NodeKind::JSXText => {
        let n = current_node.expect::<AstView::JSXText<'a>>();
        self.jsx_text(n);
      }
      NodeKind::KeyValuePatProp => {
        let n = current_node.expect::<AstView::KeyValuePatProp<'a>>();
        self.key_value_pat_prop(n);
      }
      NodeKind::KeyValueProp => {
        let n = current_node.expect::<AstView::KeyValueProp<'a>>();
        self.key_value_prop(n);
      }
      NodeKind::LabeledStmt => {
        let n = current_node.expect::<AstView::LabeledStmt<'a>>();
        self.labeled_stmt(n);
      }
      NodeKind::MemberExpr => {
        let n = current_node.expect::<AstView::MemberExpr<'a>>();
        self.member_expr(n);
      }
      NodeKind::MetaPropExpr => {
        let n = current_node.expect::<AstView::MetaPropExpr<'a>>();
        self.meta_prop_expr(n);
      }
      NodeKind::MethodProp => {
        let n = current_node.expect::<AstView::MethodProp<'a>>();
        self.method_prop(n);
      }
      NodeKind::Module => {
        let n = current_node.expect::<AstView::Module<'a>>();
        self.module(n);
      }
      NodeKind::NamedExport => {
        let n = current_node.expect::<AstView::NamedExport<'a>>();
        self.named_export(n);
      }
      NodeKind::NewExpr => {
        let n = current_node.expect::<AstView::NewExpr<'a>>();
        self.new_expr(n);
      }
      NodeKind::Null => {
        let n = current_node.expect::<AstView::Null<'a>>();
        self.null(n);
      }
      NodeKind::Number => {
        let n = current_node.expect::<AstView::Number<'a>>();
        self.number(n);
      }
      NodeKind::ObjectLit => {
        let n = current_node.expect::<AstView::ObjectLit<'a>>();
        self.object_lit(n);
      }
      NodeKind::ObjectPat => {
        let n = current_node.expect::<AstView::ObjectPat<'a>>();
        self.object_pat(n);
      }
      NodeKind::OptChainExpr => {
        let n = current_node.expect::<AstView::OptChainExpr<'a>>();
        self.opt_chain_expr(n);
      }
      NodeKind::Param => {
        let n = current_node.expect::<AstView::Param<'a>>();
        self.param(n);
      }
      NodeKind::ParenExpr => {
        let n = current_node.expect::<AstView::ParenExpr<'a>>();
        self.paren_expr(n);
      }
      NodeKind::PrivateMethod => {
        let n = current_node.expect::<AstView::PrivateMethod<'a>>();
        self.private_method(n);
      }
      NodeKind::PrivateName => {
        let n = current_node.expect::<AstView::PrivateName<'a>>();
        self.private_name(n);
      }
      NodeKind::PrivateProp => {
        let n = current_node.expect::<AstView::PrivateProp<'a>>();
        self.private_prop(n);
      }
      NodeKind::Regex => {
        let n = current_node.expect::<AstView::Regex<'a>>();
        self.regex(n);
      }
      NodeKind::RestPat => {
        let n = current_node.expect::<AstView::RestPat<'a>>();
        self.rest_pat(n);
      }
      NodeKind::ReturnStmt => {
        let n = current_node.expect::<AstView::ReturnStmt<'a>>();
        self.return_stmt(n);
      }
      NodeKind::SeqExpr => {
        let n = current_node.expect::<AstView::SeqExpr<'a>>();
        self.seq_expr(n);
      }
      NodeKind::SetterProp => {
        let n = current_node.expect::<AstView::SetterProp<'a>>();
        self.setter_prop(n);
      }
      NodeKind::SpreadElement => {
        let n = current_node.expect::<AstView::SpreadElement<'a>>();
        self.spread_element(n);
      }
      NodeKind::Str => {
        let n = current_node.expect::<AstView::Str<'a>>();
        self.str(n);
      }
      NodeKind::Super => {
        let n = current_node.expect::<AstView::Super<'a>>();
        self.super_(n);
      }
      NodeKind::SwitchCase => {
        let n = current_node.expect::<AstView::SwitchCase<'a>>();
        self.switch_case(n);
      }
      NodeKind::SwitchStmt => {
        let n = current_node.expect::<AstView::SwitchStmt<'a>>();
        self.switch_stmt(n);
      }
      NodeKind::TaggedTpl => {
        let n = current_node.expect::<AstView::TaggedTpl<'a>>();
        self.tagged_tpl(n);
      }
      NodeKind::ThisExpr => {
        let n = current_node.expect::<AstView::ThisExpr<'a>>();
        self.this_expr(n);
      }
      NodeKind::ThrowStmt => {
        let n = current_node.expect::<AstView::ThrowStmt<'a>>();
        self.throw_stmt(n);
      }
      NodeKind::Tpl => {
        let n = current_node.expect::<AstView::Tpl<'a>>();
        self.tpl(n);
      }
      NodeKind::TplElement => {
        let n = current_node.expect::<AstView::TplElement<'a>>();
        self.tpl_element(n);
      }
      NodeKind::TryStmt => {
        let n = current_node.expect::<AstView::TryStmt<'a>>();
        self.try_stmt(n);
      }
      NodeKind::TsArrayType => {
        let n = current_node.expect::<AstView::TsArrayType<'a>>();
        self.ts_array_type(n);
      }
      NodeKind::TsAsExpr => {
        let n = current_node.expect::<AstView::TsAsExpr<'a>>();
        self.ts_as_expr(n);
      }
      NodeKind::TsCallSignatureDecl => {
        let n = current_node.expect::<AstView::TsCallSignatureDecl<'a>>();
        self.ts_call_signature_decl(n);
      }
      NodeKind::TsConditionalType => {
        let n = current_node.expect::<AstView::TsConditionalType<'a>>();
        self.ts_conditional_type(n);
      }
      NodeKind::TsConstAssertion => {
        let n = current_node.expect::<AstView::TsConstAssertion<'a>>();
        self.ts_const_assertion(n);
      }
      NodeKind::TsConstructSignatureDecl => {
        let n = current_node.expect::<AstView::TsConstructSignatureDecl<'a>>();
        self.ts_construct_signature_decl(n);
      }
      NodeKind::TsConstructorType => {
        let n = current_node.expect::<AstView::TsConstructorType<'a>>();
        self.ts_constructor_type(n);
      }
      NodeKind::TsEnumDecl => {
        let n = current_node.expect::<AstView::TsEnumDecl<'a>>();
        self.ts_enum_decl(n);
      }
      NodeKind::TsEnumMember => {
        let n = current_node.expect::<AstView::TsEnumMember<'a>>();
        self.ts_enum_member(n);
      }
      NodeKind::TsExportAssignment => {
        let n = current_node.expect::<AstView::TsExportAssignment<'a>>();
        self.ts_export_assignment(n);
      }
      NodeKind::TsExprWithTypeArgs => {
        let n = current_node.expect::<AstView::TsExprWithTypeArgs<'a>>();
        self.ts_expr_with_type_args(n);
      }
      NodeKind::TsExternalModuleRef => {
        let n = current_node.expect::<AstView::TsExternalModuleRef<'a>>();
        self.ts_external_module_ref(n);
      }
      NodeKind::TsFnType => {
        let n = current_node.expect::<AstView::TsFnType<'a>>();
        self.ts_fn_type(n);
      }
      NodeKind::TsImportEqualsDecl => {
        let n = current_node.expect::<AstView::TsImportEqualsDecl<'a>>();
        self.ts_import_equal_decl(n);
      }
      NodeKind::TsImportType => {
        let n = current_node.expect::<AstView::TsImportType<'a>>();
        self.ts_import_type(n);
      }
      NodeKind::TsIndexSignature => {
        let n = current_node.expect::<AstView::TsIndexSignature<'a>>();
        self.ts_index_signature(n);
      }
      NodeKind::TsIndexedAccessType => {
        let n = current_node.expect::<AstView::TsIndexedAccessType<'a>>();
        self.ts_indexed_access_type(n);
      }
      NodeKind::TsInferType => {
        let n = current_node.expect::<AstView::TsInferType<'a>>();
        self.ts_infer_type(n);
      }
      NodeKind::TsInterfaceBody => {
        let n = current_node.expect::<AstView::TsInterfaceBody<'a>>();
        self.ts_interface_body(n);
      }
      NodeKind::TsInterfaceDecl => {
        let n = current_node.expect::<AstView::TsInterfaceDecl<'a>>();
        self.ts_interface_decl(n);
      }
      NodeKind::TsIntersectionType => {
        let n = current_node.expect::<AstView::TsIntersectionType<'a>>();
        self.ts_intersection_type(n);
      }
      NodeKind::TsKeywordType => {
        let n = current_node.expect::<AstView::TsKeywordType<'a>>();
        self.ts_keyword_type(n);
      }
      NodeKind::TsLitType => {
        let n = current_node.expect::<AstView::TsLitType<'a>>();
        self.ts_lit_type(n);
      }
      NodeKind::TsMappedType => {
        let n = current_node.expect::<AstView::TsMappedType<'a>>();
        self.ts_mapped_type(n);
      }
      NodeKind::TsMethodSignature => {
        let n = current_node.expect::<AstView::TsMethodSignature<'a>>();
        self.ts_method_signature(n);
      }
      NodeKind::TsModuleBlock => {
        let n = current_node.expect::<AstView::TsModuleBlock<'a>>();
        self.ts_module_block(n);
      }
      NodeKind::TsModuleDecl => {
        let n = current_node.expect::<AstView::TsModuleDecl<'a>>();
        self.ts_module_decl(n);
      }
      NodeKind::TsNamespaceDecl => {
        let n = current_node.expect::<AstView::TsNamespaceDecl<'a>>();
        self.ts_namespace_decl(n);
      }
      NodeKind::TsNamespaceExportDecl => {
        let n = current_node.expect::<AstView::TsNamespaceExportDecl<'a>>();
        self.ts_namespace_export_decl(n);
      }
      NodeKind::TsNonNullExpr => {
        let n = current_node.expect::<AstView::TsNonNullExpr<'a>>();
        self.ts_non_null_expr(n);
      }
      NodeKind::TsOptionalType => {
        let n = current_node.expect::<AstView::TsOptionalType<'a>>();
        self.ts_optional_type(n);
      }
      NodeKind::TsParamProp => {
        let n = current_node.expect::<AstView::TsParamProp<'a>>();
        self.ts_param_prop(n);
      }
      NodeKind::TsParenthesizedType => {
        let n = current_node.expect::<AstView::TsParenthesizedType<'a>>();
        self.ts_parenthesized_type(n);
      }
      NodeKind::TsPropertySignature => {
        let n = current_node.expect::<AstView::TsPropertySignature<'a>>();
        self.ts_property_signature(n);
      }
      NodeKind::TsQualifiedName => {
        let n = current_node.expect::<AstView::TsQualifiedName<'a>>();
        self.ts_qualified_name(n);
      }
      NodeKind::TsRestType => {
        let n = current_node.expect::<AstView::TsRestType<'a>>();
        self.ts_rest_type(n);
      }
      NodeKind::TsThisType => {
        let n = current_node.expect::<AstView::TsThisType<'a>>();
        self.ts_this_type(n);
      }
      NodeKind::TsTplLitType => {
        let n = current_node.expect::<AstView::TsTplLitType<'a>>();
        self.ts_tpl_lit_type(n);
      }
      NodeKind::TsTupleElement => {
        let n = current_node.expect::<AstView::TsTupleElement<'a>>();
        self.ts_tuple_element(n);
      }
      NodeKind::TsTupleType => {
        let n = current_node.expect::<AstView::TsTupleType<'a>>();
        self.ts_tuple_type(n);
      }
      NodeKind::TsTypeAliasDecl => {
        let n = current_node.expect::<AstView::TsTypeAliasDecl<'a>>();
        self.ts_type_alias_decl(n);
      }
      NodeKind::TsTypeAnn => {
        let n = current_node.expect::<AstView::TsTypeAnn<'a>>();
        self.ts_type_ann(n);
      }
      NodeKind::TsTypeAssertion => {
        let n = current_node.expect::<AstView::TsTypeAssertion<'a>>();
        self.ts_type_assertion(n);
      }
      NodeKind::TsTypeCastExpr => {
        let n = current_node.expect::<AstView::TsTypeCastExpr<'a>>();
        self.ts_type_cast_expr(n);
      }
      NodeKind::TsTypeLit => {
        let n = current_node.expect::<AstView::TsTypeLit<'a>>();
        self.ts_type_lit(n);
      }
      NodeKind::TsTypeOperator => {
        let n = current_node.expect::<AstView::TsTypeOperator<'a>>();
        self.ts_type_operator(n);
      }
      NodeKind::TsTypeParam => {
        let n = current_node.expect::<AstView::TsTypeParam<'a>>();
        self.ts_type_param(n);
      }
      NodeKind::TsTypeParamDecl => {
        let n = current_node.expect::<AstView::TsTypeParamDecl<'a>>();
        self.ts_type_param_decl(n);
      }
      NodeKind::TsTypeParamInstantiation => {
        let n = current_node.expect::<AstView::TsTypeParamInstantiation<'a>>();
        self.ts_type_param_instantiation(n);
      }
      NodeKind::TsTypePredicate => {
        let n = current_node.expect::<AstView::TsTypePredicate<'a>>();
        self.ts_type_predicate(n);
      }
      NodeKind::TsTypeQuery => {
        let n = current_node.expect::<AstView::TsTypeQuery<'a>>();
        self.ts_type_query(n);
      }
      NodeKind::TsTypeRef => {
        let n = current_node.expect::<AstView::TsTypeRef<'a>>();
        self.ts_type_ref(n);
      }
      NodeKind::TsUnionType => {
        let n = current_node.expect::<AstView::TsUnionType<'a>>();
        self.ts_union_type(n);
      }
      NodeKind::UnaryExpr => {
        let n = current_node.expect::<AstView::UnaryExpr<'a>>();
        self.unary_expr(n);
      }
      NodeKind::UpdateExpr => {
        let n = current_node.expect::<AstView::UpdateExpr<'a>>();
        self.update_expr(n);
      }
      NodeKind::VarDecl => {
        let n = current_node.expect::<AstView::VarDecl<'a>>();
        self.var_decl(n);
      }
      NodeKind::VarDeclarator => {
        let n = current_node.expect::<AstView::VarDeclarator<'a>>();
        self.var_declarator(n);
      }
      NodeKind::WhileStmt => {
        let n = current_node.expect::<AstView::WhileStmt<'a>>();
        self.while_stmt(n);
      }
      NodeKind::WithStmt => {
        let n = current_node.expect::<AstView::WithStmt<'a>>();
        self.with_stmt(n);
      }
      NodeKind::YieldExpr => {
        let n = current_node.expect::<AstView::YieldExpr<'a>>();
        self.yield_expr(n);
      }
    }

    for child in node.children() {
      self.traverse(child);
    }
  }
}

impl<H: Handler> Traverse for H {}
