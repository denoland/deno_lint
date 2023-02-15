// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::{
  ArrowExpr, AssignExpr, CatchClause, Expr, FnDecl, FnExpr, Ident,
  ObjectPatProp, Pat, PatOrExpr, VarDecl,
};
use deno_ast::SourceRanged;
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoShadowRestrictedNames;

const CODE: &str = "no-shadow-restricted-names";

#[derive(Display)]
enum NoShadowRestrictedNamesMessage {
  #[display(fmt = "Shadowing of global property {}", _0)]
  Shadowing(String),
}

impl LintRule for NoShadowRestrictedNames {
  fn new() -> Arc<Self> {
    Arc::new(NoShadowRestrictedNames)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoShadowRestrictedNamesHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_shadow_restricted_names.md")
  }
}

struct NoShadowRestrictedNamesHandler;

fn is_restricted_names(ident: &Ident) -> bool {
  matches!(
    ident.sym().as_ref(),
    "undefined" | "NaN" | "Infinity" | "arguments" | "eval"
  )
}

fn check_pat(pat: &Pat, ctx: &mut Context) {
  match pat {
    Pat::Ident(ident) => {
      check_shadowing(ident.id, ctx);
    }
    Pat::Expr(Expr::Ident(ident)) => {
      check_shadowing(ident, ctx);
    }
    Pat::Array(array_pat) => {
      for el in &array_pat.elems {
        if let Some(pat) = el.as_ref() {
          check_pat(pat, ctx);
        }
      }
    }
    Pat::Object(object_pat) => {
      for prop in &object_pat.props {
        match prop {
          ObjectPatProp::Assign(assign) => {
            check_shadowing(assign.key, ctx);
          }
          ObjectPatProp::Rest(rest) => check_pat(&rest.arg, ctx),
          ObjectPatProp::KeyValue(key_value) => {
            check_pat(&key_value.value, ctx);
          }
        }
      }
    }
    Pat::Rest(rest_pat) => {
      check_pat(&rest_pat.arg, ctx);
    }
    _ => {}
  }
}

fn check_shadowing(ident: &Ident, ctx: &mut Context) {
  if is_restricted_names(ident) {
    report_shadowing(ident, ctx);
  }
}

fn report_shadowing(ident: &Ident, ctx: &mut Context) {
  ctx.add_diagnostic(
    ident.range(),
    CODE,
    NoShadowRestrictedNamesMessage::Shadowing(ident.sym().to_string()),
  );
}

impl Handler for NoShadowRestrictedNamesHandler {
  fn var_decl(&mut self, node: &VarDecl, ctx: &mut Context) {
    for decl in &node.decls {
      if let Pat::Ident(ident) = &decl.name {
        // `undefined` variable declaration without init is have same meaning
        if decl.init.is_none() && *ident.id.sym() == *"undefined" {
          continue;
        }
      }

      check_pat(&decl.name, ctx);
    }
  }

  fn fn_decl(&mut self, node: &FnDecl, ctx: &mut Context) {
    check_shadowing(node.ident, ctx);

    for param in &node.function.params {
      check_pat(&param.pat, ctx);
    }
  }

  fn fn_expr(&mut self, node: &FnExpr, ctx: &mut Context) {
    if let Some(ident) = node.ident.as_ref() {
      check_shadowing(ident, ctx)
    }

    for param in &node.function.params {
      check_pat(&param.pat, ctx);
    }
  }

  fn arrow_expr(&mut self, node: &ArrowExpr, ctx: &mut Context) {
    for param in &node.params {
      check_pat(param, ctx);
    }
  }

  fn catch_clause(&mut self, node: &CatchClause, ctx: &mut Context) {
    if let Some(param) = node.param.as_ref() {
      check_pat(param, ctx);
    }
  }

  fn assign_expr(&mut self, node: &AssignExpr, ctx: &mut Context) {
    if let PatOrExpr::Pat(pat) = &node.left {
      check_pat(pat, ctx);
    }
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
