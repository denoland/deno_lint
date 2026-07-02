// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  ArrayExpression, ArrayExpressionElement, CallExpression, Expression,
  FunctionBody, JSXAttributeItem, JSXAttributeName, JSXOpeningElement, Program,
  Statement,
};

#[derive(Debug)]
pub struct JSXKey;

const CODE: &str = "jsx-key";

impl LintRule for JSXKey {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED, tags::REACT, tags::JSX]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = JSXKeyHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

enum DiagnosticKind {
  MissingKey,
  MissingFragKey,
}

impl DiagnosticKind {
  fn message(&self) -> &'static str {
    match *self {
      DiagnosticKind::MissingKey => "Missing 'key' prop for an element",
      DiagnosticKind::MissingFragKey => "Missing 'key' prop for a Fragment",
    }
  }

  fn hint(&self) -> &'static str {
    match *self {
      DiagnosticKind::MissingKey => "Add a 'key' prop",
      DiagnosticKind::MissingFragKey => {
        "Use the `<Fragment key=\"..\">` syntax instead"
      }
    }
  }
}

struct JSXKeyHandler;

impl Handler<'_> for JSXKeyHandler {
  fn array_expression(&mut self, node: &ArrayExpression, ctx: &mut Context) {
    let elems: Vec<_> = node
      .elements
      .iter()
      .filter_map(|elem| match elem {
        ArrayExpressionElement::Elision(_) => None,
        ArrayExpressionElement::SpreadElement(_) => None,
        other => Some(other.to_expression()),
      })
      .collect();

    if elems.iter().all(|expr| is_jsx_like(expr)) {
      for expr in elems {
        check_expr(ctx, expr);
      }
    }
  }
  fn call_expression(&mut self, node: &CallExpression, ctx: &mut Context) {
    if is_map_callee(&node.callee) {
      if let Some(callback) = node.arguments.first() {
        if let Some(expr) = callback.as_expression() {
          check_callback(ctx, expr);
        }
      }
    } else if let Some(member) = node.callee.as_member_expression() {
      if let Some("from") = member.static_property_name() {
        if let Expression::Identifier(id) = member.object() {
          if id.name.as_str() == "Array" {
            if let Some(el) = node.arguments.get(1) {
              if let Some(expr) = el.as_expression() {
                check_callback(ctx, expr);
              }
            }
          }
        }
      }
    }
  }
}

fn is_map_callee(expr: &Expression) -> bool {
  if let Some(member) = expr.as_member_expression() {
    return member.static_property_name() == Some("map");
  }
  false
}

fn check_callback(ctx: &mut Context, expr: &Expression) {
  match expr {
    Expression::ArrowFunctionExpression(arrow_fn) => {
      if arrow_fn.expression {
        // Single expression body
        if let Some(Statement::ExpressionStatement(expr_stmt)) =
          arrow_fn.body.statements.first()
        {
          check_expr(ctx, &expr_stmt.expression);
        }
      } else {
        check_function_body(ctx, &arrow_fn.body);
      }
    }
    Expression::FunctionExpression(fn_expr) => {
      if let Some(body) = &fn_expr.body {
        check_function_body(ctx, body);
      }
    }
    _ => {}
  }
}

fn check_function_body(ctx: &mut Context, body: &FunctionBody) {
  for stmt in &body.statements {
    check_stmt(ctx, stmt);
  }
}

fn check_stmt(ctx: &mut Context, stmt: &Statement) {
  match stmt {
    Statement::ReturnStatement(return_stmt) => {
      if let Some(arg) = &return_stmt.argument {
        check_expr(ctx, arg);
      }
    }
    Statement::IfStatement(if_stmt) => {
      check_stmt(ctx, &if_stmt.consequent);
      if let Some(alt) = &if_stmt.alternate {
        check_stmt(ctx, alt);
      }
    }
    Statement::BlockStatement(block_stmt) => {
      for stmt in &block_stmt.body {
        check_stmt(ctx, stmt);
      }
    }
    _ => {}
  }
}

fn is_jsx_like(expr: &Expression) -> bool {
  match expr {
    Expression::JSXElement(_) | Expression::JSXFragment(_) => true,
    Expression::ParenthesizedExpression(paren) => {
      is_jsx_like(&paren.expression)
    }
    Expression::ConditionalExpression(cond) => {
      is_jsx_like(&cond.consequent) || is_jsx_like(&cond.alternate)
    }
    Expression::LogicalExpression(logical) => {
      is_jsx_like(&logical.left) || is_jsx_like(&logical.right)
    }
    Expression::BinaryExpression(binary) => {
      is_jsx_like(&binary.left) || is_jsx_like(&binary.right)
    }
    _ => false,
  }
}

fn check_expr(ctx: &mut Context, expr: &Expression) {
  match expr {
    Expression::ParenthesizedExpression(paren) => {
      check_expr(ctx, &paren.expression);
    }
    Expression::JSXElement(jsx_el) => {
      if !has_key_jsx_attr(&jsx_el.opening_element) {
        ctx.add_diagnostic_with_hint(
          jsx_el.opening_element.span,
          CODE,
          DiagnosticKind::MissingKey.message(),
          DiagnosticKind::MissingKey.hint(),
        );
      }
    }
    Expression::JSXFragment(jsx_frag) => {
      ctx.add_diagnostic_with_hint(
        jsx_frag.opening_fragment.span,
        CODE,
        DiagnosticKind::MissingFragKey.message(),
        DiagnosticKind::MissingFragKey.hint(),
      );
    }
    Expression::ConditionalExpression(cond_expr) => {
      check_expr(ctx, &cond_expr.consequent);
      check_expr(ctx, &cond_expr.alternate);
    }
    Expression::BinaryExpression(bin_expr) => {
      check_expr(ctx, &bin_expr.left);
      check_expr(ctx, &bin_expr.right);
    }
    Expression::LogicalExpression(logical_expr) => {
      check_expr(ctx, &logical_expr.left);
      check_expr(ctx, &logical_expr.right);
    }
    _ => {}
  }
}

