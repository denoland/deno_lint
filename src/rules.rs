// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::linter::Context;
use swc_ecmascript::ast::Program;

pub mod adjacent_overload_signatures;
pub mod ban_ts_comment;
pub mod ban_types;
pub mod ban_untagged_ignore;
pub mod ban_untagged_todo;
pub mod camelcase;
pub mod constructor_super;
pub mod default_param_last;
pub mod eqeqeq;
pub mod explicit_function_return_type;
pub mod explicit_module_boundary_types;
pub mod for_direction;
pub mod getter_return;
pub mod no_array_constructor;
pub mod no_async_promise_executor;
pub mod no_await_in_loop;
pub mod no_case_declarations;
pub mod no_class_assign;
pub mod no_compare_neg_zero;
pub mod no_cond_assign;
pub mod no_const_assign;
pub mod no_constant_condition;
pub mod no_control_regex;
pub mod no_debugger;
pub mod no_delete_var;
pub mod no_dupe_args;
pub mod no_dupe_class_members;
pub mod no_dupe_else_if;
pub mod no_dupe_keys;
pub mod no_duplicate_case;
pub mod no_empty;
pub mod no_empty_character_class;
pub mod no_empty_interface;
pub mod no_empty_pattern;
pub mod no_eval;
pub mod no_ex_assign;
pub mod no_explicit_any;
pub mod no_extra_boolean_cast;
pub mod no_extra_non_null_assertion;
pub mod no_extra_semi;
pub mod no_fallthrough;
pub mod no_func_assign;
pub mod no_global_assign;
pub mod no_import_assign;
pub mod no_inferrable_types;
pub mod no_inner_declarations;
pub mod no_invalid_regexp;
pub mod no_irregular_whitespace;
pub mod no_misused_new;
pub mod no_mixed_spaces_and_tabs;
pub mod no_namespace;
pub mod no_new_symbol;
pub mod no_non_null_asserted_optional_chain;
pub mod no_non_null_assertion;
pub mod no_obj_calls;
pub mod no_octal;
pub mod no_prototype_builtins;
pub mod no_redeclare;
pub mod no_regex_spaces;
pub mod no_self_assign;
pub mod no_setter_return;
pub mod no_shadow_restricted_names;
pub mod no_sparse_arrays;
pub mod no_this_alias;
pub mod no_this_before_super;
pub mod no_throw_literal;
pub mod no_undef;
pub mod no_unreachable;
pub mod no_unsafe_finally;
pub mod no_unsafe_negation;
pub mod no_unused_labels;
pub mod no_unused_vars;
pub mod no_var;
pub mod no_with;
pub mod prefer_as_const;
pub mod prefer_const;
pub mod prefer_namespace_keyword;
pub mod require_yield;
pub mod single_var_declarator;
pub mod triple_slash_reference;
pub mod use_isnan;
pub mod valid_typeof;

pub trait LintRule {
  fn new() -> Box<Self>
  where
    Self: Sized;
  fn lint_program(&self, context: &mut Context, program: &Program);
  fn code(&self) -> &'static str;
  fn tags(&self) -> &[&'static str] {
    &[]
  }
  fn docs(&self) -> &'static str {
    ""
  }
}

