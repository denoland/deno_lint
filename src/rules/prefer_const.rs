// Copyright 2020 the Deno authors. All rights reserved. MIT license.
// TODO(magurotuna): remove next line
#![allow(unused)]
use super::Context;
use super::LintRule;
use std::sync::Arc;
use swc_ecmascript::ast::{
  ArrayPat, Expr, Ident, Lit, ObjectPat, Pat, TsAsExpr, TsLit, TsType,
  TsTypeAssertion, VarDecl,
};
use swc_ecmascript::visit::{Node, Visit, VisitWith};

pub struct PreferConst;

impl LintRule for PreferConst {
  fn new() -> Box<Self> {
    Box::new(PreferConst)
  }

  fn code(&self) -> &'static str {
    "prefer-const"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = PreferConstVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct PreferConstVisitor {
  context: Arc<Context>,
}

impl PreferConstVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn report(&self, ident: &Ident) {
    self.context.add_diagnostic(
      ident.span,
      "prefer-const",
      &format!(
        "'{}' is never reassigned. Use 'const' instead",
        ident.as_ref()
      ),
    );
  }
}

impl Visit for PreferConstVisitor {
  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    var_decl.visit_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn prefer_const_valid() {
    assert_lint_ok_n::<PreferConst>(vec![
      r#"var x = 0;"#,
      r#"let x;"#,
      r#"let x; { x = 0; } foo(x);"#,
      r#"let x = 0; x = 1;"#,
      r#"const x = 0;"#,
      r#"for (let i = 0, end = 10; i < end; ++i) {}"#,
      r#"for (let i in [1,2,3]) { i = 0; }"#,
      r#"for (let x of [1,2,3]) { x = 0; }"#,
      r#"(function() { var x = 0; })();"#,
      r#"(function() { let x; })();"#,
      r#"(function() { let x; { x = 0; } foo(x); })();"#,
      r#"(function() { let x = 0; x = 1; })();"#,
      r#"(function() { const x = 0; })();"#,
      r#"(function() { for (let i = 0, end = 10; i < end; ++i) {} })();"#,
      r#"(function() { for (let i in [1,2,3]) { i = 0; } })();"#,
      r#"(function() { for (let x of [1,2,3]) { x = 0; } })();"#,
      r#"(function(x = 0) { })();"#,
      r#"let a; while (a = foo());"#,
      r#"let a; do {} while (a = foo());"#,
      r#"let a; for (; a = foo(); );"#,
      r#"let a; for (;; ++a);"#,
      r#"let a; for (const {b = ++a} in foo());"#,
      r#"let a; for (const {b = ++a} of foo());"#,
      r#"let a; for (const x of [1,2,3]) { if (a) {} a = foo(); }"#,
      r#"let a; for (const x of [1,2,3]) { a = a || foo(); bar(a); }"#,
      r#"let a; for (const x of [1,2,3]) { foo(++a); }"#,
      r#"let a; function foo() { if (a) {} a = bar(); }"#,
      r#"let a; function foo() { a = a || bar(); baz(a); }"#,
      r#"let a; function foo() { bar(++a); }"#,
      r#"
    let id;
    function foo() {
        if (typeof id !== 'undefined') {
            return;
        }
        id = setInterval(() => {}, 250);
    }
    foo();
  "#,
      r#"/*exported a*/ let a; function init() { a = foo(); }"#,
      r#"/*exported a*/ let a = 1"#,
      r#"let a; if (true) a = 0; foo(a);"#,
      r#"
        (function (a) {
            let b;
            ({ a, b } = obj);
        })();
        "#,
      r#"
        (function (a) {
            let b;
            ([ a, b ] = obj);
        })();
        "#,
      r#"var a; { var b; ({ a, b } = obj); }"#,
      r#"let a; { let b; ({ a, b } = obj); }"#,
      r#"var a; { var b; ([ a, b ] = obj); }"#,
      r#"let a; { let b; ([ a, b ] = obj); }"#,
      r#"let x; { x = 0; foo(x); }"#,
      r#"(function() { let x; { x = 0; foo(x); } })();"#,
      r#"let x; for (const a of [1,2,3]) { x = foo(); bar(x); }"#,
      r#"(function() { let x; for (const a of [1,2,3]) { x = foo(); bar(x); } })();"#,
      r#"let x; for (x of array) { x; }"#,
      r#"let predicate; [typeNode.returnType, predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType, ...predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType,, predicate] = foo();"#,
      r#"let predicate; [typeNode.returnType=5, predicate] = foo();"#,
      r#"let predicate; [[typeNode.returnType=5], predicate] = foo();"#,
      r#"let predicate; [[typeNode.returnType, predicate]] = foo();"#,
      r#"let predicate; [typeNode.returnType, [predicate]] = foo();"#,
      r#"let predicate; [, [typeNode.returnType, predicate]] = foo();"#,
      r#"let predicate; [, {foo:typeNode.returnType, predicate}] = foo();"#,
      r#"let predicate; [, {foo:typeNode.returnType, ...predicate}] = foo();"#,
      r#"let a; const b = {}; ({ a, c: b.c } = func());"#,
      r#"const x = [1,2]; let y; [,y] = x; y = 0;"#,
      r#"const x = [1,2,3]; let y, z; [y,,z] = x; y = 0; z = 0;"#,
    ]);
  }

