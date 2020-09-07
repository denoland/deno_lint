// Copyright 2020 the Deno authors. All rights reserved. MIT license.
// TODO(magurotuna): remove next line
#![allow(unused)]
use super::Context;
use super::LintRule;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct Camelcase;

impl LintRule for Camelcase {
  fn new() -> Box<Self> {
    Box::new(Camelcase)
  }

  fn code(&self) -> &'static str {
    "camelcase"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = CamelcaseVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct CamelcaseVisitor {
  context: Arc<Context>,
}

impl CamelcaseVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

impl Visit for CamelcaseVisitor {}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  // Based on https://github.com/eslint/eslint/blob/v7.8.1/tests/lib/rules/camelcase.js

  #[test]
  fn camelcase_valid() {
    assert_lint_ok::<Camelcase>(r#"firstName = "Ichigo""#);
    assert_lint_ok::<Camelcase>(r#"FIRST_NAME = "Ichigo""#);
    assert_lint_ok::<Camelcase>(r#"__myPrivateVariable = "Hoshimiya""#);
    assert_lint_ok::<Camelcase>(r#"myPrivateVariable_ = "Hoshimiya""#);
    assert_lint_ok::<Camelcase>(r#"function doSomething(){}"#);
    assert_lint_ok::<Camelcase>(r#"do_something()"#);
    assert_lint_ok::<Camelcase>(r#"new do_something"#);
    assert_lint_ok::<Camelcase>(r#"new do_something()"#);
    assert_lint_ok::<Camelcase>(r#"foo.do_something()"#);
    assert_lint_ok::<Camelcase>(r#"var foo = bar.baz_boom;"#);
    assert_lint_ok::<Camelcase>(r#"var foo = bar.baz_boom.something;"#);
    assert_lint_ok::<Camelcase>(
      r#"foo.boom_pow.qux = bar.baz_boom.something;"#,
    );
    assert_lint_ok::<Camelcase>(r#"if (bar.baz_boom) {}"#);
    assert_lint_ok::<Camelcase>(r#"var obj = { key: foo.bar_baz };"#);
    assert_lint_ok::<Camelcase>(r#"var arr = [foo.bar_baz];"#);
    assert_lint_ok::<Camelcase>(r#"[foo.bar_baz]"#);
    assert_lint_ok::<Camelcase>(r#"var arr = [foo.bar_baz.qux];"#);
    assert_lint_ok::<Camelcase>(r#"[foo.bar_baz.nesting]"#);
    assert_lint_ok::<Camelcase>(
      r#"if (foo.bar_baz === boom.bam_pow) { [foo.baz_boom] }"#,
    );
    assert_lint_ok::<Camelcase>(r#"var o = {key: 1}"#);
    assert_lint_ok::<Camelcase>(r#"var o = {_leading: 1}"#);
    assert_lint_ok::<Camelcase>(r#"var o = {trailing_: 1}"#);
    assert_lint_ok::<Camelcase>(r#"var o = {bar_baz: 1}"#);
    assert_lint_ok::<Camelcase>(r#"const { ['foo']: _foo } = obj;"#);
    assert_lint_ok::<Camelcase>(r#"const { [_foo_]: foo } = obj;"#);
    assert_lint_ok::<Camelcase>(r#"var { category_id: category } = query;"#);
    assert_lint_ok::<Camelcase>(r#"var { _leading } = query;"#);
    assert_lint_ok::<Camelcase>(r#"var { trailing_ } = query;"#);
    assert_lint_ok::<Camelcase>(
      r#"import { camelCased } from "external module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { _leading } from "external module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { trailing_ } from "external module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as camelCased } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as _leading } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as trailing_ } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"import { no_camelcased as camelCased, anotherCamelCased } from "external-module";"#,
    );
    assert_lint_ok::<Camelcase>(r#"import { camelCased } from 'mod'"#);
    assert_lint_ok::<Camelcase>(r#"var _camelCased = aGlobalVariable"#);
    assert_lint_ok::<Camelcase>(r#"var camelCased = _aGlobalVariable"#);
    assert_lint_ok::<Camelcase>(
      r#"function foo({ no_camelcased: camelCased }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ no_camelcased: _leading }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ no_camelcased: trailing_ }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ camelCased = 'default value' }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ _leading = 'default value' }) {};"#,
    );
    assert_lint_ok::<Camelcase>(
      r#"function foo({ trailing_ = 'default value' }) {};"#,
    );
    assert_lint_ok::<Camelcase>(r#"function foo({ camelCased }) {};"#);
    assert_lint_ok::<Camelcase>(r#"function foo({ _leading }) {}"#);
    assert_lint_ok::<Camelcase>(r#"function foo({ trailing_ }) {}"#);
    assert_lint_ok::<Camelcase>(r#"({obj} = baz.fo_o);"#);
    assert_lint_ok::<Camelcase>(r#"([obj] = baz.fo_o);"#);
    assert_lint_ok::<Camelcase>(r#"([obj.foo = obj.fo_o] = bar);"#);
  }

  #[test]
  fn camelcase_invalid() {
    assert_lint_err::<Camelcase>(r#"first_name = "Nicholas""#, 0);
    assert_lint_err::<Camelcase>(r#"__private_first_name = "Patrick""#, 0);
    assert_lint_err::<Camelcase>(r#"function foo_bar(){}"#, 0);
    assert_lint_err::<Camelcase>(r#"obj.foo_bar = function(){};"#, 0);
    assert_lint_err::<Camelcase>(r#"bar_baz.foo = function(){};"#, 0);
    assert_lint_err::<Camelcase>(r#"[foo_bar.baz]"#, 0);
    assert_lint_err::<Camelcase>(
      r#"if (foo.bar_baz === boom.bam_pow) { [foo_bar.baz] }"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"foo.bar_baz = boom.bam_pow"#, 0);
    assert_lint_err::<Camelcase>(r#"var foo = { bar_baz: boom.bam_pow }"#, 0);
    assert_lint_err::<Camelcase>(
      r#"foo.qux.boom_pow = { bar: boom.bam_pow }"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var o = {bar_baz: 1}"#, 0);
    assert_lint_err::<Camelcase>(r#"obj.a_b = 2;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"var { category_id: category_alias } = query;"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"var { [category_id]: categoryId } = query;"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var { category_id } = query;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"var { category_id: category_id } = query;"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var { category_id = 1 } = query;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"import no_camelcased from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import * as no_camelcased from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased as no_camel_cased } from "external module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { camelCased as no_camel_cased } from "external module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { camelCased, no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import { no_camelcased as camelCased, another_no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import camelCased, { no_camelcased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"import no_camelcased, { another_no_camelcased as camelCased } from "external-module";"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"import snake_cased from 'mod'"#, 0);
    assert_lint_err::<Camelcase>(r#"import * as snake_cased from 'mod'"#, 0);
    assert_lint_err::<Camelcase>(r#"var camelCased = snake_cased"#, 0);
    assert_lint_err::<Camelcase>(r#"a_global_variable.foo()"#, 0);
    assert_lint_err::<Camelcase>(r#"a_global_variable[undefined]"#, 0);
    assert_lint_err::<Camelcase>(r#"var camelCased = snake_cased"#, 0);
    assert_lint_err::<Camelcase>(r#"var camelCased = snake_cased"#, 0);
    assert_lint_err::<Camelcase>(r#"export * as snake_cased from 'mod'"#, 0);
    assert_lint_err::<Camelcase>(r#"function foo({ no_camelcased }) {};"#, 0);
    assert_lint_err::<Camelcase>(
      r#"function foo({ no_camelcased = 'default value' }) {};"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"const no_camelcased = 0; function foo({ camelcased_value = no_camelcased}) {}"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"const { bar: no_camelcased } = foo;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"function foo({ value_1: my_default }) {}"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"function foo({ isCamelcased: no_camelcased }) {};"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"var { foo: bar_baz = 1 } = quz;"#, 0);
    assert_lint_err::<Camelcase>(
      r#"const { no_camelcased = false } = bar;"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"const { no_camelcased = foo_bar } = bar;"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"not_ignored_foo = 0;"#, 0);
    assert_lint_err::<Camelcase>(r#"not_ignored_foo = 0;"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: obj.fo_o } = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: obj.fo_o.b_ar } = baz);"#, 0);
    assert_lint_err::<Camelcase>(
      r#"({ a: { b: { c: obj.fo_o } } } = bar);"#,
      0,
    );
    assert_lint_err::<Camelcase>(
      r#"({ a: { b: { c: obj.fo_o.b_ar } } } = baz);"#,
      0,
    );
    assert_lint_err::<Camelcase>(r#"([obj.fo_o] = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"([obj.fo_o = 1] = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: [obj.fo_o] } = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"({ a: { b: [obj.fo_o] } } = bar);"#, 0);
    assert_lint_err::<Camelcase>(r#"([obj.fo_o.ba_r] = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"({...obj.fo_o} = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"({...obj.fo_o.ba_r} = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"({c: {...obj.fo_o }} = baz);"#, 0);
    assert_lint_err::<Camelcase>(r#"obj.o_k.non_camelcase = 0"#, 0);
    assert_lint_err::<Camelcase>(r#"(obj?.o_k).non_camelcase = 0"#, 0);
  }
}
