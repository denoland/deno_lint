// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{ArrowExpr, ClassMethod, Function, MethodKind};
use swc_ecmascript::visit::{self, noop_visit_type, Node, Visit, VisitWith};

use std::sync::Arc;

pub struct ExplicitFunctionReturnType;

impl LintRule for ExplicitFunctionReturnType {
  fn new() -> Box<Self> {
    Box::new(ExplicitFunctionReturnType)
  }

  fn code(&self) -> &'static str {
    "explicit-function-return-type"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = ExplicitFunctionReturnTypeVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct ExplicitFunctionReturnTypeVisitor {
  context: Arc<Context>,
}

impl ExplicitFunctionReturnTypeVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn report(&self, span: Span) {
    self.context.add_diagnostic(
      span,
      "explicit-function-return-type",
      "Missing return type on function",
    );
  }
}

impl Visit for ExplicitFunctionReturnTypeVisitor {
  noop_visit_type!();

  fn visit_function(&mut self, function: &Function, _: &dyn Node) {
    if function.return_type.is_none() {
      self.report(function.span);
    }
    function.visit_children_with(self);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    if arrow_expr.return_type.is_none() {
      self.report(arrow_expr.span);
    }
    arrow_expr.visit_children_with(self);
  }

  fn visit_class_method(
    &mut self,
    class_method: &ClassMethod,
    parent: &dyn Node,
  ) {
    match class_method.kind {
      MethodKind::Setter => {
        visit::visit_function(self, &class_method.function, parent);
      }
      _ => {
        self.visit_function(&class_method.function, parent);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  // Some tests are derived from
  // https://github.com/typescript-eslint/typescript-eslint/blob/v4.1.1/packages/eslint-plugin/tests/rules/explicit-function-return-type.test.ts
  // BSD 2-clause Licensed.
  //
  // Copyright JS Foundation and other contributors, https://js.foundation
  // Redistribution and use in source and binary forms, with or without
  // modification, are permitted provided that the following conditions are met:
  //
  //   * Redistributions of source code must retain the above copyright
  //     notice, this list of conditions and the following disclaimer.
  //   * Redistributions in binary form must reproduce the above copyright
  //     notice, this list of conditions and the following disclaimer in the
  //     documentation and/or other materials provided with the distribution.
  //
  // THIS SOFTWARE IS PROVIDED BY THE COPYRIGHT HOLDERS AND CONTRIBUTORS "AS IS"
  // AND ANY EXPRESS OR IMPLIED WARRANTIES, INCLUDING, BUT NOT LIMITED TO, THE
  // IMPLIED WARRANTIES OF MERCHANTABILITY AND FITNESS FOR A PARTICULAR PURPOSE
  // ARE DISCLAIMED. IN NO EVENT SHALL <COPYRIGHT HOLDER> BE LIABLE FOR ANY
  // DIRECT, INDIRECT, INCIDENTAL, SPECIAL, EXEMPLARY, OR CONSEQUENTIAL DAMAGES
  // (INCLUDING, BUT NOT LIMITED TO, PROCUREMENT OF SUBSTITUTE GOODS OR SERVICES;
  // LOSS OF USE, DATA, OR PROFITS; OR BUSINESS INTERRUPTION) HOWEVER CAUSED AND
  // ON ANY THEORY OF LIABILITY, WHETHER IN CONTRACT, STRICT LIABILITY, OR TORT
  // (INCLUDING NEGLIGENCE OR OTHERWISE) ARISING IN ANY WAY OUT OF THE USE OF
  // THIS SOFTWARE, EVEN IF ADVISED OF THE POSSIBILITY OF SUCH DAMAGE.

  #[test]
  fn explicit_function_return_type_valid() {
    assert_lint_ok_n::<ExplicitFunctionReturnType>(vec![
      "function fooTyped(): void { }",
      "const bar = (a: string) => { }",
      "const barTyped = (a: string): Promise<void> => { }",
      r#"
function test(): void {
  return;
}
      "#,
      r#"
var fn = function (): number {
  return 1;
};
      "#,
      r#"
var arrowFn = (): string => 'test';
      "#,
      r#"
class Test {
  constructor() {}
  get prop(): number {
    return 1;
  }
  set prop() {}
  method(): void {
    return;
  }
  arrow = (): string => 'arrow';
}
      "#,
      r#"
var arrowFn: Foo = () => 'test';
      "#,
      r#"
var funcExpr: Foo = function () {
  return 'test';
};
      "#,
      r#"const x = (() => {}) as Foo;"#,
      r#"const x = <Foo>(() => {});"#,
      r#"
const x = {
  foo: () => {},
} as Foo;
      "#,
      r#"
const x = <Foo>{
  foo: () => {},
};
      "#,
      r#"
const x: Foo = {
  foo: () => {},
};
      "#,
      r#"
type MethodType = () => void;

class App {
  private method: MethodType = () => {};
}
      "#,
      r#"
const myObj = {
  set myProp(val) {
    this.myProp = val;
  },
};
      "#,
      r#"
() => (): void => {};
      "#,
      r#"
() => function (): void {};
      "#,
      r#"
() => {
  return (): void => {};
};
      "#,
      r#"
() => {
  return function (): void {};
};
      "#,
      r#"
function fn() {
  return (): void => {};
}
      "#,
      r#"
function fn() {
  return function (): void {};
}
      "#,
      r#"
function FunctionDeclaration() {
  return function FunctionExpression_Within_FunctionDeclaration() {
    return function FunctionExpression_Within_FunctionExpression() {
      return () => {
        // ArrowFunctionExpression_Within_FunctionExpression
        return () =>
          // ArrowFunctionExpression_Within_ArrowFunctionExpression
          (): number => 1; // ArrowFunctionExpression_Within_ArrowFunctionExpression_WithNoBody
      };
    };
  };
}
      "#,
      r#"
() => () => {
  return (): void => {
    return;
  };
};
      "#,
      r#"
declare function foo(arg: () => void): void;
foo(() => 1);
foo(() => {});
foo(() => null);
foo(() => true);
foo(() => '');
      "#,
      r#"
declare function foo(arg: () => void): void;
foo?.(() => 1);
foo?.bar(() => {});
foo?.bar?.(() => null);
foo.bar?.(() => true);
foo?.(() => '');
      "#,
      r#"
class Accumulator {
  private count: number = 0;

  public accumulate(fn: () => number): void {
    this.count += fn();
  }
}

new Accumulator().accumulate(() => 1);
      "#,
      r#"
declare function foo(arg: { meth: () => number }): void;
foo({
  meth() {
    return 1;
  },
});
foo({
  meth: function () {
    return 1;
  },
});
foo({
  meth: () => {
    return 1;
  },
});
      "#,
      r#"
const func = (value: number) => ({ type: 'X', value } as const);
const func = (value: number) => ({ type: 'X', value } as const);
const func = (value: number) => x as const;
const func = (value: number) => x as const;
      "#,
      r#"
new Promise(resolve => {});
new Foo(1, () => {});
      "#,
    ]);
  }

  #[test]
  fn explicit_function_return_type_invalid() {
    assert_lint_err::<ExplicitFunctionReturnType>("function foo() { }", 0);
    assert_lint_err_on_line_n::<ExplicitFunctionReturnType>(
      r#"
function a() {
  function b() {}
}
      "#,
      vec![(2, 0), (3, 2)],
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
function test(a: number, b: number) {
  return;
}
      "#,
      2,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
function test() {
  return;
}
      "#,
      2,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
var fn = function () {
  return 1;
};
      "#,
      2,
      9,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
var arrowFn = () => 'test';
      "#,
      2,
      14,
    );
    assert_lint_err_on_line_n::<ExplicitFunctionReturnType>(
      r#"
class Test {
  constructor() {} // OK
  get prop() {
    return 1;
  }
  set prop() {} // OK
  method() {
    return;
  }
  arrow = () => 'arrow';
  private method() {
    return;
  }
}
      "#,
      vec![(4, 2), (8, 2), (11, 10), (12, 2)],
    );
    assert_lint_err::<ExplicitFunctionReturnType>(
      r#"var arrowFn = () => 'test';"#,
      14,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
var funcExpr = function () {
  return 'test';
};
      "#,
      2,
      15,
    );
    assert_lint_err::<ExplicitFunctionReturnType>(r#"() => () => {};"#, 0);
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"() => function () {};"#,
      0,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
() => {
  return () => {};
};
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
() => {
  return function () {};
};
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
function fn() {
  return () => {};
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
function fn() {
  return function () {};
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
function FunctionDeclaration() {
  return function FunctionExpression_Within_FunctionDeclaration() {
    return function FunctionExpression_Within_FunctionExpression() {
      return () => {
        // ArrowFunctionExpression_Within_FunctionExpression
        return () =>
          // ArrowFunctionExpression_Within_ArrowFunctionExpression
          () => 1; // ArrowFunctionExpression_Within_ArrowFunctionExpression_WithNoBody
      };
    };
  };
}
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
() => () => {
  return () => {
    return;
  };
};
      "#,
      0,
      0,
    );
    assert_lint_err_on_line::<ExplicitFunctionReturnType>(
      r#"
const func = (value: number) => ({ type: 'X', value } as any);
const func = (value: number) => ({ type: 'X', value } as Action);
      "#,
      0,
      0,
    );
  }
}
