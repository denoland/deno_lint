// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use derive_more::Display;

#[derive(Debug)]
pub struct NoUnsafeFinally;

const CODE: &str = "no-unsafe-finally";
const HINT: &str = "Use of the control flow statements (`return`, `throw`, `break` and `continue`) in a `finally` block\
 will most likely lead to undesired behavior.";

#[derive(Display)]
enum NoUnsafeFinallyMessage {
  #[display(fmt = "Unsafe usage of break statement")]
  Break,
  #[display(fmt = "Unsafe usage of continue statement")]
  Continue,
  #[display(fmt = "Unsafe usage of return statement")]
  Return,
  #[display(fmt = "Unsafe usage of throw statement")]
  Throw,
}

impl LintRule for NoUnsafeFinally {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut scope_stack = Vec::new();
    walk_statements(&program.body, &mut scope_stack, context);
  }
}

// This rule uses manual AST walking instead of the Handler pattern because
// it needs to push/pop scope entries around specific children of try statements
// (finalizer vs block), which the Handler's automatic traversal doesn't support.

#[derive(Clone)]
enum ScopeEntry {
  Finally,
  Function,
  Loop,
  Switch,
  Label(String),
}

fn walk_statements(
  stmts: &[Statement],
  scope_stack: &mut Vec<ScopeEntry>,
  ctx: &mut Context,
) {
  for stmt in stmts {
    walk_statement(stmt, scope_stack, ctx);
  }
}

fn walk_statement(
  stmt: &Statement,
  scope_stack: &mut Vec<ScopeEntry>,
  ctx: &mut Context,
) {
  match stmt {
    Statement::BlockStatement(block) => {
      walk_statements(&block.body, scope_stack, ctx);
    }
    Statement::IfStatement(if_stmt) => {
      walk_statement(&if_stmt.consequent, scope_stack, ctx);
      if let Some(alt) = &if_stmt.alternate {
        walk_statement(alt, scope_stack, ctx);
      }
    }
    Statement::WhileStatement(w) => {
      scope_stack.push(ScopeEntry::Loop);
      walk_statement(&w.body, scope_stack, ctx);
      scope_stack.pop();
    }
    Statement::DoWhileStatement(d) => {
      scope_stack.push(ScopeEntry::Loop);
      walk_statement(&d.body, scope_stack, ctx);
      scope_stack.pop();
    }
    Statement::ForStatement(f) => {
      scope_stack.push(ScopeEntry::Loop);
      walk_statement(&f.body, scope_stack, ctx);
      scope_stack.pop();
    }
    Statement::ForInStatement(f) => {
      scope_stack.push(ScopeEntry::Loop);
      walk_statement(&f.body, scope_stack, ctx);
      scope_stack.pop();
    }
    Statement::ForOfStatement(f) => {
      scope_stack.push(ScopeEntry::Loop);
      walk_statement(&f.body, scope_stack, ctx);
      scope_stack.pop();
    }
    Statement::SwitchStatement(s) => {
      scope_stack.push(ScopeEntry::Switch);
      for case in &s.cases {
        walk_statements(&case.consequent, scope_stack, ctx);
      }
      scope_stack.pop();
    }
    Statement::LabeledStatement(l) => {
      scope_stack.push(ScopeEntry::Label(l.label.name.to_string()));
      walk_statement(&l.body, scope_stack, ctx);
      scope_stack.pop();
    }
    Statement::TryStatement(t) => {
      walk_statements(&t.block.body, scope_stack, ctx);
      if let Some(handler) = &t.handler {
        walk_statements(&handler.body.body, scope_stack, ctx);
      }
      if let Some(finalizer) = &t.finalizer {
        scope_stack.push(ScopeEntry::Finally);
        walk_statements(&finalizer.body, scope_stack, ctx);
        scope_stack.pop();
      }
    }
    Statement::BreakStatement(n) => {
      let label = n.label.as_ref().map(|l| l.name.as_str());
      if is_break_unsafe(scope_stack, label) {
        ctx.add_diagnostic_with_hint(
          n.span,
          CODE,
          NoUnsafeFinallyMessage::Break,
          HINT,
        );
      }
    }
    Statement::ContinueStatement(n) => {
      let label = n.label.as_ref().map(|l| l.name.as_str());
      if is_continue_unsafe(scope_stack, label) {
        ctx.add_diagnostic_with_hint(
          n.span,
          CODE,
          NoUnsafeFinallyMessage::Continue,
          HINT,
        );
      }
    }
    Statement::ReturnStatement(n) => {
      if is_return_or_throw_unsafe(scope_stack) {
        ctx.add_diagnostic_with_hint(
          n.span,
          CODE,
          NoUnsafeFinallyMessage::Return,
          HINT,
        );
      }
    }
    Statement::ThrowStatement(n) => {
      if is_return_or_throw_unsafe(scope_stack) {
        ctx.add_diagnostic_with_hint(
          n.span,
          CODE,
          NoUnsafeFinallyMessage::Throw,
          HINT,
        );
      }
    }
    // Walk into expressions that may contain functions/classes with statements
    Statement::ExpressionStatement(expr_stmt) => {
      walk_expression(&expr_stmt.expression, scope_stack, ctx);
    }
    Statement::VariableDeclaration(var_decl) => {
      for decl in &var_decl.declarations {
        if let Some(init) = &decl.init {
          walk_expression(init, scope_stack, ctx);
        }
      }
    }
    _ => {
      // Walk into function/class declarations
      if let Some(decl) = stmt.as_declaration() {
        match decl {
          Declaration::FunctionDeclaration(f) => {
            scope_stack.push(ScopeEntry::Function);
            if let Some(body) = &f.body {
              walk_statements(&body.statements, scope_stack, ctx);
            }
            scope_stack.pop();
          }
          Declaration::ClassDeclaration(c) => {
            walk_class_body(&c.body, scope_stack, ctx);
          }
          _ => {}
        }
      }
    }
  }
}

