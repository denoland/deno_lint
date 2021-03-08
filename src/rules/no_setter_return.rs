// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use dprint_swc_ecma_ast_view::{self as AstView, NodeTrait, Spanned};

pub struct NoSetterReturn;

impl LintRule for NoSetterReturn {
  fn new() -> Box<Self> {
    Box::new(NoSetterReturn)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-setter-return"
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!()
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: dprint_swc_ecma_ast_view::Program<'_>,
  ) {
    NoSetterReturnHandler.traverse(program, context);
  }
}

struct NoSetterReturnHandler;

impl Handler for NoSetterReturnHandler {
  fn return_stmt(&self, return_stmt: &AstView::ReturnStmt, ctx: &mut Context) {
    fn inside_setter(node: AstView::Node) -> bool {
      use AstView::Node::*;
      match node {
        SetterProp(_) => true,
        ClassMethod(ref method)
          if method.kind() == AstView::MethodKind::Setter =>
        {
          true
        }
        _ => {
          if let Some(parent) = node.parent() {
            inside_setter(parent)
          } else {
            false
          }
        }
      }
    }

    if inside_setter(return_stmt.into_node()) {
      ctx.add_diagnostic(
        return_stmt.span(),
        "no-setter-return",
        "Setter cannot return a value",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_setter_return_invalid() {
    assert_lint_err::<NoSetterReturn>(
      r#"const a = { set setter(a) { return "something"; } };"#,
      28,
    );
    assert_lint_err_on_line_n::<NoSetterReturn>(
      r#"
class b {
  set setterA(a) {
    return "something";
  }
  private set setterB(a) {
    return "something";
  }
}
      "#,
      vec![(4, 4), (7, 4)],
    );
  }
}
