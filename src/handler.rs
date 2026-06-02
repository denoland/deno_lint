// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::context::Context;
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::ast_visit::{walk, Visit};
use deno_ast::oxc::syntax::scope::ScopeFlags;

// NOTE: Handler methods use OXC AST types directly.
// The old `ast_view` wrapper types no longer exist.
//
// Key type renames from the SWC era:
//   ArrayLit          -> ArrayExpression
//   ArrowExpr         -> ArrowFunctionExpression
//   AssignExpr        -> AssignmentExpression
//   AssignPat         -> AssignmentPattern
//   AwaitExpr         -> AwaitExpression
//   BigInt            -> BigIntLiteral
//   BinExpr           -> BinaryExpression
//   BindingIdent      -> BindingIdentifier
//   BlockStmt         -> BlockStatement
//   Bool              -> BooleanLiteral
//   BreakStmt         -> BreakStatement
//   CallExpr          -> CallExpression
//   ClassDecl/Expr    -> Class (unified)
//   ClassMethod       -> MethodDefinition
//   ClassProp         -> PropertyDefinition
//   CondExpr          -> ConditionalExpression
//   ContinueStmt      -> ContinueStatement
//   DebuggerStmt      -> DebuggerStatement
//   DoWhileStmt       -> DoWhileStatement
//   EmptyStmt         -> EmptyStatement
//   ExportAll         -> ExportAllDeclaration
//   ExportDecl        -> ExportNamedDeclaration
//   ExportDefaultDecl -> ExportDefaultDeclaration
//   ExprStmt          -> ExpressionStatement
//   FnDecl/FnExpr     -> Function (unified)
//   ForInStmt         -> ForInStatement
//   ForOfStmt         -> ForOfStatement
//   ForStmt           -> ForStatement
//   Ident             -> IdentifierReference
//   IfStmt            -> IfStatement
//   ImportDecl        -> ImportDeclaration
//   LabeledStmt       -> LabeledStatement
//   MemberExpr        -> (Static/Computed)MemberExpression
//   MetaPropExpr      -> MetaProperty
//   NewExpr           -> NewExpression
//   Null              -> NullLiteral
//   Number            -> NumericLiteral
//   ObjectLit         -> ObjectExpression
//   OptChainExpr      -> ChainExpression
//   ParenExpr         -> ParenthesizedExpression
//   PrivateName       -> PrivateIdentifier
//   Regex             -> RegExpLiteral
//   RestPat           -> BindingRestElement
//   ReturnStmt        -> ReturnStatement
//   SeqExpr           -> SequenceExpression
//   Str               -> StringLiteral
//   SwitchStmt        -> SwitchStatement
//   TaggedTpl         -> TaggedTemplateExpression
//   ThisExpr          -> ThisExpression
//   ThrowStmt         -> ThrowStatement
//   Tpl               -> TemplateLiteral
//   TplElement        -> TemplateElement
//   TryStmt           -> TryStatement
//   UnaryExpr         -> UnaryExpression
//   UpdateExpr        -> UpdateExpression
//   VarDecl           -> VariableDeclaration
//   VarDeclarator     -> VariableDeclarator
//   WhileStmt         -> WhileStatement
//   WithStmt          -> WithStatement
//   YieldExpr         -> YieldExpression

