// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::TsEnumDecl;
use deno_ast::swc::visit::Node;
use deno_ast::swc::visit::Visit;

#[derive(Debug)]
pub struct NoEmptyEnum;

const CODE: &str = "no-empty-enum";
const MESSAGE: &str = "An empty enum is equivalent to `{{}}`.";
const HINT: &str = "Remove this enum or add members to this enum.";

impl LintRule for NoEmptyEnum {
  fn new() -> Box<Self> {
    Box::new(NoEmptyEnum)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoEmptyEnumVisitor::new(context);
    match program {
      ProgramRef::Module(m) => visitor.visit_module(m, &DUMMY_NODE),
      ProgramRef::Script(s) => visitor.visit_script(s, &DUMMY_NODE),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_empty_enum.md")
  }
}

struct NoEmptyEnumVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoEmptyEnumVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoEmptyEnumVisitor<'c, 'view> {
  fn visit_ts_enum_decl(
    &mut self,
    enum_decl: &TsEnumDecl,
    _parent: &dyn Node,
  ) {
    if enum_decl.members.is_empty() {
      self.context.add_diagnostic_with_hint(
        enum_decl.span,
        CODE,
        MESSAGE,
        HINT,
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_enum_valid() {
    assert_lint_ok! {
      NoEmptyEnum,
      "enum Foo { ONE = 'ONE', TWO = 'TWO' }",
      "const enum Foo { ONE = 'ONE' }",
    };
  }

  #[test]
  fn no_empty_enum_invalid() {
    assert_lint_err! {
      NoEmptyEnum,
      "enum Foo {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "const enum Foo {}": [
        {
          col: 0,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "export enum Foo {}": [
        {
          col: 7,
          message: MESSAGE,
          hint: HINT,
        }
      ],
      "export const enum Foo {}": [
        {
          col: 7,
          message: MESSAGE,
          hint: HINT,
        }
      ]
    };
  }
}
