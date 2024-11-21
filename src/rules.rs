// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

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
pub mod button_has_type;
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
pub mod jsx_boolean_value;
pub mod jsx_curly_braces;
pub mod jsx_key;
pub mod jsx_no_children_prop;
pub mod jsx_no_comment_text_nodes;
pub mod jsx_no_danger_with_children;
pub mod jsx_no_duplicate_props;
pub mod jsx_no_target_blank;
pub mod jsx_no_unescaped_entities;
pub mod jsx_no_useless_fragment;
pub mod jsx_props_no_spread_multi;
pub mod no_array_constructor;
pub mod no_async_promise_executor;
pub mod no_await_in_loop;
pub mod no_await_in_sync_fn;
pub mod no_boolean_literal_for_arguments;
pub mod no_case_declarations;
pub mod no_class_assign;
pub mod no_compare_neg_zero;
pub mod no_cond_assign;
pub mod no_console;
pub mod no_const_assign;
pub mod no_constant_condition;
pub mod no_control_regex;
pub mod no_danger;
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
pub mod no_fallthrough;
pub mod no_func_assign;
pub mod no_global_assign;
pub mod no_implicit_declare_namespace_export;
pub mod no_import_assertions;
pub mod no_import_assign;
pub mod no_inferrable_types;
pub mod no_inner_declarations;
pub mod no_invalid_regexp;
pub mod no_invalid_triple_slash_reference;
pub mod no_irregular_whitespace;
pub mod no_misused_new;
pub mod no_namespace;
pub mod no_new_symbol;
pub mod no_node_globals;
pub mod no_non_null_asserted_optional_chain;
pub mod no_non_null_assertion;
pub mod no_obj_calls;
pub mod no_octal;
pub mod no_process_global;
pub mod no_prototype_builtins;
pub mod no_redeclare;
pub mod no_regex_spaces;
pub mod no_self_assign;
pub mod no_self_compare;
pub mod no_setter_return;
pub mod no_shadow_restricted_names;
pub mod no_sparse_arrays;
pub mod no_sync_fn_in_async_fn;
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
pub mod no_window;
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
pub mod verbatim_module_syntax;

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

pub fn get_all_rules() -> Vec<Box<dyn LintRule>> {
  get_all_rules_raw()
}

