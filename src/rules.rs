// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::context::Context;
use crate::Program;
use crate::ProgramRef;
use std::cmp::Ordering;
use std::collections::HashSet;

pub mod adjacent_overload_signatures;
pub mod ban_ts_comment;
pub mod ban_types;
pub mod ban_unknown_rule_code;
pub mod ban_untagged_ignore;
pub mod ban_untagged_todo;
pub mod ban_unused_ignore;
pub mod camelcase;
pub mod constructor_super;
pub mod default_param_last;
pub mod eqeqeq;
pub mod explicit_function_return_type;
pub mod explicit_module_boundary_types;
pub mod for_direction;
pub mod fresh_handler_export;
pub mod fresh_server_event_handlers;
pub mod getter_return;
pub mod guard_for_in;
pub mod no_array_constructor;
pub mod no_async_promise_executor;
pub mod no_await_in_loop;
pub mod no_await_in_sync_fn;
pub mod no_case_declarations;
pub mod no_class_assign;
pub mod no_compare_neg_zero;
pub mod no_cond_assign;
pub mod no_const_assign;
pub mod no_constant_condition;
pub mod no_control_regex;
pub mod no_debugger;
pub mod no_delete_var;
pub mod no_deprecated_deno_api;
pub mod no_dupe_args;
pub mod no_dupe_class_members;
pub mod no_dupe_else_if;
pub mod no_dupe_keys;
pub mod no_duplicate_case;
pub mod no_empty;
pub mod no_empty_character_class;
pub mod no_empty_enum;
pub mod no_empty_interface;
pub mod no_empty_pattern;
pub mod no_eval;
pub mod no_ex_assign;
pub mod no_explicit_any;
pub mod no_external_imports;
pub mod no_extra_boolean_cast;
pub mod no_extra_non_null_assertion;
pub mod no_extra_semi;
pub mod no_fallthrough;
pub mod no_func_assign;
pub mod no_global_assign;
pub mod no_implicit_declare_namespace_export;
pub mod no_import_assign;
pub mod no_inferrable_types;
pub mod no_inner_declarations;
pub mod no_invalid_regexp;
pub mod no_invalid_triple_slash_reference;
pub mod no_irregular_whitespace;
pub mod no_misused_new;
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
pub mod no_top_level_await;
pub mod no_undef;
pub mod no_unreachable;
pub mod no_unsafe_finally;
pub mod no_unsafe_negation;
pub mod no_unused_labels;
pub mod no_unused_vars;
pub mod no_var;
pub mod no_window_prefix;
pub mod no_with;
pub mod prefer_as_const;
pub mod prefer_ascii;
pub mod prefer_const;
pub mod prefer_namespace_keyword;
pub mod prefer_primordials;
pub mod require_await;
pub mod require_yield;
pub mod single_var_declarator;
pub mod triple_slash_reference;
pub mod use_isnan;
pub mod valid_typeof;

pub trait LintRule: std::fmt::Debug + Send + Sync {
  /// Executes lint using `dprint-swc-ecma-ast-view`.
  /// Falls back to the `lint_program` method if not implemented.
  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  );

  /// Returns the unique code that identifies the rule
  fn code(&self) -> &'static str;

  /// Returns the tags this rule belongs to, e.g. `recommended`
  fn tags(&self) -> &'static [&'static str] {
    &[]
  }

  /// Returns the documentation string for this rule, describing what this rule is for with several
  /// examples.
  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str;

  /// The lower the return value is, the earlier this rule will be run.
  ///
  /// By default it is 0. Some rules might want to defer being run to the end
  /// and they might override this value.
  fn priority(&self) -> u32 {
    0
  }
}

/// TODO(@magurotuna): remove this after all rules get to use ast_view
pub fn program_ref(program: Program) -> ProgramRef {
  match program {
    Program::Module(m) => ProgramRef::Module(m.inner),
    Program::Script(s) => ProgramRef::Script(s.inner),
  }
}

pub fn get_all_rules() -> Vec<&'static dyn LintRule> {
  get_all_rules_raw()
}

pub fn get_recommended_rules() -> Vec<&'static dyn LintRule> {
  get_all_rules_raw()
    .into_iter()
    .filter(|r| r.tags().contains(&"recommended"))
    .collect()
}

/// Returns a list of rules after filtering.
///
/// Following rules are applied (in the described order):
///
/// - if `maybe_tags` is `None` then all defined rules are returned, otherwise
///   only rules matching at least one tag will be returned; if provided list
///   is empty then all rules will be excluded by default
///
/// - if `maybe_exclude` is `Some`, all rules with matching codes will
///   be filtered out
///
/// - if `maybe_include` is `Some`, rules with matching codes will be added
///   to the return list
///
/// Before returning the list will sorted alphabetically.
pub fn get_filtered_rules(
  maybe_tags: Option<Vec<String>>,
  maybe_exclude: Option<Vec<String>>,
  maybe_include: Option<Vec<String>>,
) -> Vec<&'static dyn LintRule> {
  let tags_set =
    maybe_tags.map(|tags| tags.into_iter().collect::<HashSet<_>>());

  let mut rules = get_all_rules_raw()
    .into_iter()
    .filter(|rule| {
      let mut passes = if let Some(tags_set) = &tags_set {
        rule
          .tags()
          .iter()
          .any(|t| tags_set.contains(&t.to_string()))
      } else {
        true
      };

      if let Some(includes) = &maybe_include {
        if includes.contains(&rule.code().to_owned()) {
          passes |= true;
        }
      }

      if let Some(excludes) = &maybe_exclude {
        if excludes.contains(&rule.code().to_owned()) {
          passes &= false;
        }
      }

      passes
    })
    .collect::<Vec<_>>();

  rules.sort_by_key(|r| r.code());

  rules
}

