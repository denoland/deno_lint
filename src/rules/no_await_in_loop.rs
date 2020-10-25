// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::Span;
use swc_ecmascript::ast::{
  ArrowExpr, AwaitExpr, DoWhileStmt, ForInStmt, ForOfStmt, ForStmt, Function,
  WhileStmt,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit};

pub struct NoAwaitInLoop;

impl LintRule for NoAwaitInLoop {
  fn new() -> Box<Self> {
    Box::new(NoAwaitInLoop)
  }

  fn code(&self) -> &'static str {
    "no-await-in-loop"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoAwaitInLoopVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Requires `await` is not used in a for loop body

Async and await are used in Javascript to provide parallel execution.  If each
element in the for loop is waited upon using `await`, then this negates the
benefits of using async/await as no more elements in the loop can be processed
until the current element finishes.  

A common solution is to refactor the code to run the loop body asynchronously and
capture the promises generated.  After the loop finishes you can then await all
the promises at once.

### Invalid:
```javascript
async function doSomething(items) {
  const results = [];
  for (const item of items) {
    // Each item in the array blocks on the previous one finishing
    results.push(await someAsyncProcessing(item));
  }
  return processResults(results);
}
```
    
### Valid:
```javascript
async function doSomething(items) {
  const results = [];
  for (const item of items) {
    // Kick off all item processing asynchronously...
    results.push(someAsyncProcessing(item));
  }
  // ...and then await their completion after the loop
  return processResults(await Promise.all(results));
}
```
"#
  }
}

struct NoAwaitInLoopVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoAwaitInLoopVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span) {
    self.context.add_diagnostic_with_hint(
      span,
      "no-await-in-loop",
      "Unexpected `await` inside a loop.",
      "Remove `await` in loop body, store all promises generated and then `await Promise.all(storedPromises)` after the loop"
    );
  }
}

impl<'c> Visit for NoAwaitInLoopVisitor<'c> {
  noop_visit_type!();

  fn visit_function(&mut self, func: &Function, parent: &dyn Node) {
    let mut func_visitor = FunctionVisitor::new(self, func.is_async);
    func_visitor.visit_function(func, parent);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, parent: &dyn Node) {
    let mut func_visitor = FunctionVisitor::new(self, arrow_expr.is_async);
    func_visitor.visit_arrow_expr(arrow_expr, parent);
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, parent: &dyn Node) {
    let mut loop_visitor = LoopVisitor::new(self);
    loop_visitor.visit_for_stmt(for_stmt, parent);
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, parent: &dyn Node) {
    if for_of_stmt.await_token.is_some() {
      let mut func_visitor = FunctionVisitor::new(self, true);
      func_visitor.visit_for_of_stmt(for_of_stmt, parent);
    } else {
      let mut loop_visitor = LoopVisitor::new(self);
      loop_visitor.visit_for_of_stmt(for_of_stmt, parent);
    }
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, parent: &dyn Node) {
    let mut loop_visitor = LoopVisitor::new(self);
    loop_visitor.visit_for_in_stmt(for_in_stmt, parent);
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, parent: &dyn Node) {
    let mut loop_visitor = LoopVisitor::new(self);
    loop_visitor.visit_while_stmt(while_stmt, parent);
  }

  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &DoWhileStmt,
    parent: &dyn Node,
  ) {
    let mut loop_visitor = LoopVisitor::new(self);
    loop_visitor.visit_do_while_stmt(do_while_stmt, parent);
  }
}

struct LoopVisitor<'a, 'b> {
  root_visitor: &'b mut NoAwaitInLoopVisitor<'a>,
}

impl<'a, 'b> LoopVisitor<'a, 'b> {
  fn new(root_visitor: &'b mut NoAwaitInLoopVisitor<'a>) -> Self {
    Self { root_visitor }
  }
}

