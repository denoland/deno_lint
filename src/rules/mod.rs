// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::linter::Context;

mod ban_ts_comment;
mod ban_ts_ignore;
mod ban_untagged_ignore;
mod ban_untagged_todo;
mod constructor_super;
mod default_param_last;
mod eqeqeq;
mod explicit_function_return_type;
mod for_direction;
mod getter_return;
mod no_array_constructor;
mod no_async_promise_executor;
mod no_await_in_loop;
mod no_case_declarations;
mod no_class_assign;
mod no_compare_neg_zero;
mod no_cond_assign;
mod no_const_assign;
mod no_debugger;
mod no_delete_var;
mod no_dupe_args;
mod no_dupe_class_members;
mod no_dupe_keys;
mod no_duplicate_case;
mod no_empty;
mod no_empty_character_class;
mod no_empty_interface;
mod no_empty_pattern;
mod no_eval;
mod no_ex_assign;
mod no_explicit_any;
mod no_extra_boolean_cast;
mod no_func_assign;
mod no_inferrable_types;
mod no_misused_new;
mod no_namespace;
mod no_new_symbol;
mod no_non_null_assertion;
mod no_obj_call;
mod no_octal;
mod no_prototype_builtins;
mod no_regex_spaces;
mod no_setter_return;
mod no_sparse_array;
mod no_this_alias;
mod no_this_before_super;
mod no_throw_literal;
mod no_unsafe_finally;
mod no_unsafe_negation;
mod no_var;
mod no_with;
mod prefer_as_const;
mod prefer_namespace_keyword;
mod require_yield;
mod single_var_declarator;
mod triple_slash_reference;
mod use_isnan;
mod valid_typeof;

pub trait LintRule {
  fn new() -> Box<Self>
  where
    Self: Sized;
  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module);
  fn code(&self) -> &'static str;
}

pub fn get_recommended_rules() -> Vec<Box<dyn LintRule>> {
  vec![
    ban_ts_comment::BanTsComment::new(),
    ban_untagged_ignore::BanUntaggedIgnore::new(),
    constructor_super::ConstructorSuper::new(),
    for_direction::ForDirection::new(),
    getter_return::GetterReturn::new(),
    no_array_constructor::NoArrayConstructor::new(),
    no_async_promise_executor::NoAsyncPromiseExecutor::new(),
    no_case_declarations::NoCaseDeclarations::new(),
    no_class_assign::NoClassAssign::new(),
    no_compare_neg_zero::NoCompareNegZero::new(),
    no_cond_assign::NoCondAssign::new(),
    no_debugger::NoDebugger::new(),
    no_delete_var::NoDeleteVar::new(),
    no_dupe_args::NoDupeArgs::new(),
    no_dupe_class_members::NoDupeClassMembers::new(),
    no_dupe_keys::NoDupeKeys::new(),
    no_duplicate_case::NoDuplicateCase::new(),
    no_empty_character_class::NoEmptyCharacterClass::new(),
    no_empty_interface::NoEmptyInterface::new(),
    no_empty_pattern::NoEmptyPattern::new(),
    no_empty::NoEmpty::new(),
    no_ex_assign::NoExAssign::new(),
    no_explicit_any::NoExplicitAny::new(),
    no_extra_boolean_cast::NoExtraBooleanCast::new(),
    no_func_assign::NoFuncAssign::new(),
    no_misused_new::NoMisusedNew::new(),
    no_namespace::NoNamespace::new(),
    no_new_symbol::NoNewSymbol::new(),
    no_non_null_assertion::NoNonNullAssertion::new(),
    no_obj_call::NoObjCall::new(),
    no_octal::NoOctal::new(),
    no_prototype_builtins::NoPrototypeBuiltins::new(),
    no_regex_spaces::NoRegexSpaces::new(),
    no_setter_return::NoSetterReturn::new(),
    no_this_alias::NoThisAlias::new(),
    no_this_before_super::NoThisBeforeSuper::new(),
    no_unsafe_finally::NoUnsafeFinally::new(),
    no_unsafe_negation::NoUnsafeNegation::new(),
    no_with::NoWith::new(),
    prefer_as_const::PreferAsConst::new(),
    prefer_namespace_keyword::PreferNamespaceKeyword::new(),
    require_yield::RequireYield::new(),
    triple_slash_reference::TripleSlashReference::new(),
    use_isnan::UseIsNaN::new(),
    valid_typeof::ValidTypeof::new(),
    no_inferrable_types::NoInferrableTypes::new(),
    no_const_assign::NoConstAssign::new(),
  ]
}

