// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, Node, Pat};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct RulesOfHooks;

const CODE: &str = "rules-of-hooks";

impl LintRule for RulesOfHooks {
  fn tags(&self) -> &'static [&'static str] {
    &["react", "jsx", "fresh"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    let mut handler = RulesOfHooksHandler::new();
    handler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/rules_of_hooks.md")
  }
}

enum DiagnosticKind {
  OutsideComponent,
  Conditionally,
  Loop,
  TryCatch,
}

impl DiagnosticKind {
  fn message(&self) -> &'static str {
    match *self {
      DiagnosticKind::OutsideComponent => {
        "Hook function called outside of a Component"
      }
      DiagnosticKind::Conditionally => "Hook called conditionally",
      DiagnosticKind::Loop => "Hook called inside a loop",
      DiagnosticKind::TryCatch => "Hook called inside a try/catch-statement",
    }
  }

  fn hint(&self) -> &'static str {
    match *self {
      DiagnosticKind::OutsideComponent => {
        "Ensure that hooks are only called inside Components or custom hook functions"
      }
      DiagnosticKind::Conditionally => "Move the hook call before you call any conditions. All hooks must be invoked on every render call",
      DiagnosticKind::Loop => "Move the hook call before you call any loops or conditions. All hooks must be invoked on every render call",
      DiagnosticKind::TryCatch => "Move the hook call before you call any loops, conditions or try/catch statements. All hooks must be invoked on every render call",
    }
  }
}

#[derive(Debug)]
enum ParentKind {
  Fn((String, usize)),
  Bin,
  Cond,
  ExportDefault,
  Loop,
  TryCatch,
  Var(String),
  Unknown,
}

struct RulesOfHooksHandler {
  parent_kind: Vec<ParentKind>,
}

impl RulesOfHooksHandler {
  fn new() -> Self {
    Self {
      parent_kind: vec![],
    }
  }

  fn maybe_increase_cond_counter(&mut self) {
    if let Some(last) = self.parent_kind.last_mut() {
      if let ParentKind::Fn((name, count)) = last {
        let new_count = *count + 1;
        *last = ParentKind::Fn((name.to_string(), new_count))
      }
    }
  }
}

impl Handler for RulesOfHooksHandler {
  fn on_enter_node(&mut self, node: Node, _ctx: &mut Context) {
    match node {
      Node::FnDecl(fn_decl) => {
        let name = fn_decl.ident.sym().to_string();
        self.parent_kind.push(ParentKind::Fn((name, 0)));
      }
      Node::FnExpr(fn_expr) => {
        if let Some(id) = fn_expr.ident {
          let name = id.sym().to_string();
          self.parent_kind.push(ParentKind::Fn((name, 0)));
        } else {
          self.parent_kind.push(ParentKind::Unknown)
        }
      }
      Node::CondExpr(_) => {
        self.maybe_increase_cond_counter();
        self.parent_kind.push(ParentKind::Cond);
      }
      Node::BinExpr(_) => {
        self.maybe_increase_cond_counter();
        self.parent_kind.push(ParentKind::Bin);
      }
      Node::ArrowExpr(_) => {
        if let Some(ParentKind::Var(name)) = self.parent_kind.last() {
          self.parent_kind.push(ParentKind::Fn((name.to_string(), 0)));
          return;
        }

        self.parent_kind.push(ParentKind::Unknown);
      }
      Node::ExportDefaultExpr(_) => {
        self.parent_kind.push(ParentKind::ExportDefault)
      }
      Node::VarDeclarator(decl) => {
        if let Pat::Ident(id) = decl.name {
          self
            .parent_kind
            .push(ParentKind::Var(id.id.sym().to_string()))
        } else {
          self.parent_kind.push(ParentKind::Unknown)
        }
      }
      Node::IfStmt(_) => {
        self.maybe_increase_cond_counter();
        self.parent_kind.push(ParentKind::Cond);
      }
      Node::ForInStmt(_)
      | Node::ForOfStmt(_)
      | Node::ForStmt(_)
      | Node::WhileStmt(_) => {
        self.parent_kind.push(ParentKind::Loop);
      }
      Node::TryStmt(_) => {
        self.parent_kind.push(ParentKind::TryCatch);
      }
      _ => {}
    }
  }

  fn on_exit_node(&mut self, node: Node, _ctx: &mut Context) {
    match node {
      Node::FnDecl(_)
      | Node::ArrowExpr(_)
      | Node::BinExpr(_)
      | Node::CondExpr(_)
      | Node::VarDeclarator(_)
      | Node::IfStmt(_)
      | Node::ForInStmt(_)
      | Node::ForOfStmt(_)
      | Node::FnExpr(_)
      | Node::ForStmt(_)
      | Node::WhileStmt(_)
      | Node::ExportDefaultExpr(_)
      | Node::TryStmt(_) => {
        let _ = self.parent_kind.pop();
      }
      _ => {}
    }
  }