fn has_key_jsx_attr(opening: &JSXOpeningElement) -> bool {
  for attr in &opening.attributes {
    if let JSXAttributeItem::Attribute(attr) = attr {
      if let JSXAttributeName::Identifier(id) = &attr.name {
        if id.name.as_str() == "key" {
          return true;
        }
      }
    }
  }

  false
}

// most tests are taken from ESlint, commenting those
// requiring code path support
#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn jsx_key_valid() {
    assert_lint_ok! {
      JSXKey,
      filename: "file:///foo.jsx",
      "[1, 2, 3].map(x => {})",
      "<div />",
      r#"[<div key="1"/>, <div key="2" />]"#,
      r#"[label, <Foo />]"#,
      r#"[(<div key="1" />)]"#,
      r#"[1, 2, 3].map(function(x) { return <div key={x} /> })"#,
      r#"[1, 2, 3].map((x) => { return <div key={x} /> })"#,
      r#"[1, 2, 3].map((x) => <div key={x} />)"#,
      r#"[1, 2, 3].map((x) => foo && <div key={x} />)"#,
      r#"[1, 2, 3].map((x) => foo ? <div key={x} /> : <div key={x} />)"#,
      r#"[1, 2, 3]?.map((x) => <div key={x} />)"#,
      r#"Array.from([1, 2, 3], function(x) { return <div key={x} /> })"#,
      r#"Array.from([1, 2, 3], (x) => { return <div key={x} /> })"#,
      r#"Array.from([1, 2, 3], (x) => <div key={x} />)"#,
      r#"const Foo = () => {
        const a = [1, 2, 3];
        return (
          <div>
            {a.map(x => {
              if (a) {
                return <div key="a" />
              }

              return <div key="f" />
            })}
          </div>
        );
      }"#,
      r#"const Foo = () => {
        const a = [1, 2, 3];
        return (
          <div>
            {a.map(x => {
              if (a) return <div key="a" />
              else return <div key="a" />
            })}
          </div>
        );
      }"#,
      r#"const Foo = () => {
        const a = [1, 2, 3];
        return (
          <div>
            {a.map(x => <div key="a" />)}
          </div>
        );
      }"#

    };
  }

  #[test]
  fn jsx_key_invalid() {
    assert_lint_err! {
      JSXKey,
      filename: "file:///foo.jsx",
      "[<div />]": [
        {
          col: 1,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[<div key="foo" />, <div />]"#: [
        {
          col: 20,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[1, 2, 3].map(function(x) { return <div /> });"#: [
        {
          col: 35,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[1, 2, 3].map(x => <div />);"#: [
        {
          col: 19,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[1, 2, 3].map(x => { return <div />; });"#: [
        {
          col: 28,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[1, 2, 3].map(x => foo && <div />);"#: [
        {
          col: 26,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[1, 2, 3].map(x => foo ? <div key="foo" /> : <div />);"#: [
        {
          col: 45,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[1, 2, 3].map(x => foo ? <div /> : <div key="foo" />);"#: [
        {
          col: 25,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[1, 2, 3]?.map(x => <div />);"#: [
        {
          col: 20,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"[<></>];"#: [
        {
          col: 1,
          message: DiagnosticKind::MissingFragKey.message(),
          hint: DiagnosticKind::MissingFragKey.hint(),
        }
      ],
      r#"const a = [<></>];"#: [
        {
          col: 11,
          message: DiagnosticKind::MissingFragKey.message(),
          hint: DiagnosticKind::MissingFragKey.hint(),
        }
      ],
      r#"Array.from([1, 2, 3], function(x) { return <div /> });"#: [
        {
          col: 43,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"Array.from([1, 2, 3], (x) => { return <div /> });"#: [
        {
          col: 38,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"Array.from([1, 2, 3], (x) => <div />);"#: [
        {
          col: 29,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"const Foo = () => {
  const a = [1, 2, 3];
  return (
    <div>
      {a.map(x => {
        if (a) {
          return <div />
        }

        return <div key="f" />
      })}
    </div>
  );
}"#: [
        {
          line: 7,
          col: 17,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"const Foo = () => {
  const a = [1, 2, 3];
  return (
    <div>
      {a.map(x => {
        if (a) return <div />
        return <div key="f" />
      })}
    </div>
  );
}"#: [
        {
          line: 6,
          col: 22,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"const Foo = () => {
  const a = [1, 2, 3];
  return (
    <div>
      {a.map(x => {
        if (a) return <div key="f" />
        else return <div />;
        return <div key="f" />
      })}
    </div>
  );
}"#: [
        {
          line: 7,
          col: 20,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"const Foo = () => {
  const a = [1, 2, 3];
  return (
    <div>
      {a.map(x => {
        return <div />
      })}
    </div>
  );
}"#: [
        {
          line: 6,
          col: 15,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
      r#"const Foo = () => {
  const a = [1, 2, 3];
  return (
    <div>
      {a.map(x => <div />)}
    </div>
  );
}"#: [
        {
          line: 5,
          col: 18,
          message: DiagnosticKind::MissingKey.message(),
          hint: DiagnosticKind::MissingKey.hint(),
        }
      ],
    };
  }
}
