// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::program_ref;
use super::{Context, LintRule};
use crate::tags::{self, Tags};
use crate::Program;
use crate::ProgramRef;
use deno_ast::BindingKind;
use deno_ast::{
  swc::{
    ast::*,
    ecma_visit::{noop_visit_type, Visit, VisitWith},
  },
  SourceRange, SourceRangedForSpanned,
};

#[derive(Debug)]
pub struct NoImportAssign;

const CODE: &str = "no-import-assign";
const MESSAGE: &str = "Assignment to import is not allowed";
const HINT: &str = "Assign to another variable, this assignment is invalid";

impl LintRule for NoImportAssign {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = NoImportAssignVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }
}

struct NoImportAssignVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoImportAssignVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn check(&mut self, range: SourceRange, i: &Ident, is_assign_to_prop: bool) {
    let var = self.context.scope().var(&i.to_id());
    if var.is_some_and(|v| v.kind() == BindingKind::NamespaceImport) {
      self
        .context
        .add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
      return;
    }

    if !is_assign_to_prop
      && var.is_some_and(|v| v.kind() == BindingKind::ValueImport)
    {
      self
        .context
        .add_diagnostic_with_hint(range, CODE, MESSAGE, HINT);
    }
  }

  fn check_assign(
    &mut self,
    range: SourceRange,
    lhs: &Expr,
    is_assign_to_prop: bool,
  ) {
    if let Expr::Ident(lhs) = &lhs {
      self.check(range, lhs, is_assign_to_prop);
    }
  }

  fn check_expr(&mut self, range: SourceRange, e: &Expr) {
    match e {
      Expr::Ident(i) => {
        self.check(range, i, false);
      }
      Expr::Member(e) => self.check_assign(range, &e.obj, true),
      Expr::Paren(e) => self.check_expr(range, &e.expr),
      Expr::OptChain(e) => match &*e.base {
        OptChainBase::Call(e) => self.visit_opt_call(e),
        OptChainBase::Member(e) => self.check_expr(range, &e.obj),
      },
      _ => e.visit_children_with(self),
    }
  }

  fn check_simple_assign(
    &mut self,
    range: SourceRange,
    e: &SimpleAssignTarget,
  ) {
    match e {
      SimpleAssignTarget::Ident(i) => {
        self.check(range, i, false);
      }
      SimpleAssignTarget::Member(e) => self.check_assign(range, &e.obj, true),
      SimpleAssignTarget::Paren(e) => self.check_expr(range, &e.expr),
      SimpleAssignTarget::OptChain(e) => match &*e.base {
        OptChainBase::Call(e) => self.visit_opt_call(e),
        OptChainBase::Member(e) => self.check_expr(range, &e.obj),
      },
      _ => e.visit_children_with(self),
    }
  }

  fn is_modifier(&self, obj: &Expr, prop: &IdentName) -> bool {
    let obj = if let Expr::Ident(obj) = obj {
      obj
    } else {
      return false;
    };

    if self
      .context
      .scope()
      .var(&obj.to_id()).is_some_and(|v| !v.kind().is_import())
    {
      return false;
    }

    match &*obj.sym {
      "Object" => {
        // Check for Object.defineProperty and Object.assign
        *prop.sym == *"defineProperty"
          || *prop.sym == *"assign"
          || *prop.sym == *"setPrototypeOf"
          || *prop.sym == *"freeze"
      }

      "Reflect" => {
        *prop.sym == *"defineProperty"
          || *prop.sym == *"deleteProperty"
          || *prop.sym == *"set"
          || *prop.sym == *"setPrototypeOf"
      }
      _ => false,
    }
  }

  /// Returns true for callees like `Object.assign`
  fn modifies_first(&self, callee: &Expr) -> bool {
    match callee {
      Expr::OptChain(opt_chain) => {
        if let OptChainBase::Member(member_expr) = &*opt_chain.base {
          return self.member_expr_modifies_first(member_expr);
        }
      }
      Expr::Member(member_expr) => {
        return self.member_expr_modifies_first(member_expr);
      }

      Expr::Paren(ParenExpr { expr, .. }) => return self.modifies_first(expr),

      _ => {}
    }

    false
  }

  fn member_expr_modifies_first(&self, member_expr: &MemberExpr) -> bool {
    if let MemberProp::Ident(ident) = &member_expr.prop {
      if self.is_modifier(&member_expr.obj, ident) {
        return true;
      }
    }
    false
  }
}

