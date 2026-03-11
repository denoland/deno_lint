// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::*;
use deno_ast::oxc::span::Span;
use derive_more::Display;

#[derive(Debug)]
pub struct NoShadowRestrictedNames;

const CODE: &str = "no-shadow-restricted-names";

#[derive(Display)]
enum NoShadowRestrictedNamesMessage {
  #[display(fmt = "Shadowing of global property {}", _0)]
  Shadowing(String),
}

impl LintRule for NoShadowRestrictedNames {
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
    let mut handler = NoShadowRestrictedNamesHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoShadowRestrictedNamesHandler;

fn is_restricted(name: &str) -> bool {
  matches!(name, "undefined" | "NaN" | "Infinity" | "arguments" | "eval")
}

fn report(span: Span, name: &str, ctx: &mut Context) {
  ctx.add_diagnostic(
    span,
    CODE,
    NoShadowRestrictedNamesMessage::Shadowing(name.to_string()),
  );
}

/// Recursively check a BindingPattern for restricted names
fn check_binding_pattern(pat: &BindingPattern, ctx: &mut Context) {
  match pat {
    BindingPattern::BindingIdentifier(ident) => {
      if is_restricted(ident.name.as_str()) {
        report(ident.span, ident.name.as_str(), ctx);
      }
    }
    BindingPattern::ArrayPattern(arr) => {
      for el in &arr.elements {
        if let Some(el) = el {
          check_binding_pattern(el, ctx);
        }
      }
      if let Some(rest) = &arr.rest {
        check_binding_pattern(&rest.argument, ctx);
      }
    }
    BindingPattern::ObjectPattern(obj) => {
      for prop in &obj.properties {
        check_binding_pattern(&prop.value, ctx);
      }
      if let Some(rest) = &obj.rest {
        check_binding_pattern(&rest.argument, ctx);
      }
    }
    BindingPattern::AssignmentPattern(assign) => {
      check_binding_pattern(&assign.left, ctx);
    }
  }
}

/// Check an AssignmentTarget for restricted names
fn check_assignment_target(target: &AssignmentTarget, ctx: &mut Context) {
  match target {
    AssignmentTarget::AssignmentTargetIdentifier(ident) => {
      if is_restricted(ident.name.as_str()) {
        report(ident.span, ident.name.as_str(), ctx);
      }
    }
    AssignmentTarget::ArrayAssignmentTarget(arr) => {
      for el in arr.elements.iter().flatten() {
        match el {
          AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(with_default) => {
            check_assignment_target(&with_default.binding, ctx);
          }
          _ => {
            if let Some(target) = el.as_assignment_target() {
              check_assignment_target(target, ctx);
            }
          }
        }
      }
      if let Some(rest) = &arr.rest {
        check_assignment_target(&rest.target, ctx);
      }
    }
    AssignmentTarget::ObjectAssignmentTarget(obj) => {
      for prop in &obj.properties {
        match prop {
          AssignmentTargetProperty::AssignmentTargetPropertyIdentifier(
            ident_prop,
          ) => {
            if is_restricted(ident_prop.binding.name.as_str()) {
              report(
                ident_prop.binding.span,
                ident_prop.binding.name.as_str(),
                ctx,
              );
            }
          }
          AssignmentTargetProperty::AssignmentTargetPropertyProperty(
            key_value,
          ) => {
            match &key_value.binding {
              AssignmentTargetMaybeDefault::AssignmentTargetWithDefault(
                with_default,
              ) => {
                check_assignment_target(&with_default.binding, ctx);
              }
              _ => {
                if let Some(target) = key_value.binding.as_assignment_target()
                {
                  check_assignment_target(target, ctx);
                }
              }
            }
          }
        }
      }
      if let Some(rest) = &obj.rest {
        check_assignment_target(&rest.target, ctx);
      }
    }
    _ => {}
  }
}

impl Handler<'_> for NoShadowRestrictedNamesHandler {
  fn variable_declaration(
    &mut self,
    node: &VariableDeclaration,
    ctx: &mut Context,
  ) {
    for decl in &node.declarations {
      if let BindingPattern::BindingIdentifier(ident) = &decl.id {
        // `undefined` variable declaration without init has same meaning
        if decl.init.is_none() && ident.name.as_str() == "undefined" {
          continue;
        }
      }
      check_binding_pattern(&decl.id, ctx);
    }
  }

  fn function(&mut self, node: &Function, ctx: &mut Context) {
    if let Some(id) = &node.id {
      if is_restricted(id.name.as_str()) {
        report(id.span, id.name.as_str(), ctx);
      }
    }

    for param in &node.params.items {
      check_binding_pattern(&param.pattern, ctx);
    }
  }

