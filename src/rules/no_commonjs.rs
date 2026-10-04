// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::{
  ast::*,
  ecma_visit::{noop_visit_type, Visit, VisitWith},
};
use deno_ast::SourceRangedForSpanned;

#[derive(Debug)]
pub struct NoCommonjs;

const CODE: &str = "no-commonjs";
const MESSAGE: &str = "CommonJS exports are not allowed";
const HINT: &str = "Use an ES module export declaration, such as `export default` or `export { name }`";

impl LintRule for NoCommonjs {
  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = NoCommonjsVisitor { context };
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }
}

struct NoCommonjsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl NoCommonjsVisitor<'_, '_> {
  fn is_unshadowed(&self, ident: &Ident) -> bool {
    ident.ctxt == self.context.unresolved_ctxt()
  }

  fn is_module_exports(&self, expr: &Expr) -> bool {
    let Expr::Member(member) = expr else {
      return false;
    };
    self.is_module_exports_member(member)
  }

  fn is_module_exports_member(&self, member: &MemberExpr) -> bool {
    let Expr::Ident(object) = &*member.obj else {
      return false;
    };
    if &*object.sym != "module" || !self.is_unshadowed(object) {
      return false;
    }
    match &member.prop {
      MemberProp::Ident(property) => &*property.sym == "exports",
      MemberProp::Computed(property) => matches!(
        &*property.expr,
        Expr::Lit(Lit::Str(value)) if value.value == "exports"
      ),
      MemberProp::PrivateName(_) => false,
    }
  }

  fn is_commonjs_member(&self, member: &MemberExpr) -> bool {
    self.is_module_exports_member(member)
      || match &*member.obj {
        Expr::Ident(object) => {
          &*object.sym == "exports" && self.is_unshadowed(object)
        }
        object => self.is_module_exports(object),
      }
  }
}

impl Visit for NoCommonjsVisitor<'_, '_> {
  noop_visit_type!();

  fn visit_member_expr(&mut self, member: &MemberExpr) {
    if self.is_commonjs_member(member) {
      self.context.add_diagnostic_with_hint(
        member.range(),
        CODE,
        MESSAGE,
        HINT,
      );
      return;
    }
    member.visit_children_with(self);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_commonjs_valid() {
    assert_lint_ok! {
      NoCommonjs,
      "export const value = 1;",
      "const exports = {}; exports.value = 1;",
      "const module = { exports: {} }; module.exports.value = 1;",
      "const value = object.exports;",
    };
  }

  #[test]
  fn no_commonjs_invalid() {
    assert_lint_err! {
      NoCommonjs,
      "exports.value = 1;": [{ col: 0, line: 1, message: MESSAGE, hint: HINT }],
      "module.exports = value;": [{ col: 0, line: 1, message: MESSAGE, hint: HINT }],
      "module['exports'].value = 1;": [{ col: 0, line: 1, message: MESSAGE, hint: HINT }],
      "module.exports.value = 1;": [{ col: 0, line: 1, message: MESSAGE, hint: HINT }],
    }
  }
}