impl Visit for NoImportAssignVisitor<'_, '_> {
  noop_visit_type!();

  fn visit_pat(&mut self, n: &Pat) {
    match n {
      Pat::Ident(i) => {
        self.check(i.id.range(), &i.id, false);
      }
      Pat::Expr(e) => {
        self.check_expr(n.range(), e);
      }
      _ => {
        n.visit_children_with(self);
      }
    }
  }

  fn visit_rest_pat(&mut self, n: &RestPat) {
    if let Pat::Expr(e) = &*n.arg {
      match &**e {
        Expr::Ident(i) => {
          self.check(i.range(), i, true);
        }
        _ => {
          self.check_expr(e.range(), e);
        }
      }
    } else {
      n.visit_children_with(self)
    }
  }

  fn visit_assign_expr(&mut self, n: &AssignExpr) {
    match &n.left {
      AssignTarget::Simple(simple) => {
        self.check_simple_assign(n.range(), simple);
      }
      AssignTarget::Pat(p) => {
        p.visit_with(self);
      }
    }
    n.right.visit_with(self);
  }

  fn visit_assign_pat_prop(&mut self, n: &AssignPatProp) {
    self.check(n.key.range(), &n.key, false);

    n.value.visit_children_with(self);
  }

  fn visit_update_expr(&mut self, n: &UpdateExpr) {
    self.check_expr(n.range(), &n.arg);
  }

  fn visit_unary_expr(&mut self, n: &UnaryExpr) {
    if let UnaryOp::Delete = n.op {
      self.check_expr(n.range(), &n.arg);
    } else {
      n.arg.visit_with(self);
    }
  }

  fn visit_call_expr(&mut self, n: &CallExpr) {
    n.visit_children_with(self);

    if let Callee::Expr(callee) = &n.callee {
      if let Some(arg) = n.args.first() {
        if self.modifies_first(callee) {
          self.check_assign(n.range(), &arg.expr, true);
        }
      }
    }
  }