/// Sort lint rules by priority and alphabetically.
pub(crate) fn sort_rules_by_priority(rules: &mut [&'static dyn LintRule]) {
  rules.sort_by(|rule1, rule2| {
    let priority_cmp = rule1.priority().cmp(&rule2.priority());

    if priority_cmp == Ordering::Equal {
      return rule1.code().cmp(rule2.code());
    }

    priority_cmp
  });
}

fn get_all_rules_raw() -> Vec<&'static dyn LintRule> {
  vec![
    &adjacent_overload_signatures::AdjacentOverloadSignatures,
    &ban_ts_comment::BanTsComment,
    &ban_types::BanTypes,
    &ban_unknown_rule_code::BanUnknownRuleCode,
    &ban_untagged_ignore::BanUntaggedIgnore,
    &ban_untagged_todo::BanUntaggedTodo,
    &ban_unused_ignore::BanUnusedIgnore,
    &camelcase::Camelcase,
    &constructor_super::ConstructorSuper,
    &default_param_last::DefaultParamLast,
    &eqeqeq::Eqeqeq,
    &explicit_function_return_type::ExplicitFunctionReturnType,
    &explicit_module_boundary_types::ExplicitModuleBoundaryTypes,
    &for_direction::ForDirection,
    &fresh_handler_export::FreshHandlerExport,
    &fresh_server_event_handlers::FreshServerEventHandlers,
    &getter_return::GetterReturn,
    &guard_for_in::GuardForIn,
    &no_array_constructor::NoArrayConstructor,
    &no_async_promise_executor::NoAsyncPromiseExecutor,
    &no_await_in_loop::NoAwaitInLoop,
    &no_await_in_sync_fn::NoAwaitInSyncFn,
    &no_case_declarations::NoCaseDeclarations,
    &no_class_assign::NoClassAssign,
    &no_compare_neg_zero::NoCompareNegZero,
    &no_cond_assign::NoCondAssign,
    &no_const_assign::NoConstAssign,
    &no_constant_condition::NoConstantCondition,
    &no_control_regex::NoControlRegex,
    &no_debugger::NoDebugger,
    &no_delete_var::NoDeleteVar,
    &no_deprecated_deno_api::NoDeprecatedDenoApi,
    &no_dupe_args::NoDupeArgs,
    &no_dupe_class_members::NoDupeClassMembers,
    &no_dupe_else_if::NoDupeElseIf,
    &no_dupe_keys::NoDupeKeys,
    &no_duplicate_case::NoDuplicateCase,
    &no_empty::NoEmpty,
    &no_empty_character_class::NoEmptyCharacterClass,
    &no_empty_enum::NoEmptyEnum,
    &no_empty_interface::NoEmptyInterface,
    &no_empty_pattern::NoEmptyPattern,
    &no_eval::NoEval,
    &no_ex_assign::NoExAssign,
    &no_explicit_any::NoExplicitAny,
    &no_external_imports::NoExternalImport,
    &no_extra_boolean_cast::NoExtraBooleanCast,
    &no_extra_non_null_assertion::NoExtraNonNullAssertion,
    &no_extra_semi::NoExtraSemi,
    &no_fallthrough::NoFallthrough,
    &no_func_assign::NoFuncAssign,
    &no_global_assign::NoGlobalAssign,
    &no_implicit_declare_namespace_export::NoImplicitDeclareNamespaceExport,
    &no_import_assign::NoImportAssign,
    &no_inferrable_types::NoInferrableTypes,
    &no_inner_declarations::NoInnerDeclarations,
    &no_invalid_regexp::NoInvalidRegexp,
    &no_invalid_triple_slash_reference::NoInvalidTripleSlashReference,
    &no_irregular_whitespace::NoIrregularWhitespace,
    &no_misused_new::NoMisusedNew,
    &no_namespace::NoNamespace,
    &no_new_symbol::NoNewSymbol,
    &no_non_null_asserted_optional_chain::NoNonNullAssertedOptionalChain,
    &no_non_null_assertion::NoNonNullAssertion,
    &no_obj_calls::NoObjCalls,
    &no_octal::NoOctal,
    &no_prototype_builtins::NoPrototypeBuiltins,
    &no_redeclare::NoRedeclare,
    &no_regex_spaces::NoRegexSpaces,
    &no_self_assign::NoSelfAssign,
    &no_setter_return::NoSetterReturn,
    &no_shadow_restricted_names::NoShadowRestrictedNames,
    &no_sparse_arrays::NoSparseArrays,
    &no_this_alias::NoThisAlias,
    &no_this_before_super::NoThisBeforeSuper,
    &no_throw_literal::NoThrowLiteral,
    &no_top_level_await::NoTopLevelAwait,
    &no_undef::NoUndef,
    &no_unreachable::NoUnreachable,
    &no_unsafe_finally::NoUnsafeFinally,
    &no_unsafe_negation::NoUnsafeNegation,
    &no_unused_labels::NoUnusedLabels,
    &no_unused_vars::NoUnusedVars,
    &no_var::NoVar,
    &no_window_prefix::NoWindowPrefix,
    &no_with::NoWith,
    &prefer_as_const::PreferAsConst,
    &prefer_ascii::PreferAscii,
    &prefer_const::PreferConst,
    &prefer_namespace_keyword::PreferNamespaceKeyword,
    &prefer_primordials::PreferPrimordials,
    &require_await::RequireAwait,
    &require_yield::RequireYield,
    &single_var_declarator::SingleVarDeclarator,
    &triple_slash_reference::TripleSlashReference,
    &use_isnan::UseIsNaN,
    &valid_typeof::ValidTypeof,
  ]
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;

  #[test]
  fn recommended_rules_sorted_alphabetically() {
    let mut sorted_recommended_rules = get_recommended_rules();
    sorted_recommended_rules.sort_by_key(|r| r.code());

    for (sorted, unsorted) in sorted_recommended_rules
      .iter()
      .zip(get_recommended_rules().iter())
    {
      assert_eq!(sorted.code(), unsorted.code());
    }
  }

  #[test]
  fn all_rules_sorted_alphabetically() {
    let mut all_rules = get_all_rules_raw();
    all_rules.sort_by_key(|r| r.code());
    for (sorted, unsorted) in all_rules.iter().zip(get_all_rules_raw()) {
      assert_eq!(sorted.code(), unsorted.code());
    }
  }

  #[test]
  fn test_get_filtered_rules() {
    // Should return recommended rules when given `recommended` tag.
    let rules =
      get_filtered_rules(Some(vec!["recommended".to_string()]), None, None);
    for (r, rr) in rules.iter().zip(get_recommended_rules().iter()) {
      assert_eq!(r.code(), rr.code());
    }

    // Should allow to add more rules to recommended rules.
    let rules = get_filtered_rules(
      Some(vec!["recommended".to_string()]),
      None,
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), get_recommended_rules().len() + 1);

    // Recommended should allow to exclude some recommended rules and include more on top.
    let rules = get_filtered_rules(
      Some(vec!["recommended".to_string()]),
      Some(vec!["ban-ts-comment".to_string()]),
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), get_recommended_rules().len());

    // Should skip all rules if given empty tags vec.
    let rules = get_filtered_rules(Some(vec![]), None, None);
    assert!(rules.is_empty());

    // Should still allow to include rules when passed empty tags vec.
    let rules = get_filtered_rules(
      Some(vec![]),
      None,
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), 1);

    // Excluded rules should have priority over included rules.
    let rules = get_filtered_rules(
      Some(vec![]),
      Some(vec!["ban-untagged-todo".to_string()]),
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), 0);

    // Should still allow to include other rules, when other duplicates are excluded.
    let rules = get_filtered_rules(
      Some(vec![]),
      Some(vec![
        "ban-untagged-todo".to_string(),
        "ban-ts-comment".to_string(),
      ]),
      Some(vec![
        "ban-untagged-todo".to_string(),
        "ban-ts-comment".to_string(),
        "no-const-assign".to_string(),
        "no-throw-literal".to_string(),
      ]),
    );
    assert_eq!(rules.len(), 2);
    assert_eq!(rules[0].code(), "no-const-assign");
    assert_eq!(rules[1].code(), "no-throw-literal");
  }

  #[test]
  fn ensure_lint_rules_are_sharable_across_threads() {
    use std::thread::spawn;

    let rules = Arc::new(get_recommended_rules());
    let handles = (0..2)
      .map(|_| {
        let rules = Arc::clone(&rules);
        spawn(move || {
          for rule in rules.iter() {
            assert!(rule.tags().contains(&"recommended"));
          }
        })
      })
      .collect::<Vec<_>>();

    for handle in handles {
      handle.join().unwrap();
    }
  }

  #[test]
  fn sort_by_priority() {
    let mut rules: Vec<&'static dyn LintRule> = vec![
      &ban_unknown_rule_code::BanUnknownRuleCode,
      &ban_unused_ignore::BanUnusedIgnore,
      &no_redeclare::NoRedeclare,
      &eqeqeq::Eqeqeq,
    ];

    sort_rules_by_priority(&mut rules);

    assert_eq!(rules[0].code(), "eqeqeq");
    assert_eq!(rules[1].code(), "no-redeclare");
    assert_eq!(rules[2].code(), "ban-unknown-rule-code");
    assert_eq!(rules[3].code(), "ban-unused-ignore");
  }
}