impl<'a, 'b> Visit for LoopVisitor<'a, 'b> {
  fn visit_function(&mut self, func: &Function, parent: &dyn Node) {
    let mut func_visitor = if func.is_async {
      FunctionVisitor::new(self.root_visitor, true)
    } else {
      FunctionVisitor::new(self.root_visitor, false)
    };
    func_visitor.visit_function(func, parent);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, parent: &dyn Node) {
    let mut func_visitor = if arrow_expr.is_async {
      FunctionVisitor::new(self.root_visitor, true)
    } else {
      FunctionVisitor::new(self.root_visitor, false)
    };
    func_visitor.visit_arrow_expr(arrow_expr, parent);
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, parent: &dyn Node) {
    if let Some(test) = for_stmt.test.as_ref() {
      self.visit_expr(&**test, parent);
    }
    if let Some(update) = for_stmt.update.as_ref() {
      self.visit_expr(&**update, parent);
    }
    let body = &*for_stmt.body;
    self.visit_stmt(body, parent);
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, parent: &dyn Node) {
    let body = &*for_of_stmt.body;
    self.visit_stmt(body, parent);
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, parent: &dyn Node) {
    let body = &*for_in_stmt.body;
    self.visit_stmt(body, parent);
  }

  fn visit_await_expr(&mut self, await_expr: &AwaitExpr, parent: &dyn Node) {
    self.root_visitor.add_diagnostic(await_expr.span);
    swc_ecmascript::visit::visit_await_expr(self, await_expr, parent);
  }
}

struct FunctionVisitor<'a, 'b> {
  root_visitor: &'b mut NoAwaitInLoopVisitor<'a>,
  is_async: bool,
}

impl<'a, 'b> FunctionVisitor<'a, 'b> {
  fn new(
    root_visitor: &'b mut NoAwaitInLoopVisitor<'a>,
    is_async: bool,
  ) -> Self {
    Self {
      root_visitor,
      is_async,
    }
  }
}

impl<'a, 'b> Visit for FunctionVisitor<'a, 'b> {
  fn visit_function(&mut self, func: &Function, parent: &dyn Node) {
    let mut func_visitor =
      FunctionVisitor::new(self.root_visitor, func.is_async);
    swc_ecmascript::visit::visit_function(&mut func_visitor, func, parent);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, parent: &dyn Node) {
    let mut func_visitor =
      FunctionVisitor::new(self.root_visitor, arrow_expr.is_async);
    swc_ecmascript::visit::visit_arrow_expr(
      &mut func_visitor,
      arrow_expr,
      parent,
    );
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt, parent: &dyn Node) {
    if self.is_async {
      let mut loop_visitor = LoopVisitor::new(self.root_visitor);
      loop_visitor.visit_for_stmt(for_stmt, parent);
    } else {
      swc_ecmascript::visit::visit_for_stmt(self, for_stmt, parent);
    }
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, parent: &dyn Node) {
    if self.is_async && for_of_stmt.await_token.is_none() {
      let mut loop_visitor = LoopVisitor::new(self.root_visitor);
      loop_visitor.visit_for_of_stmt(for_of_stmt, parent);
    } else {
      swc_ecmascript::visit::visit_for_of_stmt(self, for_of_stmt, parent);
    }
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, parent: &dyn Node) {
    if self.is_async {
      let mut loop_visitor = LoopVisitor::new(self.root_visitor);
      loop_visitor.visit_for_in_stmt(for_in_stmt, parent);
    } else {
      swc_ecmascript::visit::visit_for_in_stmt(self, for_in_stmt, parent);
    }
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, parent: &dyn Node) {
    if self.is_async {
      let mut loop_visitor = LoopVisitor::new(self.root_visitor);
      loop_visitor.visit_while_stmt(while_stmt, parent);
    } else {
      swc_ecmascript::visit::visit_while_stmt(self, while_stmt, parent);
    }
  }

