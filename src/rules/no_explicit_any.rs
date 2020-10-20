// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::TsKeywordType;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoExplicitAny;

impl LintRule for NoExplicitAny {
  fn new() -> Box<Self> {
    Box::new(NoExplicitAny)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-explicit-any"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoExplicitAnyVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoExplicitAnyVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoExplicitAnyVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoExplicitAnyVisitor<'c> {
  fn visit_ts_keyword_type(
    &mut self,
    ts_keyword_type: &TsKeywordType,
    _parent: &dyn Node,
  ) {
    use swc_ecmascript::ast::TsKeywordTypeKind::*;

    if ts_keyword_type.kind == TsAnyKeyword {
      self.context.add_diagnostic(
        ts_keyword_type.span,
        "no-explicit-any",
        "`any` type is not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_explicit_any_valid() {
    assert_lint_ok! {
      NoExplicitAny,
      r#"
class Foo {
  static _extensions: {
    // deno-lint-ignore no-explicit-any
    [key: string]: (module: Module, filename: string) => any;
  } = Object.create(null);
}"#,
      r#"
type RequireWrapper = (
  // deno-lint-ignore no-explicit-any
  exports: any,
  // deno-lint-ignore no-explicit-any
  require: any,
  module: Module,
  __filename: string,
  __dirname: string
) => void;"#,
    };
  }

  #[test]
  fn no_explicit_any_invalid() {
    assert_lint_err::<NoExplicitAny>(
      "function foo(): any { return undefined; }",
      16,
    );
    assert_lint_err::<NoExplicitAny>(
      "function bar(): Promise<any> { return undefined; }",
      24,
    );
    assert_lint_err::<NoExplicitAny>("const a: any = {};", 9);

    assert_lint_err_on_line::<NoExplicitAny>(
      r#"
class Foo {
  static _extensions: {
    [key: string]: (module: Module, filename: string) => any;
  } = Object.create(null);
}"#,
      4,
      57,
    );

    assert_lint_err_on_line_n::<NoExplicitAny>(
      r#"
type RequireWrapper = (
  exports: any,
  require: any,
  module: Module,
  __filename: string,
  __dirname: string
) => void;"#,
      vec![(3, 11), (4, 11)],
    );
  }
}
