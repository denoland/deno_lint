// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;
use deno_ast::BindingKind;

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

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoImportAssignHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoImportAssignHandler;

impl NoImportAssignHandler {
  fn check_ident_ref(&self, span: Span, ident: &IdentifierReference, is_assign_to_prop: bool, ctx: &mut Context) {
    let kind = ctx.binding_kind_of_ident_ref(ident);
    if !matches!(kind, Some(k) if k.is_import()) {
      return;
    }
    // Use deno_ast scope to distinguish namespace vs value imports for property assignment handling
    let is_namespace = ctx
      .scope()
      .var_by_name(ident.name.as_str())
      .is_some_and(|v| v.kind() == BindingKind::NamespaceImport);

    if is_namespace {
      // Namespace imports: any assignment (including property) is disallowed
      ctx.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
      return;
    }

    if !is_assign_to_prop {
      // Value imports: only direct reassignment is disallowed
      ctx.add_diagnostic_with_hint(span, CODE, MESSAGE, HINT);
    }
  }

  fn check_simple_target(&self, span: Span, target: &SimpleAssignmentTarget, ctx: &mut Context) {
    match target {
      SimpleAssignmentTarget::AssignmentTargetIdentifier(ident) => {
        self.check_ident_ref(span, ident, false, ctx);
      }
      SimpleAssignmentTarget::StaticMemberExpression(member) => {
        self.check_expr_for_assign(span, &member.object, true, ctx);
      }
      SimpleAssignmentTarget::ComputedMemberExpression(member) => {
        self.check_expr_for_assign(span, &member.object, true, ctx);
      }
      _ => {}
    }
  }

  fn check_expr(&self, span: Span, expr: &Expression, ctx: &mut Context) {
    match expr {
      Expression::Identifier(ident) => {
        self.check_ident_ref(span, ident, false, ctx);
      }
      Expression::StaticMemberExpression(member) => {
        self.check_expr_for_assign(span, &member.object, true, ctx);
      }
      Expression::ComputedMemberExpression(member) => {
        self.check_expr_for_assign(span, &member.object, true, ctx);
      }
      Expression::ParenthesizedExpression(paren) => {
        self.check_expr(span, &paren.expression, ctx);
      }
      Expression::ChainExpression(chain) => match &chain.expression {
        ChainElement::CallExpression(call) => {
          self.check_call_modifies_first(call.span, &call.callee, &call.arguments, ctx);
        }
        ChainElement::StaticMemberExpression(member) => {
          self.check_expr(span, &member.object, ctx);
        }
        ChainElement::ComputedMemberExpression(member) => {
          self.check_expr(span, &member.object, ctx);
        }
        ChainElement::PrivateFieldExpression(member) => {
          self.check_expr(span, &member.object, ctx);
        }
        ChainElement::TSNonNullExpression(_) => {}
      },
      _ => {}
    }
  }

  fn check_expr_for_assign(
    &self,
    span: Span,
    expr: &Expression,
    is_assign_to_prop: bool,
    ctx: &mut Context,
  ) {
    if let Expression::Identifier(ident) = expr {
      self.check_ident_ref(span, ident, is_assign_to_prop, ctx);
    }
  }

  fn check_target(&self, target: &AssignmentTarget, ctx: &mut Context) {
    match target {
      AssignmentTarget::AssignmentTargetIdentifier(ident) => {
        self.check_ident_ref(ident.span, ident, false, ctx);
      }
      AssignmentTarget::StaticMemberExpression(member) => {
        self.check_expr_for_assign(member.span, &member.object, true, ctx);
      }
      AssignmentTarget::ComputedMemberExpression(member) => {
        self.check_expr_for_assign(member.span, &member.object, true, ctx);
      }
      AssignmentTarget::ObjectAssignmentTarget(obj) => {
        for prop in obj.properties.iter() {
          match prop {
            AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(ident) => {
              self.check_ident_ref(ident.binding.span, &ident.binding, false, ctx);
            }
            AssignmentTargetProperty::AssignmentTargetPropertyProperty(kv) => {
              self.check_target_maybe_default(&kv.binding, ctx);
            }
          }
        }
        if let Some(rest) = &obj.rest {
          self.check_target(&rest.target, ctx);
        }
      }
      AssignmentTarget::ArrayAssignmentTarget(arr) => {
        for elem in arr.elements.iter().flatten() {
          self.check_target_maybe_default(elem, ctx);
        }
        if let Some(rest) = &arr.rest {
          self.check_target(&rest.target, ctx);
        }
      }
      _ => {}
    }
  }

