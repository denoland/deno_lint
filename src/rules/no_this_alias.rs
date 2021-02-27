// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecmascript::ast::{ArrowExpr, Expr, Function, Pat, VarDecl};
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

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

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoThisAliasVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct NoThisAliasVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoThisAliasVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoThisAliasVisitor<'c> {
  noop_visit_type!();

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _parent: &dyn Node) {
    for decl in &var_decl.decls {
      if let Some(init) = &decl.init {
        if let Expr::Arrow(arrow) = &**init {
          self.visit_arrow_expr(&arrow, _parent);
        } else if let Expr::This(_) = &**init {
          if let Pat::Ident(_ident) = &decl.name {
            self.context.add_diagnostic(var_decl.span, CODE, MESSAGE);
          }
        }
      }
    }
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _parent: &dyn Node) {
    self.visit_block_stmt_or_expr(&arrow_expr.body, _parent);
  }

  fn visit_expr_stmt(
    &mut self,
    expr: &swc_ecmascript::ast::ExprStmt,
    _parent: &dyn Node,
  ) {
    if let Expr::Arrow(arrow) = &*expr.expr {
      self.visit_arrow_expr(arrow, _parent);
    }
  }

  fn visit_function(&mut self, function: &Function, _parent: &dyn Node) {
    if let Some(stmt) = &function.body {
      self.visit_block_stmt(stmt, _parent);
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
      ]
    };
  }
}
