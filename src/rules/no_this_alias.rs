// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::view::{Expr, Pat, VarDecl};
use deno_ast::SourceRanged;
use if_chain::if_chain;

#[derive(Debug)]
pub struct NoThisAlias;

const CODE: &str = "no-this-alias";
const MESSAGE: &str = "assign `this` to declare a value is not allowed";

impl LintRule for NoThisAlias {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoThisAliasHandler.traverse(program, context);
  }
}

struct NoThisAliasHandler;

impl Handler for NoThisAliasHandler {
  fn var_decl(&mut self, var_decl: &VarDecl, ctx: &mut Context) {
    for decl in var_decl.decls {
      if_chain! {
        if let Some(init) = &decl.init;
        if matches!(&init, Expr::This(_));
        if matches!(&decl.name, Pat::Ident(_));
        then {
          ctx.add_diagnostic(var_decl.range(), CODE, MESSAGE);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_this_alias_valid() {
    assert_lint_ok! {
      NoThisAlias,
      "const self = foo(this);",
      "const self = 'this';",
      "const { props, state } = this;",
      "const [foo] = this;",
    };
  }

  #[test]
  fn no_this_alias_invalid() {
    assert_lint_err! {
      NoThisAlias,
      "const self = this;": [
        {
          col: 0,
          message: MESSAGE,
        }
      ],
      "
var unscoped = this;

function testFunction() {
  let inFunction = this;
}

const testLambda = () => {
  const inLambda = this;
};": [
        {
          line: 2,
          col: 0,
          message: MESSAGE,
        },
        {
          line: 5,
          col: 2,
          message: MESSAGE,
        },
        {
          line: 9,
          col: 2,
          message: MESSAGE,
        }
      ],
      "
class TestClass {
  constructor() {
    const inConstructor = this;
    const asThis: this = this;

    const asString = 'this';
    const asArray = [this];
    const asArrayString = ['this'];
  }

  public act(scope: this = this) {
    const inMemberFunction = this;
  }
}": [
        {
          line: 4,
          col: 4,
          message: MESSAGE,
        },
        {
          line: 5,
          col: 4,
          message: MESSAGE,
        },
        {
          line: 13,
          col: 4,
          message: MESSAGE,
        }
      ],
      "const foo = function() { const self = this; };": [
        {
          col: 25,
          message: MESSAGE,
        }
      ]
    };
  }
}
