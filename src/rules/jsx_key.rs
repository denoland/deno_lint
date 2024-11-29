// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{
  ArrayLit, BlockStmtOrExpr, CallExpr, Callee, Expr, JSXAttrName,
  JSXAttrOrSpread, MemberProp, OptCall, OptChainBase, Stmt,
};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct JSXKey;

const CODE: &str = "jsx-key";

impl LintRule for JSXKey {
  fn tags(&self) -> &'static [&'static str] {
    &["react", "jsx"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    JSXKeyHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/jsx_key.md")
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

impl Handler for JSXKeyHandler {
  fn array_lit(&mut self, node: &ArrayLit, ctx: &mut Context) {
    for elem in node.elems.iter().flatten() {
      check_expr(ctx, &elem.expr);
    }
  }

  fn opt_call(&mut self, node: &OptCall, ctx: &mut Context) {
    if is_map_member(&node.callee) {
      if let Some(callback) = node.args.first() {
        match callback.expr {
          Expr::Arrow(arrow_fn) => match arrow_fn.body {
            BlockStmtOrExpr::BlockStmt(stmt) => {
              check_stmt(ctx, &Stmt::Block(stmt))
            }
            BlockStmtOrExpr::Expr(expr) => check_expr(ctx, &expr),
          },
          Expr::Fn(fn_expr) => {
            if let Some(body) = fn_expr.function.body {
              check_stmt(ctx, &Stmt::Block(body))
            }
          }
          _ => {}
        }
      }
    }
  }

  fn call_expr(&mut self, node: &CallExpr, ctx: &mut Context) {
    if let Callee::Expr(callee) = node.callee {
      if is_map_member(&callee) {
        if let Some(callback) = node.args.first() {
          check_callback(ctx, &callback.expr)
        }
      } else if let Expr::Member(member) = callee {
        if let Expr::Ident(id) = member.obj {
          if id.sym() == "Array" {
            if let MemberProp::Ident(member_id) = member.prop {
              if member_id.sym() == "from" {
                if let Some(el) = node.args.get(1) {
                  check_callback(ctx, &el.expr);
                }
              }
            }
          }
        }
      }
    }
  }
}

fn is_map_member(expr: &Expr) -> bool {
  match expr {
    Expr::Member(member) => {
      if let MemberProp::Ident(member_id) = member.prop {
        if member_id.sym() == "map" {
          return true;
        }
      }

      false
    }
    Expr::OptChain(opt_member) => {
      if let OptChainBase::Member(member) = opt_member.base {
        if let MemberProp::Ident(member_id) = member.prop {
          if member_id.sym() == "map" {
            return true;
          }
        }
      }

      false
    }
    _ => false,
  }
}

fn check_callback(ctx: &mut Context, expr: &Expr) {
  match expr {
    Expr::Arrow(arrow_fn) => match arrow_fn.body {
      BlockStmtOrExpr::BlockStmt(stmt) => check_stmt(ctx, &Stmt::Block(stmt)),
      BlockStmtOrExpr::Expr(expr) => check_expr(ctx, &expr),
    },
    Expr::Fn(fn_expr) => {
      if let Some(body) = fn_expr.function.body {
        check_stmt(ctx, &Stmt::Block(body))
      }
    }
    _ => {}
  }
}

fn check_stmt(ctx: &mut Context, stmt: &Stmt) {
  match stmt {
    Stmt::Return(return_stmt) => {
      if let Some(arg) = return_stmt.arg {
        check_expr(ctx, &arg);
      }
    }
    Stmt::If(if_stmt) => {
      check_stmt(ctx, &if_stmt.cons);
      if let Some(alt) = if_stmt.alt {
        check_stmt(ctx, &alt);
      }
    }
    Stmt::Block(block_stmt) => {
      for stmt in block_stmt.stmts {
        check_stmt(ctx, stmt);
      }
    }
    _ => {}
  }
}

fn check_expr(ctx: &mut Context, expr: &Expr) {
  match expr {
    Expr::JSXElement(jsx_el) => {
      if !has_key_jsx_attr(jsx_el.opening.attrs) {
        ctx.add_diagnostic_with_hint(
          jsx_el.opening.range(),
          CODE,
          DiagnosticKind::MissingKey.message(),
          DiagnosticKind::MissingKey.hint(),
        );
      }
    }
    Expr::JSXFragment(jsx_frag) => {
      ctx.add_diagnostic_with_hint(
        jsx_frag.opening.range(),
        CODE,
        DiagnosticKind::MissingFragKey.message(),
        DiagnosticKind::MissingFragKey.hint(),
      );
    }
    Expr::Cond(cond_epxr) => {
      check_expr(ctx, &cond_epxr.cons);
      check_expr(ctx, &cond_epxr.alt);
    }
    Expr::Bin(bin_expr) => {
      check_expr(ctx, &bin_expr.left);
      check_expr(ctx, &bin_expr.right);
    }
    _ => {}
  }
}

fn has_key_jsx_attr(attrs: &[JSXAttrOrSpread]) -> bool {
  for attr in attrs {
    if let JSXAttrOrSpread::JSXAttr(attr) = attr {
      if let JSXAttrName::Ident(id) = attr.name {
        if id.sym() == "key" {
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
