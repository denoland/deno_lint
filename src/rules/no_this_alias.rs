// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use if_chain::if_chain;
use swc_ecmascript::ast::{Expr, Pat, VarDecl};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::{VisitAll, VisitAllWith};

pub struct NoThisAlias;

const CODE: &str = "no-this-alias";
const MESSAGE: &str = "assign `this` to declare a value is not allowed";

impl LintRule for NoThisAlias {
  fn new() -> Box<Self> {
    Box::new(NoThisAlias)
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
    let mut visitor = NoThisAliasVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_this_alias.md")
  }
}

struct NoThisAliasVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoThisAliasVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> VisitAll for NoThisAliasVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    for decl in &var_decl.decls {
      if_chain! {
        if let Some(init) = &decl.init;
        if matches!(&**init, Expr::This(_));
        if matches!(&decl.name, Pat::Ident(_));
        then {
          self.context.add_diagnostic(var_decl.span, CODE, MESSAGE);
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