  fn visit_opt_call(&mut self, n: &OptCall) {
    n.visit_children_with(self);

    if let Some(arg) = n.args.first() {
      if self.modifies_first(&n.callee) {
        self.check_assign(n.range(), &arg.expr, true);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_import_assign_valid() {
    assert_lint_ok! {
      NoImportAssign,
      "import mod from 'mod'; mod.prop = 0",
      "import mod from 'mod'; mod.prop += 0;",
      "import mod from 'mod'; mod.prop++",
      "import mod from 'mod'; delete mod.prop",
      "import mod from 'mod'; for (mod.prop in foo);",
      "import mod from 'mod'; for (mod.prop of foo);",
      "import mod from 'mod'; [mod.prop] = foo;",
      "import mod from 'mod'; [...mod.prop] = foo;",
      "import mod from 'mod'; ({ bar: mod.prop } = foo);",
      "import mod from 'mod'; ({ ...mod.prop } = foo);",
      "import {named} from 'mod'; named.prop = 0",
      "import {named} from 'mod'; named.prop += 0",
      "import {named} from 'mod'; named.prop++",
      "import {named} from 'mod'; delete named.prop",
      "import {named} from 'mod'; for (named.prop in foo);",
      "import {named} from 'mod'; for (named.prop of foo);",
      "import {named} from 'mod'; [named.prop] = foo;",
      "import {named} from 'mod'; [...named.prop] = foo;",
      "import {named} from 'mod'; ({ bar: named.prop } = foo);",
      "import {named} from 'mod'; ({ ...named.prop } = foo);",
      "import * as mod from 'mod'; mod.named.prop = 0",
      "import * as mod from 'mod'; mod.named.prop += 0",
      "import * as mod from 'mod'; mod.named.prop++",
      "import * as mod from 'mod'; delete mod.named.prop",
      "import * as mod from 'mod'; for (mod.named.prop in foo);",
      "import * as mod from 'mod'; for (mod.named.prop of foo);",
      "import * as mod from 'mod'; [mod.named.prop] = foo;",
      "import * as mod from 'mod'; [...mod.named.prop] = foo;",
      "import * as mod from 'mod'; ({ bar: mod.named.prop } = foo);",
      "import * as mod from 'mod'; ({ ...mod.named.prop } = foo);",
      "import * as mod from 'mod'; obj[mod] = 0",
      "import * as mod from 'mod'; obj[mod.named] = 0",
      "import * as mod from 'mod'; for (var foo in mod.named);",
      "import * as mod from 'mod'; for (var foo of mod.named);",
      "import * as mod from 'mod'; [bar = mod.named] = foo;",
      "import * as mod from 'mod'; ({ bar = mod.named } = foo);",
      "import * as mod from 'mod'; ({ bar: baz = mod.named } = foo);",
      "import * as mod from 'mod'; ({ [mod.named]: bar } = foo);",
      "import * as mod from 'mod'; var obj = { ...mod.named };",
      "import * as mod from 'mod'; var obj = { foo: mod.named };",
      "import mod from 'mod'; { let mod = 0; mod = 1 }",
      "import * as mod from 'mod'; { let mod = 0; mod = 1 }",
      "import * as mod from 'mod'; { let mod = 0; mod.named = 1 }",
      "import {} from 'mod'",
      "import 'mod'",
      "import mod from 'mod'; Object.assign(mod, obj);",
      "import {named} from 'mod'; Object.assign(named, obj);",
      "import * as mod from 'mod'; Object.assign(mod.prop, obj);",
      "import * as mod from 'mod'; Object.assign(obj, mod, other);",
      "import * as mod from 'mod'; Object[assign](mod, obj);",
      "import * as mod from 'mod'; Object.getPrototypeOf(mod);",
      "import * as mod from 'mod'; Reflect.set(obj, key, mod);",
      "import * as mod from 'mod'; { var Object; Object.assign(mod, obj); }",
      "import * as mod from 'mod'; var Object; Object.assign(mod, obj);",
      "import * as mod from 'mod'; Object.seal(mod, obj)",
      "import * as mod from 'mod'; Object.preventExtensions(mod)",
      "import * as mod from 'mod'; Reflect.preventExtensions(mod)",
    };
  }

  #[test]
  fn no_import_assign_invalid() {
    assert_lint_err! {
      NoImportAssign,
      "import mod1 from 'mod'; mod1 = 0": [{ col: 24, message: MESSAGE, hint: HINT }],
      "import mod2 from 'mod'; mod2 += 0": [{ col: 24, message: MESSAGE, hint: HINT }],
      "import mod3 from 'mod'; mod3++": [{ col: 24, message: MESSAGE, hint: HINT }],
      "import mod4 from 'mod'; for (mod4 in foo);": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import mod5 from 'mod'; for (mod5 of foo);": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import mod6 from 'mod'; [mod6] = foo": [{ col: 25, message: MESSAGE, hint: HINT }],
      "import mod7 from 'mod'; [mod7 = 0] = foo": [{ col: 25, message: MESSAGE, hint: HINT }],
      "import mod8 from 'mod'; [...mod8] = foo": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import mod9 from 'mod'; ({ bar: mod9 } = foo)": [{ col: 32, message: MESSAGE, hint: HINT }],
      "import mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)": [{ col: 33, message: MESSAGE, hint: HINT }],
      "import mod11 from 'mod'; ({ ...mod11 } = foo)": [{ col: 31, message: MESSAGE, hint: HINT }],
      "import {named1} from 'mod'; named1 = 0": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import {named2} from 'mod'; named2 += 0": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import {named3} from 'mod'; named3++": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import {named4} from 'mod'; for (named4 in foo);": [{ col: 33, message: MESSAGE, hint: HINT }],
      "import {named5} from 'mod'; for (named5 of foo);": [{ col: 33, message: MESSAGE, hint: HINT }],
      "import {named6} from 'mod'; [named6] = foo": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import {named7} from 'mod'; [named7 = 0] = foo": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import {named8} from 'mod'; [...named8] = foo": [{ col: 32, message: MESSAGE, hint: HINT }],
      "import {named9} from 'mod'; ({ bar: named9 } = foo)": [{ col: 36, message: MESSAGE, hint: HINT }],
      "import {named10} from 'mod'; ({ bar: named10 = 0 } = foo)": [{ col: 37, message: MESSAGE, hint: HINT }],
      "import {named11} from 'mod'; ({ ...named11 } = foo)": [{ col: 35, message: MESSAGE, hint: HINT }],
      "import {named12 as foo} from 'mod'; foo = 0; named12 = 0": [{ col: 36, message: MESSAGE, hint: HINT }],
      "import * as mod1 from 'mod'; mod1 = 0": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import * as mod2 from 'mod'; mod2 += 0": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import * as mod3 from 'mod'; mod3++": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import * as mod4 from 'mod'; for (mod4 in foo);": [{ col: 34, message: MESSAGE, hint: HINT }],
      "import * as mod5 from 'mod'; for (mod5 of foo);": [{ col: 34, message: MESSAGE, hint: HINT }],
      "import * as mod6 from 'mod'; [mod6] = foo": [{ col: 30, message: MESSAGE, hint: HINT }],
      "import * as mod7 from 'mod'; [mod7 = 0] = foo": [{ col: 30, message: MESSAGE, hint: HINT }],
      "import * as mod8 from 'mod'; [...mod8] = foo": [{ col: 33, message: MESSAGE, hint: HINT }],
      "import * as mod9 from 'mod'; ({ bar: mod9 } = foo)": [{ col: 37, message: MESSAGE, hint: HINT }],
      "import * as mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)": [{ col: 38, message: MESSAGE, hint: HINT }],
      "import * as mod11 from 'mod'; ({ ...mod11 } = foo)": [{ col: 36, message: MESSAGE, hint: HINT }],
      "import * as mod1 from 'mod'; mod1.named = 0": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import * as mod2 from 'mod'; mod2.named += 0": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import * as mod3 from 'mod'; mod3.named++": [{ col: 29, message: MESSAGE, hint: HINT }],
      "import * as mod4 from 'mod'; for (mod4.named in foo);": [{ col: 34, message: MESSAGE, hint: HINT }],
      "import * as mod5 from 'mod'; for (mod5.named of foo);": [{ col: 34, message: MESSAGE, hint: HINT }],
      "import * as mod6 from 'mod'; [mod6.named] = foo": [{ col: 30, message: MESSAGE, hint: HINT }],
      "import * as mod7 from 'mod'; [mod7.named = 0] = foo": [{ col: 30, message: MESSAGE, hint: HINT }],
      "import * as mod8 from 'mod'; [...mod8.named] = foo": [{ col: 33, message: MESSAGE, hint: HINT }],
      "import * as mod9 from 'mod'; ({ bar: mod9.named } = foo)": [{ col: 37, message: MESSAGE, hint: HINT }],
      "import * as mod10 from 'mod'; ({ bar: mod10.named = 0 } = foo)": [{ col: 38, message: MESSAGE, hint: HINT }],
      "import * as mod11 from 'mod'; ({ ...mod11.named } = foo)": [{ col: 36, message: MESSAGE, hint: HINT }],
      "import * as mod12 from 'mod'; delete mod12.named": [{ col: 30, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Object.assign(mod, obj)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Object.defineProperty(mod, key, d)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Object.setPrototypeOf(mod, proto)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Object.freeze(mod)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Reflect.defineProperty(mod, key, d)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Reflect.deleteProperty(mod, key)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Reflect.set(mod, key, value)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Reflect.setPrototypeOf(mod, proto)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import mod, * as mod_ns from 'mod'; mod.prop = 0; mod_ns.prop = 0": [{ col: 50, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; Object?.defineProperty(mod, key, d)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; (Object?.defineProperty)(mod, key, d)": [{ col: 28, message: MESSAGE, hint: HINT }],
      "import * as mod from 'mod'; delete mod?.prop": [{ col: 28, message: MESSAGE, hint: HINT }],
    }
  }
}
