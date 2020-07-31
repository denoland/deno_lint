// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_atoms::JsWord;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct BanTypes;

impl LintRule for BanTypes {
  fn new() -> Box<Self> {
    Box::new(BanTypes)
  }

  fn code(&self) -> &'static str {
    "ban-types"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecmascript::ast::Module) {
    let mut visitor = BanTypesVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct BanTypesVisitor {
  context: Arc<Context>,
}

impl BanTypesVisitor {
  pub fn new(context: Arc<Context>) -> Self {
    Self { context }
  }
}

const BANNED_TYPES: [(&str, &str); 6] = [
  ("String", "Use `string` instead"),
  ("Boolean", "Use `boolean` instead"),
  ("Number", "Use `number` instead"),
  ("Symbol", "Use `symbol` instead"),
  ("Function", "Define the function shape Explicitly."),
  ("Object", "if you want a type meaning `any object` use `Record<string, unknown>` instead,
or if you want a type meaning `any value`, you probably want `unknown` instead."),
];

impl Visit for BanTypesVisitor {
  fn visit_ts_type_ref(
    &mut self,
    ts_type_ref: &swc_ecmascript::ast::TsTypeRef,
    _parent: &dyn Node,
  ) {
    if let swc_ecmascript::ast::TsEntityName::Ident(ident) = &ts_type_ref.type_name {
      if let Some((_, message)) = BANNED_TYPES
        .iter()
        .find(|banned_type| JsWord::from(banned_type.0) == ident.sym)
      {
        self
          .context
          .add_diagnostic(ts_type_ref.span, "ban-types", message);
      }
    }
    if let Some(type_param) = &ts_type_ref.type_params {
      self.visit_ts_type_param_instantiation(type_param, ts_type_ref);
    }
  }
  fn visit_ts_type_lit(
    &mut self,
    ts_type_lit: &swc_ecmascript::ast::TsTypeLit,
    _parent: &dyn Node,
  ) {
    if !ts_type_lit.members.is_empty() {
      for element in ts_type_lit.members.iter() {
        self.visit_ts_type_element(element, ts_type_lit);
      }
      return;
    }
    self.context.add_diagnostic(
      ts_type_lit.span,
      "ban-types",
      BANNED_TYPES[5].1, // `Object` message
    );
  }
  fn visit_ts_keyword_type(
    &mut self,
    ts_keyword_type: &swc_ecmascript::ast::TsKeywordType,
    _parent: &dyn Node,
  ) {
    if let swc_ecmascript::ast::TsKeywordTypeKind::TsObjectKeyword =
      ts_keyword_type.kind
    {
      self.context.add_diagnostic(
        ts_keyword_type.span,
        "ban-types",
        "Use `Record<string, unknown>` instead",
      );
    }
  }
  fn visit_ts_type_param_instantiation(
    &mut self,
    ts_type_param_instantiation: &swc_ecmascript::ast::TsTypeParamInstantiation,
    _parent: &dyn Node,
  ) {
    for param in ts_type_param_instantiation.params.iter() {
      self.visit_ts_type(&param, ts_type_param_instantiation);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ban_types_valid() {
    assert_lint_ok::<BanTypes>("let f = Object();");
    assert_lint_ok::<BanTypes>(
      "let f: { x: number; y: number } = { x: 1, y: 1 };",
    );
    assert_lint_ok::<BanTypes>("let f = Object();");
    assert_lint_ok::<BanTypes>("let g = Object.create(null);");
    assert_lint_ok::<BanTypes>("let h = String(false);");
    assert_lint_ok::<BanTypes>("let e: foo.String;");
  }

  #[test]
  fn ban_types_invalid() {
    assert_lint_err::<BanTypes>("let a: String;", 7);
    assert_lint_err::<BanTypes>("let a: Object;", 7);
    assert_lint_err::<BanTypes>("let a: Number;", 7);
    assert_lint_err::<BanTypes>("let a: Function;", 7);
    assert_lint_err::<BanTypes>("let a: object;", 7);
    assert_lint_err::<BanTypes>("let a: {};", 7);
    assert_lint_err::<BanTypes>("let a: { b: String};", 12);
    assert_lint_err::<BanTypes>("let a: { b: Number};", 12);
    assert_lint_err_n::<BanTypes>(
      "let a: { b: object, c: Object};",
      vec![12, 23],
    );
    assert_lint_err::<BanTypes>("let a: { b: { c : Function}};", 18);
    assert_lint_err::<BanTypes>("let a: Array<String>", 13);
    assert_lint_err_n::<BanTypes>("let a: Number<Function>", vec![7, 14]);
    assert_lint_err::<BanTypes>("function foo(a: String) {}", 16);
    assert_lint_err::<BanTypes>("function foo(): Number {}", 16);
    assert_lint_err::<BanTypes>("let a: () => Number;", 13);
    assert_lint_err::<BanTypes>("'a' as String;", 7);
    assert_lint_err::<BanTypes>("1 as Number;", 5);
    assert_lint_err_on_line_n::<BanTypes>(
      "
class Foo<F = String> extends Bar<String> implements Baz<Object> {
  constructor(foo: String | Object) {}
    
  exit(): Array<String> {
    const foo: String = 1 as String;
  }
}",
      vec![
        (2, 14),
        (2, 34),
        (2, 57),
        (3, 19),
        (3, 28),
        (5, 16),
        (6, 15),
        (6, 29),
      ],
    )
  }
}