  fn visit_do_while_stmt(
    &mut self,
    do_while_stmt: &DoWhileStmt,
    parent: &dyn Node,
  ) {
    if self.is_async {
      let mut loop_visitor = LoopVisitor::new(self.root_visitor);
      loop_visitor.visit_do_while_stmt(do_while_stmt, parent);
    } else {
      swc_ecmascript::visit::visit_do_while_stmt(self, do_while_stmt, parent);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_await_in_loop_valid() {
    assert_lint_ok! {
      NoAwaitInLoop,
      r#"
async function foo(things) {
  const results = [];
  for (const thing of things) {
    results.push(bar(thing));
  }
  return baz(await Promise.all(results));
}
      "#,
      r#"
async function foo(things) {
  for (const thing of things) {
    const a = async () => await bar(thing);
  }
}
      "#,
      r#"
async function foo(things) {
  for (const thing of things) {
    async function a() {
      return await bar(42);
    }
  }
}
      "#,
      r#"
async function foo(things) {
  for (const thing of things) {
    const a = async function() {
      return await bar(42);
    }
  }
}
      "#,
      r#"
async function foo(things) {
  for await (const thing of things) {
    console.log(await bar(thing));
  }
}
      "#,
      r#"
async function foo(things) {
  for await (const thing of await things) {
    console.log(await bar(thing));
  }
}
      "#,
      r#"
async function foo() {
  for (let i = await bar(); i < n; i++) {
    baz(i);
  }
}
      "#,
      r#"
async function foo() {
  for (const thing of await things) {
    bar(thing);
  }
}
      "#,
      r#"
async function foo() {
  for (let thing in await things) {
    bar(thing);
  }
}
      "#,
      r#"
function foo() {
  async function bar() {
    for (const thing of await things) {}
  }
}
      "#,

      // toplevel await
      r#"
for (const thing of things) {
  const a = async () => await bar(thing);
}
      "#,
      r#"
for (const thing of things) {
  async function a() {
    return await bar(42);
  }
}
      "#,
      r#"
for (const thing of things) {
  const a = async function() {
    return await bar(42);
  }
}
      "#,
      r#"
for await (const thing of things) {
  console.log(await bar(thing));
}
      "#,
      r#"
for await (const thing of await things) {
  console.log(await bar(thing));
}
      "#,
      r#"
for (let i = await bar(); i < n; i++) {
  baz(i);
}
      "#,
      r#"
for (const thing of await things) {
  bar(thing);
}
      "#,
      r#"
for (let thing in await things) {
  bar(thing);
}
      "#,
    };
  }

  #[test]
  fn no_await_in_loop_invalid() {
    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
async function foo(things) {
  const results = [];
  for (const thing of things) {
    results.push(await bar(thing));
  }
  return baz(results);
}
      "#,
      5,
      17,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
for (const thing of things) {
  results.push(await foo(thing));
}
      "#,
      3,
      15,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
for (let i = 0; i < await foo(); i++) {
  bar();
}
      "#,
      2,
      20,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
for (let i = 0; i < 42; await foo(i)) {
  bar();
}
      "#,
      2,
      24,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
for (let i = 0; i < 42; i++) {
  await bar();
}
      "#,
      3,
      2,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
for (const thing in things) {
  await foo(thing);
}
      "#,
      3,
      2,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
while (await foo()) {
  bar();
}
      "#,
      2,
      7,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
while (true) {
  await foo();
}
      "#,
      3,
      2,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
while (true) {
  await foo();
}
      "#,
      3,
      2,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
do {
  foo();
} while (await bar());
      "#,
      4,
      9,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
do {
  await foo();
} while (true);
      "#,
      3,
      2,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
for await (const thing of things) {
  async function foo() {
    for (const one of them) {
      await bar(one);
    }
  }
  await baz();
}
      "#,
      5,
      6,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
function foo() {
  async function bar() {
    for (const thing of things) {
      await baz(thing);
    }
  }
}
      "#,
      5,
      6,
    );

    assert_lint_err_on_line::<NoAwaitInLoop>(
      r#"
async function foo() {
  for (const thing of things) {
    const xs = bar(thing);
    for (const x in xs) {
      await baz(x);
    }
  }
}
      "#,
      6,
      6,
    );
  }
}
