// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::linter::Context;

mod ban_ts_ignore;
mod ban_untagged_todo;
mod constructor_super;
mod default_param_last;
mod eqeqeq;
mod explicit_function_return_type;
mod for_direction;
mod getter_return;
mod no_async_promise_executor;
mod no_case_declarations;
mod no_compare_neg_zero;
mod no_cond_assign;
mod no_debugger;
mod no_delete_var;
mod no_dupe_args;
mod no_dupe_keys;
mod no_duplicate_case;
mod no_empty;
mod no_empty_function;
mod no_empty_interface;
mod no_eval;
mod no_explicit_any;
mod no_new_symbol;
mod no_prototype_builtins;
mod no_setter_return;
mod no_sparse_array;
mod no_throw_literal;
mod no_unsafe_finally;
mod no_var;
mod no_with;
mod require_yield;
mod single_var_declarator;
mod use_isnan;
mod valid_typeof;

pub trait LintRule {
  fn new() -> Box<Self>
  where
    Self: Sized;
  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module);
}

pub fn get_all_rules() -> Vec<Box<dyn LintRule>> {
  vec![
    constructor_super::ConstructorSuper::new(),
    no_explicit_any::NoExplicitAny::new(),
    no_debugger::NoDebugger::new(),
    no_var::NoVar::new(),
    single_var_declarator::SingleVarDeclarator::new(),
    explicit_function_return_type::ExplicitFunctionReturnType::new(),
    no_eval::NoEval::new(),
    no_empty_interface::NoEmptyInterface::new(),
    no_delete_var::NoDeleteVar::new(),
    use_isnan::UseIsNaN::new(),
    no_empty_function::NoEmptyFunction::new(),
    no_async_promise_executor::NoAsyncPromiseExecutor::new(),
    no_sparse_array::NoSparseArray::new(),
    no_duplicate_case::NoDuplicateCase::new(),
    no_dupe_args::NoDupeArgs::new(),
    ban_ts_ignore::BanTsIgnore::new(),
    ban_untagged_todo::BanUntaggedTodo::new(),
    getter_return::GetterReturn::new(),
    no_setter_return::NoSetterReturn::new(),
    eqeqeq::Eqeqeq::new(),
    no_dupe_keys::NoDupeKeys::new(),
    no_compare_neg_zero::NoCompareNegZero::new(),
    no_unsafe_finally::NoUnsafeFinally::new(),
    valid_typeof::ValidTypeof::new(),
    no_throw_literal::NoThrowLiteral::new(),
    no_new_symbol::NoNewSymbol::new(),
    default_param_last::DefaultParamLast::new(),
    no_empty::NoEmpty::new(),
    no_cond_assign::NoCondAssign::new(),
    no_with::NoWith::new(),
    no_case_declarations::NoCaseDeclarations::new(),
    require_yield::RequireYield::new(),
    no_prototype_builtins::NoPrototypeBuiltins::new(),
    for_direction::ForDirection::new(),
  ]
}
