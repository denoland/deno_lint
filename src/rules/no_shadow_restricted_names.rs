// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, DUMMY_NODE};
use crate::ProgramRef;
use deno_ast::swc::ast::{
  ArrowExpr, AssignExpr, CatchClause, Expr, FnDecl, FnExpr, Ident,
  ObjectPatProp, Pat, PatOrExpr, VarDecl,
};
use deno_ast::swc::visit::{noop_visit_type, Node, VisitAll, VisitAllWith};
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

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoShadowRestrictedNamesVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_shadow_restricted_names.md")
  }
}

struct NoShadowRestrictedNamesVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoShadowRestrictedNamesVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }

  fn is_restricted_names(&self, ident: &Ident) -> bool {
    matches!(
      ident.sym.as_ref(),
      "undefined" | "NaN" | "Infinity" | "arguments" | "eval"
    )
  }

  fn check_pat(&mut self, pat: &Pat) {
    match pat {
      Pat::Ident(ident) => {
        self.check_shadowing(&ident.id);
      }
      Pat::Expr(expr) => {
        if let Expr::Ident(ident) = expr.as_ref() {
          self.check_shadowing(ident);
        }
      }
      Pat::Array(array_pat) => {
        for el in &array_pat.elems {
          if let Some(pat) = el.as_ref() {
            self.check_pat(pat);
          }
        }
      }
      Pat::Object(object_pat) => {
        for prop in &object_pat.props {
          match prop {
            ObjectPatProp::Assign(assign) => {
              self.check_shadowing(&assign.key);
            }
            ObjectPatProp::Rest(rest) => self.check_pat(&rest.arg),
            ObjectPatProp::KeyValue(key_value) => {
              self.check_pat(&key_value.value);
            }
          }
        }
      }
      Pat::Rest(rest_pat) => {
        self.check_pat(&rest_pat.arg);
      }
      _ => {}
    }
  }

  fn check_shadowing(&mut self, ident: &Ident) {
    if self.is_restricted_names(ident) {
      self.report_shadowing(ident);
    }
  }

  fn report_shadowing(&mut self, ident: &Ident) {
    self.context.add_diagnostic(
      ident.span,
      CODE,
      NoShadowRestrictedNamesMessage::Shadowing(ident.sym.to_string()),
    );
  }
}

impl<'c, 'view> VisitAll for NoShadowRestrictedNamesVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_var_decl(&mut self, node: &VarDecl, _: &dyn Node) {
    for decl in &node.decls {
      if let Pat::Ident(ident) = &decl.name {
        // `undefined` variable declaration without init is have same meaning
        if decl.init.is_none() && &ident.id.sym == "undefined" {
          continue;
        }
      }

      self.check_pat(&decl.name);
    }
  }

  fn visit_fn_decl(&mut self, node: &FnDecl, _: &dyn Node) {
    self.check_shadowing(&node.ident);

    for param in &node.function.params {
      self.check_pat(&param.pat);
    }
  }

  fn visit_fn_expr(&mut self, node: &FnExpr, _: &dyn Node) {
    if let Some(ident) = node.ident.as_ref() {
      self.check_shadowing(ident)
    }

    for param in &node.function.params {
      self.check_pat(&param.pat);
    }
  }

  fn visit_arrow_expr(&mut self, node: &ArrowExpr, _: &dyn Node) {
    for param in &node.params {
      self.check_pat(param);
    }
  }

  fn visit_catch_clause(&mut self, node: &CatchClause, _: &dyn Node) {
    if let Some(param) = node.param.as_ref() {
      self.check_pat(param);
    }
  }

  fn visit_assign_expr(&mut self, node: &AssignExpr, _: &dyn Node) {
    if let PatOrExpr::Pat(pat) = &node.left {
      self.check_pat(pat);
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