fn walk_expression(
  expr: &Expression,
  scope_stack: &mut Vec<ScopeEntry>,
  ctx: &mut Context,
) {
  match expr {
    Expression::FunctionExpression(f) => {
      scope_stack.push(ScopeEntry::Function);
      if let Some(body) = &f.body {
        walk_statements(&body.statements, scope_stack, ctx);
      }
      scope_stack.pop();
    }
    Expression::ArrowFunctionExpression(a) => {
      scope_stack.push(ScopeEntry::Function);
      walk_statements(&a.body.statements, scope_stack, ctx);
      scope_stack.pop();
    }
    Expression::ClassExpression(c) => {
      walk_class_body(&c.body, scope_stack, ctx);
    }
    _ => {}
  }
}

fn walk_class_body(
  body: &ClassBody,
  scope_stack: &mut Vec<ScopeEntry>,
  ctx: &mut Context,
) {
  for element in &body.body {
    match element {
      ClassElement::MethodDefinition(m) => {
        scope_stack.push(ScopeEntry::Function);
        if let Some(body) = &m.value.body {
          walk_statements(&body.statements, scope_stack, ctx);
        }
        scope_stack.pop();
      }
      ClassElement::PropertyDefinition(p) => {
        if let Some(value) = &p.value {
          walk_expression(value, scope_stack, ctx);
        }
      }
      ClassElement::StaticBlock(s) => {
        walk_statements(&s.body, scope_stack, ctx);
      }
      _ => {}
    }
  }
}

fn is_break_unsafe(scope_stack: &[ScopeEntry], label: Option<&str>) -> bool {
  for entry in scope_stack.iter().rev() {
    match entry {
      ScopeEntry::Finally => return true,
      ScopeEntry::Function => return false,
      ScopeEntry::Switch if label.is_none() => return false,
      ScopeEntry::Loop if label.is_none() => return false,
      ScopeEntry::Label(name) => {
        if let Some(target) = label {
          if name == target {
            return false;
          }
        }
      }
      _ => {}
    }
  }
  false
}

fn is_continue_unsafe(
  scope_stack: &[ScopeEntry],
  label: Option<&str>,
) -> bool {
  for entry in scope_stack.iter().rev() {
    match entry {
      ScopeEntry::Finally => return true,
      ScopeEntry::Function => return false,
      ScopeEntry::Loop if label.is_none() => return false,
      ScopeEntry::Label(name) => {
        if let Some(target) = label {
          if name == target {
            return false;
          }
        }
      }
      _ => {}
    }
  }
  false
}

