// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::TsKeywordType;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoExplicitAny;

const CODE: &str = "no-explicit-any";
const MESSAGE: &str = "`any` type is not allowed";

impl LintRule for NoExplicitAny {
  fn new() -> Box<Self> {
    Box::new(NoExplicitAny)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
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
      self
        .context
        .add_diagnostic(ts_keyword_type.span, CODE, MESSAGE);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

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
    assert_lint_err! {
      NoExplicitAny,
      "function foo(): any { return undefined; }": [{ col: 16, message: MESSAGE }],
      "function bar(): Promise<any> { return undefined; }": [{ col: 24, message: MESSAGE }],
      "const a: any = {};": [{ col: 9, message: MESSAGE }],
      r#"
class Foo {
  static _extensions: {
    [key: string]: (module: Module, filename: string) => any;
  } = Object.create(null);
}"#: [{ line: 4, col: 57, message: MESSAGE }],
      r#"
type RequireWrapper = (
  exports: any,
  require: any,
  module: Module,
  __filename: string,
  __dirname: string
) => void;"#: [{ line: 3, col: 11, message: MESSAGE }, { line: 4, col: 11, message: MESSAGE }],
    }
  }
}