  fn call_expr(&mut self, node: &CallExpr, ctx: &mut Context) {
    if is_hook_call(node) {
      if self.parent_kind.is_empty() {
        ctx.add_diagnostic_with_hint(
          node.range(),
          CODE,
          DiagnosticKind::OutsideComponent.message(),
          DiagnosticKind::OutsideComponent.hint(),
        );
      }

      for item in self.parent_kind.iter().rev() {
        match item {
          ParentKind::Unknown => break,
          ParentKind::Var(name) => {
            if !is_hook_or_component_name(name) {
              ctx.add_diagnostic_with_hint(
                node.range(),
                CODE,
                DiagnosticKind::OutsideComponent.message(),
                DiagnosticKind::OutsideComponent.hint(),
              );
            }

            break;
          }
          ParentKind::Fn((name, cond_count)) => {
            if *cond_count > 0 {
              ctx.add_diagnostic_with_hint(
                node.range(),
                CODE,
                DiagnosticKind::Conditionally.message(),
                DiagnosticKind::Conditionally.hint(),
              );
            } else if !is_hook_or_component_name(name) {
              ctx.add_diagnostic_with_hint(
                node.range(),
                CODE,
                DiagnosticKind::OutsideComponent.message(),
                DiagnosticKind::OutsideComponent.hint(),
              );
            }

            break;
          }
          ParentKind::Loop => {
            ctx.add_diagnostic_with_hint(
              node.range(),
              CODE,
              DiagnosticKind::Loop.message(),
              DiagnosticKind::Loop.hint(),
            );
          }
          ParentKind::TryCatch => {
            ctx.add_diagnostic_with_hint(
              node.range(),
              CODE,
              DiagnosticKind::TryCatch.message(),
              DiagnosticKind::TryCatch.hint(),
            );
            break;
          }
          ParentKind::Bin | ParentKind::Cond => {
            ctx.add_diagnostic_with_hint(
              node.range(),
              CODE,
              DiagnosticKind::Conditionally.message(),
              DiagnosticKind::Conditionally.hint(),
            );
            break;
          }
          _ => {}
        }
      }
    }
  }
}

fn is_hook_call(call_expr: &CallExpr) -> bool {
  if let Callee::Expr(Expr::Ident(id)) = call_expr.callee {
    return id.sym().starts_with("use");
  }

  false
}

fn is_hook_or_component_name(name: &str) -> bool {
  if name.starts_with("use") {
    return true;
  }

  if let Some(ch) = name.chars().next() {
    return ch.is_uppercase();
  }

  false
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn rules_of_hooks_valid() {
    assert_lint_ok! {
      RulesOfHooks,
      filename: "file:///foo.jsx",
      r#"function Foo() { useState(0) }"#,
      r#"function useFoo() { useState(0) }"#,
      r#"export default () => { useState(0) }"#,
      r#"export const Foo = () => { useState(0) }"#,
      r#"export function Foo() { useState(0) }"#,
      r#"const Foo = () => { useState(0) }"#,
      r#"function foo() { return function Foo() { useState(0) }}"#,
      r#"function foo() { return () => { useState(0) }}"#,
    };
  }

  #[test]
  fn rules_of_hooks_invalid() {
    assert_lint_err! {
      RulesOfHooks,
      filename: "file:///foo.jsx",
      r#"function foo() { useState(0) }"#: [
        {
          col: 17,
          message: DiagnosticKind::OutsideComponent.message(),
          hint: DiagnosticKind::OutsideComponent.hint(),
        }
      ],
      r#"const foo = () => { useState(0) }"#: [
        {
          col: 20,
          message: DiagnosticKind::OutsideComponent.message(),
          hint: DiagnosticKind::OutsideComponent.hint(),
        }
      ],
      r#"export const foo = () => { useState(0) }"#: [
        {
          col: 27,
          message: DiagnosticKind::OutsideComponent.message(),
          hint: DiagnosticKind::OutsideComponent.hint(),
        }
      ],
      r#"function Foo() { if (cond) { useState(0) } }"#: [
        {
          col: 29,
          message: DiagnosticKind::Conditionally.message(),
          hint: DiagnosticKind::Conditionally.hint(),
        }
      ],
      r#"function Foo() { for (let i = 0; i < 10; i++) { useState(0) } }"#: [
        {
          col: 48,
          message: DiagnosticKind::Loop.message(),
          hint: DiagnosticKind::Loop.hint(),
        }
      ],
      r#"function Foo() { for (let a in b) { useState(0) } }"#: [
        {
          col: 36,
          message: DiagnosticKind::Loop.message(),
          hint: DiagnosticKind::Loop.hint(),
        }
      ],
      r#"function Foo() { for (let a of b) { useState(0) } }"#: [
        {
          col: 36,
          message: DiagnosticKind::Loop.message(),
          hint: DiagnosticKind::Loop.hint(),
        }
      ],
      r#"function Foo() { while (cond) { useState(0) } }"#: [
        {
          col: 32,
          message: DiagnosticKind::Loop.message(),
          hint: DiagnosticKind::Loop.hint(),
        }
      ],
      r#"function Foo() { if (cond) { return } useState(0) }"#: [
        {
          col: 38,
          message: DiagnosticKind::Conditionally.message(),
          hint: DiagnosticKind::Conditionally.hint(),
        }
      ],
      r#"function Foo() { try { useState(0) } catch {} }"#: [
        {
          col: 23,
          message: DiagnosticKind::TryCatch.message(),
          hint: DiagnosticKind::TryCatch.hint(),
        }
      ],
    };
  }
}