/// Filters the lint rules to only the recommended rules.
pub fn recommended_rules(
  all_rules: Vec<Box<dyn LintRule>>,
) -> Vec<Box<dyn LintRule>> {
  all_rules
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
pub fn filtered_rules(
  all_rules: Vec<Box<dyn LintRule>>,
  maybe_tags: Option<Vec<String>>,
  maybe_exclude: Option<Vec<String>>,
  maybe_include: Option<Vec<String>>,
) -> Vec<Box<dyn LintRule>> {
  let tags_set =
    maybe_tags.map(|tags| tags.into_iter().collect::<HashSet<_>>());

  let mut rules = all_rules
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
pub(crate) fn sort_rules_by_priority(rules: &mut [Box<dyn LintRule>]) {
  rules.sort_by(|rule1, rule2| {
    let priority_cmp = rule1.priority().cmp(&rule2.priority());

    if priority_cmp == Ordering::Equal {
      return rule1.code().cmp(rule2.code());
    }

    priority_cmp
  });
}

fn get_all_rules_raw() -> Vec<Box<dyn LintRule>> {
  vec![
    Box::new(adjacent_overload_signatures::AdjacentOverloadSignatures),
    Box::new(ban_ts_comment::BanTsComment),
    Box::new(ban_types::BanTypes),
    Box::new(ban_unknown_rule_code::BanUnknownRuleCode),
    Box::new(ban_untagged_ignore::BanUntaggedIgnore),
    Box::new(ban_untagged_todo::BanUntaggedTodo),
    Box::new(ban_unused_ignore::BanUnusedIgnore),
    Box::new(button_has_type::ButtonHasType),
    Box::new(camelcase::Camelcase),
    Box::new(constructor_super::ConstructorSuper),
    Box::new(default_param_last::DefaultParamLast),
    Box::new(eqeqeq::Eqeqeq),
    Box::new(explicit_function_return_type::ExplicitFunctionReturnType),
    Box::new(explicit_module_boundary_types::ExplicitModuleBoundaryTypes),
    Box::new(for_direction::ForDirection),
    Box::new(fresh_handler_export::FreshHandlerExport),
    Box::new(fresh_server_event_handlers::FreshServerEventHandlers),
    Box::new(getter_return::GetterReturn),
    Box::new(guard_for_in::GuardForIn),
    Box::new(jsx_boolean_value::JSXBooleanValue),
    Box::new(jsx_curly_braces::JSXCurlyBraces),
    Box::new(jsx_key::JSXKey),
    Box::new(jsx_no_children_prop::JSXNoChildrenProp),
    Box::new(jsx_no_comment_text_nodes::JSXNoCommentTextNodes),
    Box::new(jsx_no_danger_with_children::JSXNoDangerWithChildren),
    Box::new(jsx_no_duplicate_props::JSXNoDuplicateProps),
    Box::new(jsx_no_target_blank::JSXNoTargetBlank),
    Box::new(jsx_no_unescaped_entities::JSXNoUnescapedEntities),
    Box::new(jsx_no_useless_fragment::JSXNoUselessFragment),
    Box::new(jsx_props_no_spread_multi::JSXPropsNoSpreadMulti),
    Box::new(no_array_constructor::NoArrayConstructor),
    Box::new(no_async_promise_executor::NoAsyncPromiseExecutor),
    Box::new(no_await_in_loop::NoAwaitInLoop),
    Box::new(no_await_in_sync_fn::NoAwaitInSyncFn),
    Box::new(no_boolean_literal_for_arguments::NoBooleanLiteralForArguments),
    Box::new(no_case_declarations::NoCaseDeclarations),
    Box::new(no_class_assign::NoClassAssign),
    Box::new(no_compare_neg_zero::NoCompareNegZero),
    Box::new(no_cond_assign::NoCondAssign),
    Box::new(no_console::NoConsole),
    Box::new(no_const_assign::NoConstAssign),
    Box::new(no_constant_condition::NoConstantCondition),
    Box::new(no_control_regex::NoControlRegex),
    Box::new(no_danger::NoDanger),
    Box::new(no_debugger::NoDebugger),
    Box::new(no_delete_var::NoDeleteVar),
    Box::new(no_deprecated_deno_api::NoDeprecatedDenoApi),
    Box::new(no_dupe_args::NoDupeArgs),
    Box::new(no_dupe_class_members::NoDupeClassMembers),
    Box::new(no_dupe_else_if::NoDupeElseIf),
    Box::new(no_dupe_keys::NoDupeKeys),
    Box::new(no_duplicate_case::NoDuplicateCase),
    Box::new(no_empty::NoEmpty),
    Box::new(no_empty_character_class::NoEmptyCharacterClass),
    Box::new(no_empty_enum::NoEmptyEnum),
    Box::new(no_empty_interface::NoEmptyInterface),
    Box::new(no_empty_pattern::NoEmptyPattern),
    Box::new(no_eval::NoEval),
    Box::new(no_ex_assign::NoExAssign),
    Box::new(no_explicit_any::NoExplicitAny),
    Box::new(no_external_imports::NoExternalImport),
    Box::new(no_extra_boolean_cast::NoExtraBooleanCast),
    Box::new(no_extra_non_null_assertion::NoExtraNonNullAssertion),
    Box::new(no_fallthrough::NoFallthrough),
    Box::new(no_func_assign::NoFuncAssign),
    Box::new(no_global_assign::NoGlobalAssign),
    Box::new(
      no_implicit_declare_namespace_export::NoImplicitDeclareNamespaceExport,
    ),
    Box::new(no_import_assertions::NoImportAssertions),
    Box::new(no_import_assign::NoImportAssign),
    Box::new(no_inferrable_types::NoInferrableTypes),
    Box::new(no_inner_declarations::NoInnerDeclarations),
    Box::new(no_invalid_regexp::NoInvalidRegexp),
    Box::new(no_invalid_triple_slash_reference::NoInvalidTripleSlashReference),
    Box::new(no_irregular_whitespace::NoIrregularWhitespace),
    Box::new(no_misused_new::NoMisusedNew),
    Box::new(no_namespace::NoNamespace),
    Box::new(no_new_symbol::NoNewSymbol),
    Box::new(no_node_globals::NoNodeGlobals),
    Box::new(
      no_non_null_asserted_optional_chain::NoNonNullAssertedOptionalChain,
    ),
    Box::new(no_non_null_assertion::NoNonNullAssertion),
    Box::new(no_obj_calls::NoObjCalls),
    Box::new(no_octal::NoOctal),
    Box::new(no_process_global::NoProcessGlobal),
    Box::new(no_prototype_builtins::NoPrototypeBuiltins),
    Box::new(no_redeclare::NoRedeclare),
    Box::new(no_regex_spaces::NoRegexSpaces),
    Box::new(no_self_assign::NoSelfAssign),
    Box::new(no_self_compare::NoSelfCompare),
    Box::new(no_setter_return::NoSetterReturn),
    Box::new(no_shadow_restricted_names::NoShadowRestrictedNames),
    Box::new(no_sparse_arrays::NoSparseArrays),
    Box::new(no_sync_fn_in_async_fn::NoSyncFnInAsyncFn),
    Box::new(no_this_alias::NoThisAlias),
    Box::new(no_this_before_super::NoThisBeforeSuper),
    Box::new(no_throw_literal::NoThrowLiteral),
    Box::new(no_top_level_await::NoTopLevelAwait),
    Box::new(no_undef::NoUndef),
    Box::new(no_unreachable::NoUnreachable),
    Box::new(no_unsafe_finally::NoUnsafeFinally),
    Box::new(no_unsafe_negation::NoUnsafeNegation),
    Box::new(no_unused_labels::NoUnusedLabels),
    Box::new(no_unused_vars::NoUnusedVars),
    Box::new(no_var::NoVar),
    Box::new(no_window::NoWindow),
    Box::new(no_window_prefix::NoWindowPrefix),
    Box::new(no_with::NoWith),
    Box::new(prefer_as_const::PreferAsConst),
    Box::new(prefer_ascii::PreferAscii),
    Box::new(prefer_const::PreferConst),
    Box::new(prefer_namespace_keyword::PreferNamespaceKeyword),
    Box::new(prefer_primordials::PreferPrimordials),
    Box::new(require_await::RequireAwait),
    Box::new(require_yield::RequireYield),
    Box::new(single_var_declarator::SingleVarDeclarator),
    Box::new(triple_slash_reference::TripleSlashReference),
    Box::new(use_isnan::UseIsNaN),
    Box::new(valid_typeof::ValidTypeof),
    Box::new(verbatim_module_syntax::VerbatimModuleSyntax),
  ]
}

#[cfg(test)]
mod tests {
  use std::sync::Arc;

  use super::*;

  #[test]
  fn recommended_rules_sorted_alphabetically() {
    let mut sorted_recommended_rules = recommended_rules(get_all_rules());
    sorted_recommended_rules.sort_by_key(|r| r.code());

    for (sorted, unsorted) in sorted_recommended_rules
      .iter()
      .zip(recommended_rules(get_all_rules()).iter())
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
    let rules = filtered_rules(
      get_all_rules(),
      Some(vec!["recommended".to_string()]),
      None,
      None,
    );
    for (r, rr) in rules.iter().zip(recommended_rules(get_all_rules()).iter()) {
      assert_eq!(r.code(), rr.code());
    }

    // Should allow to add more rules to recommended rules.
    let rules = filtered_rules(
      get_all_rules(),
      Some(vec!["recommended".to_string()]),
      None,
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), recommended_rules(get_all_rules()).len() + 1);

    // Recommended should allow to exclude some recommended rules and include more on top.
    let rules = filtered_rules(
      get_all_rules(),
      Some(vec!["recommended".to_string()]),
      Some(vec!["ban-ts-comment".to_string()]),
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), recommended_rules(get_all_rules()).len());

    // Should skip all rules if given empty tags vec.
    let rules = filtered_rules(get_all_rules(), Some(vec![]), None, None);
    assert!(rules.is_empty());

    // Should still allow to include rules when passed empty tags vec.
    let rules = filtered_rules(
      get_all_rules(),
      Some(vec![]),
      None,
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), 1);

    // Excluded rules should have priority over included rules.
    let rules = filtered_rules(
      get_all_rules(),
      Some(vec![]),
      Some(vec!["ban-untagged-todo".to_string()]),
      Some(vec!["ban-untagged-todo".to_string()]),
    );
    assert_eq!(rules.len(), 0);

    // Should still allow to include other rules, when other duplicates are excluded.
    let rules = filtered_rules(
      get_all_rules(),
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

    let rules = Arc::new(recommended_rules(get_all_rules()));
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
    let mut rules: Vec<Box<dyn LintRule>> = vec![
      Box::new(ban_unknown_rule_code::BanUnknownRuleCode),
      Box::new(ban_unused_ignore::BanUnusedIgnore),
      Box::new(no_redeclare::NoRedeclare),
      Box::new(eqeqeq::Eqeqeq),
    ];

    sort_rules_by_priority(&mut rules);

    assert_eq!(rules[0].code(), "eqeqeq");
    assert_eq!(rules[1].code(), "no-redeclare");
    assert_eq!(rules[2].code(), "ban-unknown-rule-code");
    assert_eq!(rules[3].code(), "ban-unused-ignore");
  }
}