pub fn get_all_rules() -> Vec<Box<dyn LintRule>> {
  vec![
    adjacent_overload_signatures::AdjacentOverloadSignatures::new(),
    ban_ts_comment::BanTsComment::new(),
    ban_types::BanTypes::new(),
    ban_untagged_ignore::BanUntaggedIgnore::new(),
    ban_untagged_todo::BanUntaggedTodo::new(),
    camelcase::Camelcase::new(),
    constructor_super::ConstructorSuper::new(),
    default_param_last::DefaultParamLast::new(),
    eqeqeq::Eqeqeq::new(),
    explicit_function_return_type::ExplicitFunctionReturnType::new(),
    explicit_module_boundary_types::ExplicitModuleBoundaryTypes::new(),
    for_direction::ForDirection::new(),
    getter_return::GetterReturn::new(),
    no_array_constructor::NoArrayConstructor::new(),
    no_async_promise_executor::NoAsyncPromiseExecutor::new(),
    no_await_in_loop::NoAwaitInLoop::new(),
    no_case_declarations::NoCaseDeclarations::new(),
    no_class_assign::NoClassAssign::new(),
    no_compare_neg_zero::NoCompareNegZero::new(),
    no_cond_assign::NoCondAssign::new(),
    no_const_assign::NoConstAssign::new(),
    no_constant_condition::NoConstantCondition::new(),
    no_control_regex::NoControlRegex::new(),
    no_debugger::NoDebugger::new(),
    no_delete_var::NoDeleteVar::new(),
    no_dupe_args::NoDupeArgs::new(),
    no_dupe_class_members::NoDupeClassMembers::new(),
    no_dupe_else_if::NoDupeElseIf::new(),
    no_dupe_keys::NoDupeKeys::new(),
    no_duplicate_case::NoDuplicateCase::new(),
    no_empty::NoEmpty::new(),
    no_empty_character_class::NoEmptyCharacterClass::new(),
    no_empty_interface::NoEmptyInterface::new(),
    no_empty_pattern::NoEmptyPattern::new(),
    no_eval::NoEval::new(),
    no_ex_assign::NoExAssign::new(),
    no_explicit_any::NoExplicitAny::new(),
    no_extra_boolean_cast::NoExtraBooleanCast::new(),
    no_extra_non_null_assertion::NoExtraNonNullAssertion::new(),
    no_extra_semi::NoExtraSemi::new(),
    no_fallthrough::NoFallthrough::new(),
    no_func_assign::NoFuncAssign::new(),
    no_global_assign::NoGlobalAssign::new(),
    no_import_assign::NoImportAssign::new(),
    no_inferrable_types::NoInferrableTypes::new(),
    no_inner_declarations::NoInnerDeclarations::new(),
    no_invalid_regexp::NoInvalidRegexp::new(),
    no_irregular_whitespace::NoIrregularWhitespace::new(),
    no_misused_new::NoMisusedNew::new(),
    no_mixed_spaces_and_tabs::NoMixedSpacesAndTabs::new(),
    no_namespace::NoNamespace::new(),
    no_new_symbol::NoNewSymbol::new(),
    no_non_null_asserted_optional_chain::NoNonNullAssertedOptionalChain::new(),
    no_non_null_assertion::NoNonNullAssertion::new(),
    no_obj_calls::NoObjCalls::new(),
    no_octal::NoOctal::new(),
    no_prototype_builtins::NoPrototypeBuiltins::new(),
    no_redeclare::NoRedeclare::new(),
    no_regex_spaces::NoRegexSpaces::new(),
    no_self_assign::NoSelfAssign::new(),
    no_setter_return::NoSetterReturn::new(),
    no_shadow_restricted_names::NoShadowRestrictedNames::new(),
    no_sparse_arrays::NoSparseArrays::new(),
    no_this_alias::NoThisAlias::new(),
    no_this_before_super::NoThisBeforeSuper::new(),
    no_throw_literal::NoThrowLiteral::new(),
    no_undef::NoUndef::new(),
    no_unreachable::NoUnreachable::new(),
    no_unsafe_finally::NoUnsafeFinally::new(),
    no_unsafe_negation::NoUnsafeNegation::new(),
    no_unused_labels::NoUnusedLabels::new(),
    no_unused_vars::NoUnusedVars::new(),
    no_var::NoVar::new(),
    no_with::NoWith::new(),
    prefer_as_const::PreferAsConst::new(),
    prefer_const::PreferConst::new(),
    prefer_namespace_keyword::PreferNamespaceKeyword::new(),
    require_yield::RequireYield::new(),
    single_var_declarator::SingleVarDeclarator::new(),
    triple_slash_reference::TripleSlashReference::new(),
    use_isnan::UseIsNaN::new(),
    valid_typeof::ValidTypeof::new(),
  ]
}

pub fn get_recommended_rules() -> Vec<Box<dyn LintRule>> {
  get_all_rules()
    .into_iter()
    .filter(|r| r.tags().contains(&"recommended"))
    .collect()
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn recommended_rules_sorted_alphabetically() {
    let mut recommended_rules = get_recommended_rules();
    recommended_rules.sort_by_key(|r| r.code());
    for (sorted, unsorted) in
      recommended_rules.into_iter().zip(get_recommended_rules())
    {
      assert_eq!(sorted.code(), unsorted.code());
    }
  }

  #[test]
  fn all_rules_sorted_alphabetically() {
    let mut all_rules = get_all_rules();
    all_rules.sort_by_key(|r| r.code());
    for (sorted, unsorted) in all_rules.into_iter().zip(get_all_rules()) {
      assert_eq!(sorted.code(), unsorted.code());
    }
  }
}