#[allow(unused_variables)]
pub trait Handler<'a> {
  fn program(&mut self, n: &Program<'a>, ctx: &mut Context<'a>) {}

  // Expressions
  fn array_expression(
    &mut self,
    n: &ArrayExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn arrow_function_expression(
    &mut self,
    n: &ArrowFunctionExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn assignment_expression(
    &mut self,
    n: &AssignmentExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn await_expression(
    &mut self,
    n: &AwaitExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn binary_expression(
    &mut self,
    n: &BinaryExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn logical_expression(
    &mut self,
    n: &LogicalExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn call_expression(&mut self, n: &CallExpression<'a>, ctx: &mut Context<'a>) {
  }
  fn chain_expression(
    &mut self,
    n: &ChainExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn conditional_expression(
    &mut self,
    n: &ConditionalExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn import_expression(
    &mut self,
    n: &ImportExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn member_expression(
    &mut self,
    n: &MemberExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn static_member_expression(
    &mut self,
    n: &StaticMemberExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn computed_member_expression(
    &mut self,
    n: &ComputedMemberExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn private_field_expression(
    &mut self,
    n: &PrivateFieldExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn meta_property(&mut self, n: &MetaProperty<'a>, ctx: &mut Context<'a>) {}
  fn new_expression(&mut self, n: &NewExpression<'a>, ctx: &mut Context<'a>) {}
  fn object_expression(
    &mut self,
    n: &ObjectExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn object_property(&mut self, n: &ObjectProperty<'a>, ctx: &mut Context<'a>) {
  }
  fn parenthesized_expression(
    &mut self,
    n: &ParenthesizedExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn sequence_expression(
    &mut self,
    n: &SequenceExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn tagged_template_expression(
    &mut self,
    n: &TaggedTemplateExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn template_literal(
    &mut self,
    n: &TemplateLiteral<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn template_element(
    &mut self,
    n: &TemplateElement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn this_expression(&mut self, n: &ThisExpression, ctx: &mut Context<'a>) {}
  fn unary_expression(
    &mut self,
    n: &UnaryExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn update_expression(
    &mut self,
    n: &UpdateExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn yield_expression(
    &mut self,
    n: &YieldExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn private_in_expression(
    &mut self,
    n: &PrivateInExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn spread_element(&mut self, n: &SpreadElement<'a>, ctx: &mut Context<'a>) {}
  fn argument(&mut self, n: &Argument<'a>, ctx: &mut Context<'a>) {}

  // Literals
  fn boolean_literal(&mut self, n: &BooleanLiteral, ctx: &mut Context<'a>) {}
  fn null_literal(&mut self, n: &NullLiteral, ctx: &mut Context<'a>) {}
  fn numeric_literal(&mut self, n: &NumericLiteral<'a>, ctx: &mut Context<'a>) {
  }
  fn big_int_literal(&mut self, n: &BigIntLiteral<'a>, ctx: &mut Context<'a>) {}
  fn string_literal(&mut self, n: &StringLiteral<'a>, ctx: &mut Context<'a>) {}
  fn reg_exp_literal(&mut self, n: &RegExpLiteral<'a>, ctx: &mut Context<'a>) {}

  // Identifiers
  fn identifier_reference(
    &mut self,
    n: &IdentifierReference<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn binding_identifier(
    &mut self,
    n: &BindingIdentifier<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn identifier_name(&mut self, n: &IdentifierName<'a>, ctx: &mut Context<'a>) {
  }
  fn private_identifier(
    &mut self,
    n: &PrivateIdentifier<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn super_(&mut self, n: &Super, ctx: &mut Context<'a>) {}

  // Statements
  fn block_statement(&mut self, n: &BlockStatement<'a>, ctx: &mut Context<'a>) {
  }
  fn break_statement(&mut self, n: &BreakStatement<'a>, ctx: &mut Context<'a>) {
  }
  fn continue_statement(
    &mut self,
    n: &ContinueStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn debugger_statement(
    &mut self,
    n: &DebuggerStatement,
    ctx: &mut Context<'a>,
  ) {
  }
  fn do_while_statement(
    &mut self,
    n: &DoWhileStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn empty_statement(&mut self, n: &EmptyStatement, ctx: &mut Context<'a>) {}
  fn expression_statement(
    &mut self,
    n: &ExpressionStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn for_in_statement(
    &mut self,
    n: &ForInStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn for_of_statement(
    &mut self,
    n: &ForOfStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn for_statement(&mut self, n: &ForStatement<'a>, ctx: &mut Context<'a>) {}
  fn if_statement(&mut self, n: &IfStatement<'a>, ctx: &mut Context<'a>) {}
  fn labeled_statement(
    &mut self,
    n: &LabeledStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn return_statement(
    &mut self,
    n: &ReturnStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn switch_statement(
    &mut self,
    n: &SwitchStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn switch_case(&mut self, n: &SwitchCase<'a>, ctx: &mut Context<'a>) {}
  fn throw_statement(&mut self, n: &ThrowStatement<'a>, ctx: &mut Context<'a>) {
  }
  fn try_statement(&mut self, n: &TryStatement<'a>, ctx: &mut Context<'a>) {}
  fn catch_clause(&mut self, n: &CatchClause<'a>, ctx: &mut Context<'a>) {}
  fn while_statement(&mut self, n: &WhileStatement<'a>, ctx: &mut Context<'a>) {
  }
  fn with_statement(&mut self, n: &WithStatement<'a>, ctx: &mut Context<'a>) {}

  // Declarations
  fn variable_declaration(
    &mut self,
    n: &VariableDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn variable_declarator(
    &mut self,
    n: &VariableDeclarator<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn function(&mut self, n: &Function<'a>, ctx: &mut Context<'a>) {}

  // Patterns
  fn array_pattern(&mut self, n: &ArrayPattern<'a>, ctx: &mut Context<'a>) {}
  fn object_pattern(&mut self, n: &ObjectPattern<'a>, ctx: &mut Context<'a>) {}
  fn assignment_pattern(
    &mut self,
    n: &AssignmentPattern<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn binding_rest_element(
    &mut self,
    n: &BindingRestElement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn binding_property(
    &mut self,
    n: &BindingProperty<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn formal_parameters(
    &mut self,
    n: &FormalParameters<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn formal_parameter(
    &mut self,
    n: &FormalParameter<'a>,
    ctx: &mut Context<'a>,
  ) {
  }

  // Class
  fn class(&mut self, n: &Class<'a>, ctx: &mut Context<'a>) {}
  fn class_body(&mut self, n: &ClassBody<'a>, ctx: &mut Context<'a>) {}
  fn method_definition(
    &mut self,
    n: &MethodDefinition<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn property_definition(
    &mut self,
    n: &PropertyDefinition<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn static_block(&mut self, n: &StaticBlock<'a>, ctx: &mut Context<'a>) {}
  fn decorator(&mut self, n: &Decorator<'a>, ctx: &mut Context<'a>) {}

  // Modules
  fn import_declaration(
    &mut self,
    n: &ImportDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn import_specifier(
    &mut self,
    n: &ImportSpecifier<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn import_default_specifier(
    &mut self,
    n: &ImportDefaultSpecifier<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn import_namespace_specifier(
    &mut self,
    n: &ImportNamespaceSpecifier<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn export_named_declaration(
    &mut self,
    n: &ExportNamedDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn export_default_declaration(
    &mut self,
    n: &ExportDefaultDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn export_all_declaration(
    &mut self,
    n: &ExportAllDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn export_specifier(
    &mut self,
    n: &ExportSpecifier<'a>,
    ctx: &mut Context<'a>,
  ) {
  }

  // JSX
  fn jsx_element(&mut self, n: &JSXElement<'a>, ctx: &mut Context<'a>) {}
  fn jsx_opening_element(
    &mut self,
    n: &JSXOpeningElement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_closing_element(
    &mut self,
    n: &JSXClosingElement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_fragment(&mut self, n: &JSXFragment<'a>, ctx: &mut Context<'a>) {}
  fn jsx_opening_fragment(
    &mut self,
    n: &JSXOpeningFragment,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_closing_fragment(
    &mut self,
    n: &JSXClosingFragment,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_attribute(&mut self, n: &JSXAttribute<'a>, ctx: &mut Context<'a>) {}
  fn jsx_spread_attribute(
    &mut self,
    n: &JSXSpreadAttribute<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_expression_container(
    &mut self,
    n: &JSXExpressionContainer<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_empty_expression(
    &mut self,
    n: &JSXEmptyExpression,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_member_expression(
    &mut self,
    n: &JSXMemberExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_namespaced_name(
    &mut self,
    n: &JSXNamespacedName<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_spread_child(
    &mut self,
    n: &JSXSpreadChild<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn jsx_text(&mut self, n: &JSXText<'a>, ctx: &mut Context<'a>) {}

  // TypeScript
  fn ts_enum_declaration(
    &mut self,
    n: &TSEnumDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_enum_member(&mut self, n: &TSEnumMember<'a>, ctx: &mut Context<'a>) {}
  fn ts_module_declaration(
    &mut self,
    n: &TSModuleDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_global_declaration(
    &mut self,
    n: &TSGlobalDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_module_block(&mut self, n: &TSModuleBlock<'a>, ctx: &mut Context<'a>) {}
  fn ts_type_alias_declaration(
    &mut self,
    n: &TSTypeAliasDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_interface_declaration(
    &mut self,
    n: &TSInterfaceDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_interface_body(
    &mut self,
    n: &TSInterfaceBody<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_property_signature(
    &mut self,
    n: &TSPropertySignature<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_method_signature(
    &mut self,
    n: &TSMethodSignature<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_index_signature(
    &mut self,
    n: &TSIndexSignature<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_call_signature_declaration(
    &mut self,
    n: &TSCallSignatureDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_construct_signature_declaration(
    &mut self,
    n: &TSConstructSignatureDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_annotation(
    &mut self,
    n: &TSTypeAnnotation<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_parameter(
    &mut self,
    n: &TSTypeParameter<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_parameter_declaration(
    &mut self,
    n: &TSTypeParameterDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_parameter_instantiation(
    &mut self,
    n: &TSTypeParameterInstantiation<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_assertion(
    &mut self,
    n: &TSTypeAssertion<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_as_expression(
    &mut self,
    n: &TSAsExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_satisfies_expression(
    &mut self,
    n: &TSSatisfiesExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_non_null_expression(
    &mut self,
    n: &TSNonNullExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_instantiation_expression(
    &mut self,
    n: &TSInstantiationExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_import_equals_declaration(
    &mut self,
    n: &TSImportEqualsDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_export_assignment(
    &mut self,
    n: &TSExportAssignment<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_namespace_export_declaration(
    &mut self,
    n: &TSNamespaceExportDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }

  // TS Types
  fn ts_any_keyword(&mut self, n: &TSAnyKeyword, ctx: &mut Context<'a>) {}
  fn ts_string_keyword(&mut self, n: &TSStringKeyword, ctx: &mut Context<'a>) {}
  fn ts_boolean_keyword(
    &mut self,
    n: &TSBooleanKeyword,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_number_keyword(&mut self, n: &TSNumberKeyword, ctx: &mut Context<'a>) {}
  fn ts_never_keyword(&mut self, n: &TSNeverKeyword, ctx: &mut Context<'a>) {}
  fn ts_unknown_keyword(
    &mut self,
    n: &TSUnknownKeyword,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_void_keyword(&mut self, n: &TSVoidKeyword, ctx: &mut Context<'a>) {}
  fn ts_null_keyword(&mut self, n: &TSNullKeyword, ctx: &mut Context<'a>) {}
  fn ts_undefined_keyword(
    &mut self,
    n: &TSUndefinedKeyword,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_symbol_keyword(&mut self, n: &TSSymbolKeyword, ctx: &mut Context<'a>) {}
  fn ts_big_int_keyword(&mut self, n: &TSBigIntKeyword, ctx: &mut Context<'a>) {
  }
  fn ts_object_keyword(&mut self, n: &TSObjectKeyword, ctx: &mut Context<'a>) {}
  fn ts_this_type(&mut self, n: &TSThisType, ctx: &mut Context<'a>) {}
  fn ts_type_reference(
    &mut self,
    n: &TSTypeReference<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_array_type(&mut self, n: &TSArrayType<'a>, ctx: &mut Context<'a>) {}
  fn ts_tuple_type(&mut self, n: &TSTupleType<'a>, ctx: &mut Context<'a>) {}
  fn ts_union_type(&mut self, n: &TSUnionType<'a>, ctx: &mut Context<'a>) {}
  fn ts_intersection_type(
    &mut self,
    n: &TSIntersectionType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_conditional_type(
    &mut self,
    n: &TSConditionalType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_function_type(
    &mut self,
    n: &TSFunctionType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_constructor_type(
    &mut self,
    n: &TSConstructorType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_mapped_type(&mut self, n: &TSMappedType<'a>, ctx: &mut Context<'a>) {}
  fn ts_literal_type(&mut self, n: &TSLiteralType<'a>, ctx: &mut Context<'a>) {}
  fn ts_indexed_access_type(
    &mut self,
    n: &TSIndexedAccessType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_operator(
    &mut self,
    n: &TSTypeOperator<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_predicate(
    &mut self,
    n: &TSTypePredicate<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_type_query(&mut self, n: &TSTypeQuery<'a>, ctx: &mut Context<'a>) {}
  fn ts_import_type(&mut self, n: &TSImportType<'a>, ctx: &mut Context<'a>) {}
  fn ts_type_literal(&mut self, n: &TSTypeLiteral<'a>, ctx: &mut Context<'a>) {}
  fn ts_infer_type(&mut self, n: &TSInferType<'a>, ctx: &mut Context<'a>) {}
  fn ts_optional_type(
    &mut self,
    n: &TSOptionalType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_rest_type(&mut self, n: &TSRestType<'a>, ctx: &mut Context<'a>) {}
  fn ts_parenthesized_type(
    &mut self,
    n: &TSParenthesizedType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_template_literal_type(
    &mut self,
    n: &TSTemplateLiteralType<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_qualified_name(
    &mut self,
    n: &TSQualifiedName<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_class_implements(
    &mut self,
    n: &TSClassImplements<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn ts_interface_heritage(
    &mut self,
    n: &TSInterfaceHeritage<'a>,
    ctx: &mut Context<'a>,
  ) {
  }

  // Exit hooks (for rules that need to act after children are traversed)
  fn function_exit(&mut self, n: &Function<'a>, ctx: &mut Context<'a>) {}
  fn arrow_function_expression_exit(
    &mut self,
    n: &ArrowFunctionExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn method_definition_exit(
    &mut self,
    n: &MethodDefinition<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn labeled_statement_exit(
    &mut self,
    n: &LabeledStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn class_exit(&mut self, n: &Class<'a>, ctx: &mut Context<'a>) {}
  fn if_statement_exit(&mut self, n: &IfStatement<'a>, ctx: &mut Context<'a>) {}
  fn for_statement_exit(
    &mut self,
    n: &ForStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn for_in_statement_exit(
    &mut self,
    n: &ForInStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn for_of_statement_exit(
    &mut self,
    n: &ForOfStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn while_statement_exit(
    &mut self,
    n: &WhileStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn try_statement_exit(
    &mut self,
    n: &TryStatement<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn conditional_expression_exit(
    &mut self,
    n: &ConditionalExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn binary_expression_exit(
    &mut self,
    n: &BinaryExpression<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn variable_declarator_exit(
    &mut self,
    n: &VariableDeclarator<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
  fn export_default_declaration_exit(
    &mut self,
    n: &ExportDefaultDeclaration<'a>,
    ctx: &mut Context<'a>,
  ) {
  }
}

/// Traverses the program, dispatching to Handler methods for each node.
/// Uses OXC's Visit trait for walking the AST.
pub fn traverse_program<'a>(
  handler: &mut (impl Handler<'a> + 'a),
  program: &Program<'a>,
  ctx: &mut Context<'a>,
) {
  ctx.assert_traverse_init();
  let mut traverser = Traverser { handler, ctx };
  traverser.visit_program(program);
}

struct Traverser<'a, 'h, H: Handler<'a>> {
  handler: &'h mut H,
  ctx: &'h mut Context<'a>,
}

macro_rules! visit_handler {
  ($self:ident, $method:ident, $walk:path, $node:expr $(, $extra:expr)*) => {
    $self.handler.$method($node, $self.ctx);
    if !$self.ctx.should_stop_traverse() {
      $walk($self, $node $(, $extra)*);
    }
  };
}

macro_rules! visit_handler_with_exit {
  ($self:ident, $method:ident, $exit_method:ident, $walk:path, $node:expr $(, $extra:expr)*) => {
    $self.handler.$method($node, $self.ctx);
    if !$self.ctx.should_stop_traverse() {
      $walk($self, $node $(, $extra)*);
    }
    $self.handler.$exit_method($node, $self.ctx);
  };
}

impl<'a, H: Handler<'a>> Visit<'a> for Traverser<'a, '_, H> {
  // Program
  fn visit_program(&mut self, it: &Program<'a>) {
    visit_handler!(self, program, walk::walk_program, it);
  }

  // Expressions
  fn visit_array_expression(&mut self, it: &ArrayExpression<'a>) {
    visit_handler!(self, array_expression, walk::walk_array_expression, it);
  }
  fn visit_arrow_function_expression(
    &mut self,
    it: &ArrowFunctionExpression<'a>,
  ) {
    visit_handler_with_exit!(
      self,
      arrow_function_expression,
      arrow_function_expression_exit,
      walk::walk_arrow_function_expression,
      it
    );
  }
  fn visit_assignment_expression(&mut self, it: &AssignmentExpression<'a>) {
    visit_handler!(
      self,
      assignment_expression,
      walk::walk_assignment_expression,
      it
    );
  }
  fn visit_await_expression(&mut self, it: &AwaitExpression<'a>) {
    visit_handler!(self, await_expression, walk::walk_await_expression, it);
  }
  fn visit_binary_expression(&mut self, it: &BinaryExpression<'a>) {
    visit_handler_with_exit!(
      self,
      binary_expression,
      binary_expression_exit,
      walk::walk_binary_expression,
      it
    );
  }
  fn visit_logical_expression(&mut self, it: &LogicalExpression<'a>) {
    visit_handler!(self, logical_expression, walk::walk_logical_expression, it);
  }
  fn visit_call_expression(&mut self, it: &CallExpression<'a>) {
    visit_handler!(self, call_expression, walk::walk_call_expression, it);
  }
  fn visit_chain_expression(&mut self, it: &ChainExpression<'a>) {
    visit_handler!(self, chain_expression, walk::walk_chain_expression, it);
  }
  fn visit_conditional_expression(&mut self, it: &ConditionalExpression<'a>) {
    visit_handler_with_exit!(
      self,
      conditional_expression,
      conditional_expression_exit,
      walk::walk_conditional_expression,
      it
    );
  }
  fn visit_import_expression(&mut self, it: &ImportExpression<'a>) {
    visit_handler!(self, import_expression, walk::walk_import_expression, it);
  }
  fn visit_member_expression(&mut self, it: &MemberExpression<'a>) {
    visit_handler!(self, member_expression, walk::walk_member_expression, it);
  }
  fn visit_static_member_expression(
    &mut self,
    it: &StaticMemberExpression<'a>,
  ) {
    visit_handler!(
      self,
      static_member_expression,
      walk::walk_static_member_expression,
      it
    );
  }
  fn visit_computed_member_expression(
    &mut self,
    it: &ComputedMemberExpression<'a>,
  ) {
    visit_handler!(
      self,
      computed_member_expression,
      walk::walk_computed_member_expression,
      it
    );
  }
  fn visit_private_field_expression(
    &mut self,
    it: &PrivateFieldExpression<'a>,
  ) {
    visit_handler!(
      self,
      private_field_expression,
      walk::walk_private_field_expression,
      it
    );
  }
  fn visit_meta_property(&mut self, it: &MetaProperty<'a>) {
    visit_handler!(self, meta_property, walk::walk_meta_property, it);
  }
  fn visit_new_expression(&mut self, it: &NewExpression<'a>) {
    visit_handler!(self, new_expression, walk::walk_new_expression, it);
  }
  fn visit_object_expression(&mut self, it: &ObjectExpression<'a>) {
    visit_handler!(self, object_expression, walk::walk_object_expression, it);
  }
  fn visit_object_property(&mut self, it: &ObjectProperty<'a>) {
    visit_handler!(self, object_property, walk::walk_object_property, it);
  }
  fn visit_parenthesized_expression(
    &mut self,
    it: &ParenthesizedExpression<'a>,
  ) {
    visit_handler!(
      self,
      parenthesized_expression,
      walk::walk_parenthesized_expression,
      it
    );
  }
  fn visit_sequence_expression(&mut self, it: &SequenceExpression<'a>) {
    visit_handler!(
      self,
      sequence_expression,
      walk::walk_sequence_expression,
      it
    );
  }
  fn visit_tagged_template_expression(
    &mut self,
    it: &TaggedTemplateExpression<'a>,
  ) {
    visit_handler!(
      self,
      tagged_template_expression,
      walk::walk_tagged_template_expression,
      it
    );
  }
  fn visit_template_literal(&mut self, it: &TemplateLiteral<'a>) {
    visit_handler!(self, template_literal, walk::walk_template_literal, it);
  }
  fn visit_template_element(&mut self, it: &TemplateElement<'a>) {
    visit_handler!(self, template_element, walk::walk_template_element, it);
  }
  fn visit_this_expression(&mut self, it: &ThisExpression) {
    visit_handler!(self, this_expression, walk::walk_this_expression, it);
  }
  fn visit_unary_expression(&mut self, it: &UnaryExpression<'a>) {
    visit_handler!(self, unary_expression, walk::walk_unary_expression, it);
  }
  fn visit_update_expression(&mut self, it: &UpdateExpression<'a>) {
    visit_handler!(self, update_expression, walk::walk_update_expression, it);
  }
  fn visit_yield_expression(&mut self, it: &YieldExpression<'a>) {
    visit_handler!(self, yield_expression, walk::walk_yield_expression, it);
  }
  fn visit_private_in_expression(&mut self, it: &PrivateInExpression<'a>) {
    visit_handler!(
      self,
      private_in_expression,
      walk::walk_private_in_expression,
      it
    );
  }
  fn visit_spread_element(&mut self, it: &SpreadElement<'a>) {
    visit_handler!(self, spread_element, walk::walk_spread_element, it);
  }
  fn visit_argument(&mut self, it: &Argument<'a>) {
    visit_handler!(self, argument, walk::walk_argument, it);
  }

  // Literals
  fn visit_boolean_literal(&mut self, it: &BooleanLiteral) {
    visit_handler!(self, boolean_literal, walk::walk_boolean_literal, it);
  }
  fn visit_null_literal(&mut self, it: &NullLiteral) {
    visit_handler!(self, null_literal, walk::walk_null_literal, it);
  }
  fn visit_numeric_literal(&mut self, it: &NumericLiteral<'a>) {
    visit_handler!(self, numeric_literal, walk::walk_numeric_literal, it);
  }
  fn visit_big_int_literal(&mut self, it: &BigIntLiteral<'a>) {
    visit_handler!(self, big_int_literal, walk::walk_big_int_literal, it);
  }
  fn visit_string_literal(&mut self, it: &StringLiteral<'a>) {
    visit_handler!(self, string_literal, walk::walk_string_literal, it);
  }
  fn visit_reg_exp_literal(&mut self, it: &RegExpLiteral<'a>) {
    visit_handler!(self, reg_exp_literal, walk::walk_reg_exp_literal, it);
  }

  // Identifiers
  fn visit_identifier_reference(&mut self, it: &IdentifierReference<'a>) {
    visit_handler!(
      self,
      identifier_reference,
      walk::walk_identifier_reference,
      it
    );
  }
  fn visit_binding_identifier(&mut self, it: &BindingIdentifier<'a>) {
    visit_handler!(self, binding_identifier, walk::walk_binding_identifier, it);
  }
  fn visit_identifier_name(&mut self, it: &IdentifierName<'a>) {
    visit_handler!(self, identifier_name, walk::walk_identifier_name, it);
  }
  fn visit_private_identifier(&mut self, it: &PrivateIdentifier<'a>) {
    visit_handler!(self, private_identifier, walk::walk_private_identifier, it);
  }
  fn visit_super(&mut self, it: &Super) {
    visit_handler!(self, super_, walk::walk_super, it);
  }

  // Statements
  fn visit_block_statement(&mut self, it: &BlockStatement<'a>) {
    visit_handler!(self, block_statement, walk::walk_block_statement, it);
  }
  fn visit_break_statement(&mut self, it: &BreakStatement<'a>) {
    visit_handler!(self, break_statement, walk::walk_break_statement, it);
  }
  fn visit_continue_statement(&mut self, it: &ContinueStatement<'a>) {
    visit_handler!(self, continue_statement, walk::walk_continue_statement, it);
  }
  fn visit_debugger_statement(&mut self, it: &DebuggerStatement) {
    visit_handler!(self, debugger_statement, walk::walk_debugger_statement, it);
  }
  fn visit_do_while_statement(&mut self, it: &DoWhileStatement<'a>) {
    visit_handler!(self, do_while_statement, walk::walk_do_while_statement, it);
  }
  fn visit_empty_statement(&mut self, it: &EmptyStatement) {
    visit_handler!(self, empty_statement, walk::walk_empty_statement, it);
  }
  fn visit_expression_statement(&mut self, it: &ExpressionStatement<'a>) {
    visit_handler!(
      self,
      expression_statement,
      walk::walk_expression_statement,
      it
    );
  }
  fn visit_for_in_statement(&mut self, it: &ForInStatement<'a>) {
    visit_handler_with_exit!(
      self,
      for_in_statement,
      for_in_statement_exit,
      walk::walk_for_in_statement,
      it
    );
  }
  fn visit_for_of_statement(&mut self, it: &ForOfStatement<'a>) {
    visit_handler_with_exit!(
      self,
      for_of_statement,
      for_of_statement_exit,
      walk::walk_for_of_statement,
      it
    );
  }
  fn visit_for_statement(&mut self, it: &ForStatement<'a>) {
    visit_handler_with_exit!(
      self,
      for_statement,
      for_statement_exit,
      walk::walk_for_statement,
      it
    );
  }
  fn visit_if_statement(&mut self, it: &IfStatement<'a>) {
    visit_handler_with_exit!(
      self,
      if_statement,
      if_statement_exit,
      walk::walk_if_statement,
      it
    );
  }
  fn visit_labeled_statement(&mut self, it: &LabeledStatement<'a>) {
    visit_handler_with_exit!(
      self,
      labeled_statement,
      labeled_statement_exit,
      walk::walk_labeled_statement,
      it
    );
  }
  fn visit_return_statement(&mut self, it: &ReturnStatement<'a>) {
    visit_handler!(self, return_statement, walk::walk_return_statement, it);
  }
  fn visit_switch_statement(&mut self, it: &SwitchStatement<'a>) {
    visit_handler!(self, switch_statement, walk::walk_switch_statement, it);
  }
  fn visit_switch_case(&mut self, it: &SwitchCase<'a>) {
    visit_handler!(self, switch_case, walk::walk_switch_case, it);
  }
  fn visit_throw_statement(&mut self, it: &ThrowStatement<'a>) {
    visit_handler!(self, throw_statement, walk::walk_throw_statement, it);
  }
  fn visit_try_statement(&mut self, it: &TryStatement<'a>) {
    visit_handler_with_exit!(
      self,
      try_statement,
      try_statement_exit,
      walk::walk_try_statement,
      it
    );
  }
  fn visit_catch_clause(&mut self, it: &CatchClause<'a>) {
    visit_handler!(self, catch_clause, walk::walk_catch_clause, it);
  }
  fn visit_while_statement(&mut self, it: &WhileStatement<'a>) {
    visit_handler_with_exit!(
      self,
      while_statement,
      while_statement_exit,
      walk::walk_while_statement,
      it
    );
  }
  fn visit_with_statement(&mut self, it: &WithStatement<'a>) {
    visit_handler!(self, with_statement, walk::walk_with_statement, it);
  }

  // Declarations
  fn visit_variable_declaration(&mut self, it: &VariableDeclaration<'a>) {
    visit_handler!(
      self,
      variable_declaration,
      walk::walk_variable_declaration,
      it
    );
  }
  fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
    visit_handler_with_exit!(
      self,
      variable_declarator,
      variable_declarator_exit,
      walk::walk_variable_declarator,
      it
    );
  }
  fn visit_function(&mut self, it: &Function<'a>, flags: ScopeFlags) {
    visit_handler_with_exit!(
      self,
      function,
      function_exit,
      walk::walk_function,
      it,
      flags
    );
  }

  // Patterns
  fn visit_array_pattern(&mut self, it: &ArrayPattern<'a>) {
    visit_handler!(self, array_pattern, walk::walk_array_pattern, it);
  }
  fn visit_object_pattern(&mut self, it: &ObjectPattern<'a>) {
    visit_handler!(self, object_pattern, walk::walk_object_pattern, it);
  }
  fn visit_assignment_pattern(&mut self, it: &AssignmentPattern<'a>) {
    visit_handler!(self, assignment_pattern, walk::walk_assignment_pattern, it);
  }
  fn visit_binding_rest_element(&mut self, it: &BindingRestElement<'a>) {
    visit_handler!(
      self,
      binding_rest_element,
      walk::walk_binding_rest_element,
      it
    );
  }
  fn visit_binding_property(&mut self, it: &BindingProperty<'a>) {
    visit_handler!(self, binding_property, walk::walk_binding_property, it);
  }
  fn visit_formal_parameters(&mut self, it: &FormalParameters<'a>) {
    visit_handler!(self, formal_parameters, walk::walk_formal_parameters, it);
  }
  fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
    visit_handler!(self, formal_parameter, walk::walk_formal_parameter, it);
  }

  // Class
  fn visit_class(&mut self, it: &Class<'a>) {
    visit_handler_with_exit!(self, class, class_exit, walk::walk_class, it);
  }
  fn visit_class_body(&mut self, it: &ClassBody<'a>) {
    visit_handler!(self, class_body, walk::walk_class_body, it);
  }
  fn visit_method_definition(&mut self, it: &MethodDefinition<'a>) {
    visit_handler_with_exit!(
      self,
      method_definition,
      method_definition_exit,
      walk::walk_method_definition,
      it
    );
  }
  fn visit_property_definition(&mut self, it: &PropertyDefinition<'a>) {
    visit_handler!(
      self,
      property_definition,
      walk::walk_property_definition,
      it
    );
  }
  fn visit_static_block(&mut self, it: &StaticBlock<'a>) {
    visit_handler!(self, static_block, walk::walk_static_block, it);
  }
  fn visit_decorator(&mut self, it: &Decorator<'a>) {
    visit_handler!(self, decorator, walk::walk_decorator, it);
  }

  // Modules
  fn visit_import_declaration(&mut self, it: &ImportDeclaration<'a>) {
    visit_handler!(self, import_declaration, walk::walk_import_declaration, it);
  }
  fn visit_import_specifier(&mut self, it: &ImportSpecifier<'a>) {
    visit_handler!(self, import_specifier, walk::walk_import_specifier, it);
  }
  fn visit_import_default_specifier(
    &mut self,
    it: &ImportDefaultSpecifier<'a>,
  ) {
    visit_handler!(
      self,
      import_default_specifier,
      walk::walk_import_default_specifier,
      it
    );
  }
  fn visit_import_namespace_specifier(
    &mut self,
    it: &ImportNamespaceSpecifier<'a>,
  ) {
    visit_handler!(
      self,
      import_namespace_specifier,
      walk::walk_import_namespace_specifier,
      it
    );
  }
  fn visit_export_named_declaration(
    &mut self,
    it: &ExportNamedDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      export_named_declaration,
      walk::walk_export_named_declaration,
      it
    );
  }
  fn visit_export_default_declaration(
    &mut self,
    it: &ExportDefaultDeclaration<'a>,
  ) {
    visit_handler_with_exit!(
      self,
      export_default_declaration,
      export_default_declaration_exit,
      walk::walk_export_default_declaration,
      it
    );
  }
  fn visit_export_all_declaration(&mut self, it: &ExportAllDeclaration<'a>) {
    visit_handler!(
      self,
      export_all_declaration,
      walk::walk_export_all_declaration,
      it
    );
  }
  fn visit_export_specifier(&mut self, it: &ExportSpecifier<'a>) {
    visit_handler!(self, export_specifier, walk::walk_export_specifier, it);
  }

  // JSX
  fn visit_jsx_element(&mut self, it: &JSXElement<'a>) {
    visit_handler!(self, jsx_element, walk::walk_jsx_element, it);
  }
  fn visit_jsx_opening_element(&mut self, it: &JSXOpeningElement<'a>) {
    visit_handler!(
      self,
      jsx_opening_element,
      walk::walk_jsx_opening_element,
      it
    );
  }
  fn visit_jsx_closing_element(&mut self, it: &JSXClosingElement<'a>) {
    visit_handler!(
      self,
      jsx_closing_element,
      walk::walk_jsx_closing_element,
      it
    );
  }
  fn visit_jsx_fragment(&mut self, it: &JSXFragment<'a>) {
    visit_handler!(self, jsx_fragment, walk::walk_jsx_fragment, it);
  }
  fn visit_jsx_opening_fragment(&mut self, it: &JSXOpeningFragment) {
    visit_handler!(
      self,
      jsx_opening_fragment,
      walk::walk_jsx_opening_fragment,
      it
    );
  }
  fn visit_jsx_closing_fragment(&mut self, it: &JSXClosingFragment) {
    visit_handler!(
      self,
      jsx_closing_fragment,
      walk::walk_jsx_closing_fragment,
      it
    );
  }
  fn visit_jsx_attribute(&mut self, it: &JSXAttribute<'a>) {
    visit_handler!(self, jsx_attribute, walk::walk_jsx_attribute, it);
  }
  fn visit_jsx_spread_attribute(&mut self, it: &JSXSpreadAttribute<'a>) {
    visit_handler!(
      self,
      jsx_spread_attribute,
      walk::walk_jsx_spread_attribute,
      it
    );
  }
  fn visit_jsx_expression_container(
    &mut self,
    it: &JSXExpressionContainer<'a>,
  ) {
    visit_handler!(
      self,
      jsx_expression_container,
      walk::walk_jsx_expression_container,
      it
    );
  }
  fn visit_jsx_empty_expression(&mut self, it: &JSXEmptyExpression) {
    visit_handler!(
      self,
      jsx_empty_expression,
      walk::walk_jsx_empty_expression,
      it
    );
  }
  fn visit_jsx_member_expression(&mut self, it: &JSXMemberExpression<'a>) {
    visit_handler!(
      self,
      jsx_member_expression,
      walk::walk_jsx_member_expression,
      it
    );
  }
  fn visit_jsx_namespaced_name(&mut self, it: &JSXNamespacedName<'a>) {
    visit_handler!(
      self,
      jsx_namespaced_name,
      walk::walk_jsx_namespaced_name,
      it
    );
  }
  fn visit_jsx_spread_child(&mut self, it: &JSXSpreadChild<'a>) {
    visit_handler!(self, jsx_spread_child, walk::walk_jsx_spread_child, it);
  }
  fn visit_jsx_text(&mut self, it: &JSXText<'a>) {
    visit_handler!(self, jsx_text, walk::walk_jsx_text, it);
  }

  // TypeScript
  fn visit_ts_enum_declaration(&mut self, it: &TSEnumDeclaration<'a>) {
    visit_handler!(
      self,
      ts_enum_declaration,
      walk::walk_ts_enum_declaration,
      it
    );
  }
  fn visit_ts_enum_member(&mut self, it: &TSEnumMember<'a>) {
    visit_handler!(self, ts_enum_member, walk::walk_ts_enum_member, it);
  }
  fn visit_ts_global_declaration(&mut self, it: &TSGlobalDeclaration<'a>) {
    visit_handler!(
      self,
      ts_global_declaration,
      walk::walk_ts_global_declaration,
      it
    );
  }
  fn visit_ts_module_declaration(&mut self, it: &TSModuleDeclaration<'a>) {
    visit_handler!(
      self,
      ts_module_declaration,
      walk::walk_ts_module_declaration,
      it
    );
  }
  fn visit_ts_module_block(&mut self, it: &TSModuleBlock<'a>) {
    visit_handler!(self, ts_module_block, walk::walk_ts_module_block, it);
  }
  fn visit_ts_type_alias_declaration(
    &mut self,
    it: &TSTypeAliasDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      ts_type_alias_declaration,
      walk::walk_ts_type_alias_declaration,
      it
    );
  }
  fn visit_ts_interface_declaration(
    &mut self,
    it: &TSInterfaceDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      ts_interface_declaration,
      walk::walk_ts_interface_declaration,
      it
    );
  }
  fn visit_ts_interface_body(&mut self, it: &TSInterfaceBody<'a>) {
    visit_handler!(self, ts_interface_body, walk::walk_ts_interface_body, it);
  }
  fn visit_ts_property_signature(&mut self, it: &TSPropertySignature<'a>) {
    visit_handler!(
      self,
      ts_property_signature,
      walk::walk_ts_property_signature,
      it
    );
  }
  fn visit_ts_method_signature(&mut self, it: &TSMethodSignature<'a>) {
    visit_handler!(
      self,
      ts_method_signature,
      walk::walk_ts_method_signature,
      it
    );
  }
  fn visit_ts_index_signature(&mut self, it: &TSIndexSignature<'a>) {
    visit_handler!(self, ts_index_signature, walk::walk_ts_index_signature, it);
  }
  fn visit_ts_call_signature_declaration(
    &mut self,
    it: &TSCallSignatureDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      ts_call_signature_declaration,
      walk::walk_ts_call_signature_declaration,
      it
    );
  }
  fn visit_ts_construct_signature_declaration(
    &mut self,
    it: &TSConstructSignatureDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      ts_construct_signature_declaration,
      walk::walk_ts_construct_signature_declaration,
      it
    );
  }
  fn visit_ts_type_annotation(&mut self, it: &TSTypeAnnotation<'a>) {
    visit_handler!(self, ts_type_annotation, walk::walk_ts_type_annotation, it);
  }
  fn visit_ts_type_parameter(&mut self, it: &TSTypeParameter<'a>) {
    visit_handler!(self, ts_type_parameter, walk::walk_ts_type_parameter, it);
  }
  fn visit_ts_type_parameter_declaration(
    &mut self,
    it: &TSTypeParameterDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      ts_type_parameter_declaration,
      walk::walk_ts_type_parameter_declaration,
      it
    );
  }
  fn visit_ts_type_parameter_instantiation(
    &mut self,
    it: &TSTypeParameterInstantiation<'a>,
  ) {
    visit_handler!(
      self,
      ts_type_parameter_instantiation,
      walk::walk_ts_type_parameter_instantiation,
      it
    );
  }
  fn visit_ts_type_assertion(&mut self, it: &TSTypeAssertion<'a>) {
    visit_handler!(self, ts_type_assertion, walk::walk_ts_type_assertion, it);
  }
  fn visit_ts_as_expression(&mut self, it: &TSAsExpression<'a>) {
    visit_handler!(self, ts_as_expression, walk::walk_ts_as_expression, it);
  }
  fn visit_ts_satisfies_expression(&mut self, it: &TSSatisfiesExpression<'a>) {
    visit_handler!(
      self,
      ts_satisfies_expression,
      walk::walk_ts_satisfies_expression,
      it
    );
  }
  fn visit_ts_non_null_expression(&mut self, it: &TSNonNullExpression<'a>) {
    visit_handler!(
      self,
      ts_non_null_expression,
      walk::walk_ts_non_null_expression,
      it
    );
  }
  fn visit_ts_instantiation_expression(
    &mut self,
    it: &TSInstantiationExpression<'a>,
  ) {
    visit_handler!(
      self,
      ts_instantiation_expression,
      walk::walk_ts_instantiation_expression,
      it
    );
  }
  fn visit_ts_import_equals_declaration(
    &mut self,
    it: &TSImportEqualsDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      ts_import_equals_declaration,
      walk::walk_ts_import_equals_declaration,
      it
    );
  }
  fn visit_ts_export_assignment(&mut self, it: &TSExportAssignment<'a>) {
    visit_handler!(
      self,
      ts_export_assignment,
      walk::walk_ts_export_assignment,
      it
    );
  }
  fn visit_ts_namespace_export_declaration(
    &mut self,
    it: &TSNamespaceExportDeclaration<'a>,
  ) {
    visit_handler!(
      self,
      ts_namespace_export_declaration,
      walk::walk_ts_namespace_export_declaration,
      it
    );
  }

  // TS Types
  fn visit_ts_any_keyword(&mut self, it: &TSAnyKeyword) {
    visit_handler!(self, ts_any_keyword, walk::walk_ts_any_keyword, it);
  }
  fn visit_ts_string_keyword(&mut self, it: &TSStringKeyword) {
    visit_handler!(self, ts_string_keyword, walk::walk_ts_string_keyword, it);
  }
  fn visit_ts_boolean_keyword(&mut self, it: &TSBooleanKeyword) {
    visit_handler!(self, ts_boolean_keyword, walk::walk_ts_boolean_keyword, it);
  }
  fn visit_ts_number_keyword(&mut self, it: &TSNumberKeyword) {
    visit_handler!(self, ts_number_keyword, walk::walk_ts_number_keyword, it);
  }
  fn visit_ts_never_keyword(&mut self, it: &TSNeverKeyword) {
    visit_handler!(self, ts_never_keyword, walk::walk_ts_never_keyword, it);
  }
  fn visit_ts_unknown_keyword(&mut self, it: &TSUnknownKeyword) {
    visit_handler!(self, ts_unknown_keyword, walk::walk_ts_unknown_keyword, it);
  }
  fn visit_ts_void_keyword(&mut self, it: &TSVoidKeyword) {
    visit_handler!(self, ts_void_keyword, walk::walk_ts_void_keyword, it);
  }
  fn visit_ts_null_keyword(&mut self, it: &TSNullKeyword) {
    visit_handler!(self, ts_null_keyword, walk::walk_ts_null_keyword, it);
  }
  fn visit_ts_undefined_keyword(&mut self, it: &TSUndefinedKeyword) {
    visit_handler!(
      self,
      ts_undefined_keyword,
      walk::walk_ts_undefined_keyword,
      it
    );
  }
  fn visit_ts_symbol_keyword(&mut self, it: &TSSymbolKeyword) {
    visit_handler!(self, ts_symbol_keyword, walk::walk_ts_symbol_keyword, it);
  }
  fn visit_ts_big_int_keyword(&mut self, it: &TSBigIntKeyword) {
    visit_handler!(self, ts_big_int_keyword, walk::walk_ts_big_int_keyword, it);
  }
  fn visit_ts_object_keyword(&mut self, it: &TSObjectKeyword) {
    visit_handler!(self, ts_object_keyword, walk::walk_ts_object_keyword, it);
  }
  fn visit_ts_this_type(&mut self, it: &TSThisType) {
    visit_handler!(self, ts_this_type, walk::walk_ts_this_type, it);
  }
  fn visit_ts_type_reference(&mut self, it: &TSTypeReference<'a>) {
    visit_handler!(self, ts_type_reference, walk::walk_ts_type_reference, it);
  }
  fn visit_ts_array_type(&mut self, it: &TSArrayType<'a>) {
    visit_handler!(self, ts_array_type, walk::walk_ts_array_type, it);
  }
  fn visit_ts_tuple_type(&mut self, it: &TSTupleType<'a>) {
    visit_handler!(self, ts_tuple_type, walk::walk_ts_tuple_type, it);
  }
  fn visit_ts_union_type(&mut self, it: &TSUnionType<'a>) {
    visit_handler!(self, ts_union_type, walk::walk_ts_union_type, it);
  }
  fn visit_ts_intersection_type(&mut self, it: &TSIntersectionType<'a>) {
    visit_handler!(
      self,
      ts_intersection_type,
      walk::walk_ts_intersection_type,
      it
    );
  }
  fn visit_ts_conditional_type(&mut self, it: &TSConditionalType<'a>) {
    visit_handler!(
      self,
      ts_conditional_type,
      walk::walk_ts_conditional_type,
      it
    );
  }
  fn visit_ts_function_type(&mut self, it: &TSFunctionType<'a>) {
    visit_handler!(self, ts_function_type, walk::walk_ts_function_type, it);
  }
  fn visit_ts_constructor_type(&mut self, it: &TSConstructorType<'a>) {
    visit_handler!(
      self,
      ts_constructor_type,
      walk::walk_ts_constructor_type,
      it
    );
  }
  fn visit_ts_mapped_type(&mut self, it: &TSMappedType<'a>) {
    visit_handler!(self, ts_mapped_type, walk::walk_ts_mapped_type, it);
  }
  fn visit_ts_literal_type(&mut self, it: &TSLiteralType<'a>) {
    visit_handler!(self, ts_literal_type, walk::walk_ts_literal_type, it);
  }
  fn visit_ts_indexed_access_type(&mut self, it: &TSIndexedAccessType<'a>) {
    visit_handler!(
      self,
      ts_indexed_access_type,
      walk::walk_ts_indexed_access_type,
      it
    );
  }
  fn visit_ts_type_operator(&mut self, it: &TSTypeOperator<'a>) {
    visit_handler!(self, ts_type_operator, walk::walk_ts_type_operator, it);
  }
  fn visit_ts_type_predicate(&mut self, it: &TSTypePredicate<'a>) {
    visit_handler!(self, ts_type_predicate, walk::walk_ts_type_predicate, it);
  }
  fn visit_ts_type_query(&mut self, it: &TSTypeQuery<'a>) {
    visit_handler!(self, ts_type_query, walk::walk_ts_type_query, it);
  }
  fn visit_ts_import_type(&mut self, it: &TSImportType<'a>) {
    visit_handler!(self, ts_import_type, walk::walk_ts_import_type, it);
  }
  fn visit_ts_type_literal(&mut self, it: &TSTypeLiteral<'a>) {
    visit_handler!(self, ts_type_literal, walk::walk_ts_type_literal, it);
  }
  fn visit_ts_infer_type(&mut self, it: &TSInferType<'a>) {
    visit_handler!(self, ts_infer_type, walk::walk_ts_infer_type, it);
  }
  fn visit_ts_optional_type(&mut self, it: &TSOptionalType<'a>) {
    visit_handler!(self, ts_optional_type, walk::walk_ts_optional_type, it);
  }
  fn visit_ts_rest_type(&mut self, it: &TSRestType<'a>) {
    visit_handler!(self, ts_rest_type, walk::walk_ts_rest_type, it);
  }
  fn visit_ts_parenthesized_type(&mut self, it: &TSParenthesizedType<'a>) {
    visit_handler!(
      self,
      ts_parenthesized_type,
      walk::walk_ts_parenthesized_type,
      it
    );
  }
  fn visit_ts_template_literal_type(&mut self, it: &TSTemplateLiteralType<'a>) {
    visit_handler!(
      self,
      ts_template_literal_type,
      walk::walk_ts_template_literal_type,
      it
    );
  }
  fn visit_ts_qualified_name(&mut self, it: &TSQualifiedName<'a>) {
    visit_handler!(self, ts_qualified_name, walk::walk_ts_qualified_name, it);
  }
  fn visit_ts_class_implements(&mut self, it: &TSClassImplements<'a>) {
    visit_handler!(
      self,
      ts_class_implements,
      walk::walk_ts_class_implements,
      it
    );
  }
  fn visit_ts_interface_heritage(&mut self, it: &TSInterfaceHeritage<'a>) {
    visit_handler!(
      self,
      ts_interface_heritage,
      walk::walk_ts_interface_heritage,
      it
    );
  }
}