  #[test]
  fn prefer_const_invalid() {
    assert_lint_err::<PreferConst>(r#"let x = 1; foo(x);"#, 0);
    assert_lint_err::<PreferConst>(r#"for (let i in [1,2,3]) { foo(i); }"#, 0);
    assert_lint_err::<PreferConst>(r#"for (let x of [1,2,3]) { foo(x); }"#, 0);
    assert_lint_err::<PreferConst>(r#"let [x = -1, y] = [1,2]; y = 0;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let {a: x = -1, b: y} = {a:1,b:2}; y = 0;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let x = 1; foo(x); })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { for (let i in [1,2,3]) { foo(i); } })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { for (let x of [1,2,3]) { foo(x); } })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let [x = -1, y] = [1,2]; y = 0; })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let f = (function() { let g = x; })(); f = 1;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"(function() { let {a: x = -1, b: y} = {a:1,b:2}; y = 0; })();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let x = 0; { let x = 1; foo(x); } x = 0;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"for (let i = 0; i < 10; ++i) { let x = 1; foo(x); }"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"for (let i in [1,2,3]) { let x = 1; foo(x); }"#,
      0,
    );
    assert_lint_err_on_line::<PreferConst>(
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       a = 1;
   }
};
    "#,
      0,
      0,
    );
    assert_lint_err_on_line::<PreferConst>(
      r#"
var foo = function() {
    for (const b of c) {
       let a;
       ({a} = 1);
   }
};
    "#,
      0,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let x; x = 0;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"switch (a) { case 0: let x; x = 0; }"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"(function() { let x; x = 1; })();"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let {a = 0, b} = obj; b = 0; foo(a, b);"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let {a: {b, c}} = {a: {b: 1, c: 2}}; b = 3;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let a, b; ({a = 0, b} = obj); b = 0; foo(a, b);"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let [a] = [1]"#, 0);
    assert_lint_err::<PreferConst>(r#"let {a} = obj"#, 0);
    assert_lint_err::<PreferConst>(r#"let {a = 0, b} = obj, c = a; b = a;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let { name, ...otherStuff } = obj; otherStuff = {};"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let { name, ...otherStuff } = obj; otherStuff = {};"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let x; function foo() { bar(x); } x = 0;"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"/*eslint use-x:error*/ let x = 1"#, 0);
    assert_lint_err::<PreferConst>(
      r#"/*eslint use-x:error*/ { let x = 1 }"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let { foo, bar } = baz;"#, 0);
    assert_lint_err::<PreferConst>(r#"const x = [1,2]; let [,y] = x;"#, 0);
    assert_lint_err::<PreferConst>(r#"const x = [1,2,3]; let [y,,z] = x;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, predicate}] = foo();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, predicate}, ...bar ] = foo();"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let predicate; [, {foo:returnType, ...predicate} ] = foo();"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let x = 'x', y = 'y';"#, 0);
    assert_lint_err::<PreferConst>(r#"let x = 'x', y = 'y'; x = 1"#, 0);
    assert_lint_err::<PreferConst>(r#"let x = 1, y = 'y'; let z = 1;"#, 0);
    assert_lint_err::<PreferConst>(
      r#"let { a, b, c} = obj; let { x, y, z} = anotherObj; x = 2;"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let x = 'x', y = 'y'; function someFunc() { let a = 1, b = 2; foo(a, b) }"#,
      0,
    );
    assert_lint_err::<PreferConst>(
      r#"let someFunc = () => { let a = 1, b = 2; foo(a, b) }"#,
      0,
    );
    assert_lint_err::<PreferConst>(r#"let {a, b} = c, d;"#, 0);
    assert_lint_err::<PreferConst>(r#"let {a, b, c} = {}, e, f;"#, 0);
    assert_lint_err_on_line::<PreferConst>(
      r#"
function a() {
  let foo = 0,
  bar = 1;
  foo = 1;
}
function b() {
  let foo = 0,
  bar = 2;
  foo = 2;
}
    "#,
      0,
      0,
    );
  }
}