pub fn get_all_rules() -> Vec<Box<dyn LintRule>> {
  vec![
    ban_ts_comment::BanTsComment::new(),
    ban_ts_ignore::BanTsIgnore::new(),
    ban_untagged_ignore::BanUntaggedIgnore::new(),
    ban_untagged_todo::BanUntaggedTodo::new(),
    constructor_super::ConstructorSuper::new(),
    default_param_last::DefaultParamLast::new(),
    eqeqeq::Eqeqeq::new(),
    explicit_function_return_type::ExplicitFunctionReturnType::new(),
    for_direction::ForDirection::new(),
    getter_return::GetterReturn::new(),
    no_array_constructor::NoArrayConstructor::new(),
    no_async_promise_executor::NoAsyncPromiseExecutor::new(),
    no_await_in_loop::NoAwaitInLoop::new(),
    no_case_declarations::NoCaseDeclarations::new(),
    no_class_assign::NoClassAssign::new(),
    no_compare_neg_zero::NoCompareNegZero::new(),
    no_cond_assign::NoCondAssign::new(),
    no_debugger::NoDebugger::new(),
    no_delete_var::NoDeleteVar::new(),
    no_dupe_args::NoDupeArgs::new(),
    no_dupe_class_members::NoDupeClassMembers::new(),
    no_dupe_keys::NoDupeKeys::new(),
    no_duplicate_case::NoDuplicateCase::new(),
    no_empty_character_class::NoEmptyCharacterClass::new(),
    no_empty_interface::NoEmptyInterface::new(),
    no_empty_pattern::NoEmptyPattern::new(),
    no_empty::NoEmpty::new(),
    no_eval::NoEval::new(),
    no_ex_assign::NoExAssign::new(),
    no_explicit_any::NoExplicitAny::new(),
    no_extra_boolean_cast::NoExtraBooleanCast::new(),
    no_func_assign::NoFuncAssign::new(),
    no_misused_new::NoMisusedNew::new(),
    no_namespace::NoNamespace::new(),
    no_new_symbol::NoNewSymbol::new(),
    no_non_null_assertion::NoNonNullAssertion::new(),
    no_obj_call::NoObjCall::new(),
    no_octal::NoOctal::new(),
    no_prototype_builtins::NoPrototypeBuiltins::new(),
    no_regex_spaces::NoRegexSpaces::new(),
    no_setter_return::NoSetterReturn::new(),
    no_sparse_array::NoSparseArray::new(),
    no_this_alias::NoThisAlias::new(),
    no_this_before_super::NoThisBeforeSuper::new(),
    no_throw_literal::NoThrowLiteral::new(),
    no_unsafe_finally::NoUnsafeFinally::new(),
    no_unsafe_negation::NoUnsafeNegation::new(),
    no_var::NoVar::new(),
    no_with::NoWith::new(),
    prefer_as_const::PreferAsConst::new(),
    prefer_namespace_keyword::PreferNamespaceKeyword::new(),
    require_yield::RequireYield::new(),
    single_var_declarator::SingleVarDeclarator::new(),
    triple_slash_reference::TripleSlashReference::new(),
    use_isnan::UseIsNaN::new(),
    valid_typeof::ValidTypeof::new(),
    no_inferrable_types::NoInferrableTypes::new(),
    no_const_assign::NoConstAssign::new(),
  ]
}