fn is_return_or_throw_unsafe(scope_stack: &[ScopeEntry]) -> bool {
  for entry in scope_stack.iter().rev() {
    match entry {
      ScopeEntry::Finally => return true,
      ScopeEntry::Function => return false,
      _ => {}
    }
  }
  false
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_unsafe_finally_valid() {
    assert_lint_ok! {
      NoUnsafeFinally,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    console.log("hola!");
  }
};
     "#,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    let a = function() {
      return "hola!";
    }
  }
};
     "#,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    function bar() {
      return "hola!";
    }
  }
};
     "#,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    const f = (x) => {
      return x + 1;
    };
  }
};
     "#,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    class Foo {
      method(x: number): number {
        return x * 2;
      }
    }
  }
};
     "#,
      r#"
let foo = function(a) {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    switch(a) {
      case 1: {
        console.log("hola!")
        break;
      }
    }
  }
};
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  while (true) break;
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  while (true) continue;
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  do {
    break;
  } while (true)
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  do {
    continue;
  } while (true)
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  label: while (true) {
    if (x) break label;
    else continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (let i = 0; i < 100; i++) {
    break;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (let i = 0; i < 100; i++) {
    continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x of xs) {
    continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x of xs) {
    break;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x in xs) {
    continue;
  }
}
      "#,
      r#"
try {
  throw 42;
} catch (err) {
  console.log('hi');
} finally {
  for (const x in xs) {
    break;
  }
}
      "#,
      r#"
      "#,
      r#"
      "#,
    };
  }

  #[test]
  fn no_unsafe_finally_invalid() {
    assert_lint_err! {
      NoUnsafeFinally,
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    break;
  }
};
     "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    continue;
  }
};
     "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Continue,
          hint: HINT,
        }
      ],
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    return 3;
  }
};
          "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        }
      ],
      r#"
let foo = function() {
  try {
    return 1;
  } catch(err) {
    return 2;
  } finally {
    throw new Error;
  }
};
     "#: [
        {
          line: 8,
          col: 4,
          message: NoUnsafeFinallyMessage::Throw,
          hint: HINT,
        }
      ],
      r#"
try {}
finally {
  try {}
  finally {
    throw new Error;
  }
}
     "#: [
        {
          line: 6,
          col: 4,
          message: NoUnsafeFinallyMessage::Throw,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  try {}
  finally {
    if (x) {
      return 0;
    } else {
      return 1;
    }
  }
}
     "#: [
        {
          line: 6,
          col: 6,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        },
        {
          line: 8,
          col: 6,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        },
      ],
      r#"
function foo() {
  try {}
  finally {
    return () => {
      return 0;
    };
  }
}
     "#: [
        {
          line: 5,
          col: 4,
          message: NoUnsafeFinallyMessage::Return,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  label: try {
    return 0;
  } finally {
    break label;
  }
}
     "#: [
        {
          line: 6,
          col: 4,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  while (x) {
    try {}
    finally {
      break;
    }
  }
}
     "#: [
        {
          line: 6,
          col: 6,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  while (x) {
    try {}
    finally {
      continue;
    }
  }
}
     "#: [
        {
          line: 6,
          col: 6,
          message: NoUnsafeFinallyMessage::Continue,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  switch (x) {
    case 0:
      try {}
      finally {
        break;
      }
  }
}
     "#: [
        {
          line: 7,
          col: 8,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  a: while (x) {
    try {}
    finally {
      switch (y) {
        case 0:
          break a;
      }
    }
  }
}
     "#: [
        {
          line: 8,
          col: 10,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  while (x) {
    try {}
    finally {
      switch (y) {
        case 0:
          continue;
      }
    }
  }
}
     "#: [
        {
          line: 8,
          col: 10,
          message: NoUnsafeFinallyMessage::Continue,
          hint: HINT,
        }
      ],
      r#"
function foo() {
  a: switch (x) {
    case 0:
      try {}
      finally {
        switch (y) {
          case 1:
            break a;
        }
      }
  }
}
     "#: [
        {
          line: 9,
          col: 12,
          message: NoUnsafeFinallyMessage::Break,
          hint: HINT,
        }
      ],
    };
  }
}