  fn arrow_function_expression(
    &mut self,
    node: &ArrowFunctionExpression,
    ctx: &mut Context,
  ) {
    for param in &node.params.items {
      check_binding_pattern(&param.pattern, ctx);
    }
  }

  fn catch_clause(&mut self, node: &CatchClause, ctx: &mut Context) {
    if let Some(param) = &node.param {
      check_binding_pattern(&param.pattern, ctx);
    }
  }

  fn assignment_expression(
    &mut self,
    node: &AssignmentExpression,
    ctx: &mut Context,
  ) {
    check_assignment_target(&node.left, ctx);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_shadow_restricted_names_valid() {
    assert_lint_ok! {
      NoShadowRestrictedNames,
      "function foo(bar){ var baz; }",
      "!function foo(bar){ var baz; }",
      "!function(bar){ var baz; }",
      "try {} catch(e) {}",
      "export default function() {}",
      "try {} catch {}",
      "var undefined;",
      "var undefined; doSomething(undefined);",
      "var undefined; var undefined;",
      "let undefined",
      "let [...foo] = []",
      "function bar (...rest) {}",
    };
  }

  #[test]
  fn no_shadow_restricted_names_invalid() {
    assert_lint_err! {
      NoShadowRestrictedNames,
      "function NaN(NaN) { var NaN; !function NaN(NaN) { try {} catch(NaN) {} }; }": [
        {
          line: 1,
          col: 9,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        },
        {
          line: 1,
          col: 13,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        },
        {
          line: 1,
          col: 24,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        },
        {
          line: 1,
          col: 39,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        },
        {
          line: 1,
          col: 43,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        },
        {
          line: 1,
          col: 63,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        }
      ],
      "function undefined(undefined) { !function undefined(undefined) { try {} catch(undefined) {} }; }": [
        {
          line: 1,
          col: 9,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        },
        {
          line: 1,
          col: 19,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        },
        {
          line: 1,
          col: 42,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        },
        {
          line: 1,
          col: 52,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        },
        {
          line: 1,
          col: 78,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        }
      ],
      "function Infinity(Infinity) { var Infinity; !function Infinity(Infinity) { try {} catch(Infinity) {} }; }": [
        {
          line: 1,
          col: 9,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "Infinity"),
        },
        {
          line: 1,
          col: 18,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "Infinity"),
        },
        {
          line: 1,
          col: 34,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "Infinity"),
        },
        {
          line: 1,
          col: 54,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "Infinity"),
        },
        {
          line: 1,
          col: 63,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "Infinity"),
        },
        {
          line: 1,
          col: 88,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "Infinity"),
        }
      ],
      "function arguments(arguments) { var arguments; !function arguments(arguments) { try {} catch(arguments) {} }; }": [
        {
          line: 1,
          col: 9,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "arguments"),
        },
        {
          line: 1,
          col: 19,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "arguments"),
        },
        {
          line: 1,
          col: 36,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "arguments"),
        },
        {
          line: 1,
          col: 57,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "arguments"),
        },
        {
          line: 1,
          col: 67,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "arguments"),
        },
        {
          line: 1,
          col: 93,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "arguments"),
        }
      ],
      "function eval(eval) { var eval; !function eval(eval) { try {} catch(eval) {} }; }": [
        {
          line: 1,
          col: 9,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 14,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 26,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 42,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 47,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 68,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        }
      ],
      "var eval = (eval) => { var eval; !function eval(eval) { try {} catch(eval) {} }; }": [
        {
          line: 1,
          col: 4,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 12,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 27,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 43,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 48,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        },
        {
          line: 1,
          col: 69,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "eval"),
        }
      ],
      "var {undefined} = obj; var {a: undefined} = obj; var {a: {b: {undefined}}} = obj; var {a, ...undefined} = obj;": [
        {
          line: 1,
          col: 5,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        },
        {
          line: 1,
          col: 31,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        },
        {
          line: 1,
          col: 62,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        },
        {
          line: 1,
          col: 93,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        }
      ],
      "var [undefined] = [1]": [
        {
          col: 5,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        }
      ],
      "var undefined; undefined = 5;": [
        {
          col: 15,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        }
      ],
      "var [...undefined] = []": [
        {
          col: 8,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "undefined"),
        }
      ],
      "try {} catch { try{} catch(NaN) {} }": [
        {
          col: 27,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        }
      ],

      // nested assignment
      "f = () => { NaN = 42; };": [
        {
          col: 12,
          message: variant!(NoShadowRestrictedNamesMessage, Shadowing, "NaN"),
        }
      ],
    };
  }
}