  fn check_target_maybe_default(
    &self,
    target: &AssignmentTargetMaybeDefault,
    ctx: &mut Context,
  ) {
    match target {
      AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with_def) => {
        self.check_target(&with_def.binding, ctx);
      }
      _ => {
        if let Some(t) = target.as_assignment_target() {
          self.check_target(t, ctx);
        }
      }
    }
  }

  fn is_modifier(&self, obj_name: &str, prop_name: &str, ctx: &Context) -> bool {
    if ctx.scope().var_by_name(obj_name).is_some_and(|v| !v.kind().is_import()) {
      return false;
    }

    match obj_name {
      "Object" => {
        matches!(
          prop_name,
          "defineProperty" | "assign" | "setPrototypeOf" | "freeze"
        )
      }
      "Reflect" => {
        matches!(
          prop_name,
          "defineProperty" | "deleteProperty" | "set" | "setPrototypeOf"
        )
      }
      _ => false,
    }
  }

  fn modifies_first(&self, callee: &Expression, ctx: &Context) -> bool {
    match callee {
      Expression::ChainExpression(chain) => {
        if let ChainElement::StaticMemberExpression(member) = &chain.expression {
          return self.member_expr_modifies_first(&member.object, member.property.name.as_str(), ctx);
        }
        false
      }
      Expression::StaticMemberExpression(member) => {
        self.member_expr_modifies_first(&member.object, member.property.name.as_str(), ctx)
      }
      Expression::ParenthesizedExpression(paren) => {
        self.modifies_first(&paren.expression, ctx)
      }
      _ => false,
    }
  }

  fn member_expr_modifies_first(
    &self,
    obj: &Expression,
    prop_name: &str,
    ctx: &Context,
  ) -> bool {
    if let Expression::Identifier(ident) = obj {
      self.is_modifier(ident.name.as_str(), prop_name, ctx)
    } else {
      false
    }
  }

  fn check_call_modifies_first(
    &self,
    span: Span,
    callee: &Expression,
    arguments: &[Argument],
    ctx: &mut Context,
  ) {
    if let Some(arg) = arguments.first() {
      if self.modifies_first(callee, ctx) {
        if let Some(expr) = arg.as_expression() {
          if let Expression::Identifier(ident) = expr {
            self.check_ident_ref(span, ident, true, ctx);
          }
        }
      }
    }
  }
}

impl Handler<'_> for NoImportAssignHandler {
  fn assignment_expression(
    &mut self,
    n: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    self.check_target(&n.left, ctx);
  }

  fn update_expression(&mut self, n: &UpdateExpression, ctx: &mut Context) {
    self.check_simple_target(n.span, &n.argument, ctx);
  }

  fn unary_expression(&mut self, n: &UnaryExpression, ctx: &mut Context) {
    if n.operator == UnaryOperator::Delete {
      self.check_expr(n.span, &n.argument, ctx);
    }
  }

  fn call_expression(&mut self, n: &CallExpression, ctx: &mut Context) {
    self.check_call_modifies_first(n.span, &n.callee, &n.arguments, ctx);
  }

  fn for_in_statement(&mut self, n: &ForInStatement, ctx: &mut Context) {
    if let ForStatementLeft::AssignmentTargetIdentifier(ident) = &n.left {
      self.check_ident_ref(ident.span, ident, false, ctx);
    } else if let Some(target) = n.left.as_assignment_target() {
      self.check_target(target, ctx);
    }
  }

  fn for_of_statement(&mut self, n: &ForOfStatement, ctx: &mut Context) {
    if let ForStatementLeft::AssignmentTargetIdentifier(ident) = &n.left {
      self.check_ident_ref(ident.span, ident, false, ctx);
    } else if let Some(target) = n.left.as_assignment_target() {
      self.check_target(target, ctx);
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
