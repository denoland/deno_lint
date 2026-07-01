use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::rules::config::{RuleConfigError, RuleDef, RuleSeverity};
use crate::tags::Tags;
use crate::Program;

use deno_ast::swc::ast::Id;
use deno_ast::view as ast_view;
use deno_ast::SourceRanged;
use serde::Deserialize;
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct NoConsole {
  /// Console methods that are permitted, e.g. `["warn", "error"]`.
  allowed: Vec<String>,
}

const MESSAGE: &str = "`console` usage is not allowed.";
const CODE: &str = "no-console";

/// Options for `no-console`, mirroring eslint's `{ allow: string[] }`.
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct NoConsoleOptions {
  allow: Vec<String>,
}

fn configure(
  options: Option<&serde_json::Value>,
) -> Result<Box<dyn LintRule>, RuleConfigError> {
  let options: NoConsoleOptions = match options {
    None => NoConsoleOptions::default(),
    Some(value) => serde_json::from_value(value.clone()).map_err(|e| {
      RuleConfigError::InvalidOptions {
        code: CODE,
        message: e.to_string(),
      }
    })?,
  };
  Ok(Box::new(NoConsole {
    allowed: options.allow,
  }))
}

impl NoConsole {
  /// The rule *definition*: metadata plus the constructor used to build a
  /// configured instance. See [`crate::rules::config`].
  pub fn def() -> RuleDef {
    RuleDef {
      code: CODE,
      tags: &[],
      // Not a recommended rule, so off unless explicitly enabled.
      default_severity: RuleSeverity::Off,
      configure_options: configure,
    }
  }
}

impl LintRule for NoConsole {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoConsoleHandler {
      imported_console: HashSet::new(),
      allowed: &self.allowed,
    }
    .traverse(program, context);
  }
}

struct NoConsoleHandler<'a> {
  /// Bindings imported as the default export of `node:console`, e.g.
  /// `import console from "node:console";`. These refer to the same `console`
  /// object as the global, so usages should be flagged too.
  imported_console: HashSet<Id>,
  /// Permitted console methods (the `allow` option).
  allowed: &'a [String],
}

impl NoConsoleHandler<'_> {
  fn is_console(&self, ident: &ast_view::Ident, ctx: &mut Context) -> bool {
    let id = ident.inner.to_id();
    // `console` imported from `node:console` (any local name).
    if self.imported_console.contains(&id) {
      return true;
    }
    // The global `console`.
    ident.sym() == "console" && ctx.scope().is_global(&id)
  }

  /// Whether the accessed property is in the `allow` list, e.g. the `warn` in
  /// `console.warn(...)`. Only the simple `console.method` form is recognized;
  /// computed access (`console["warn"]`) is not, matching the common case.
  fn is_allowed_property(&self, prop: &ast_view::MemberProp) -> bool {
    if self.allowed.is_empty() {
      return false;
    }
    if let ast_view::MemberProp::Ident(ident) = prop {
      let name = ident.sym();
      return self.allowed.iter().any(|allowed| name == allowed.as_str());
    }
    false
  }
}

impl Handler for NoConsoleHandler<'_> {
  fn import_decl(&mut self, import: &ast_view::ImportDecl, _ctx: &mut Context) {
    if import.src.value().to_string_lossy() != "node:console" {
      return;
    }
    for specifier in import.specifiers {
      if let ast_view::ImportSpecifier::Default(default) = specifier {
        self.imported_console.insert(default.local.to_id());
      }
    }
  }

  fn member_expr(&mut self, expr: &ast_view::MemberExpr, ctx: &mut Context) {
    if expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use deno_ast::view::Expr;
    if let Expr::Ident(ident) = &expr.obj {
      if self.is_console(ident, ctx) && !self.is_allowed_property(&expr.prop) {
        ctx.add_diagnostic(ident.range(), CODE, MESSAGE);
      }
    }
  }

  fn expr_stmt(&mut self, expr: &ast_view::ExprStmt, ctx: &mut Context) {
    use deno_ast::view::Expr;
    if let Expr::Ident(ident) = &expr.expr {
      if self.is_console(ident, ctx) {
        ctx.add_diagnostic(ident.range(), CODE, MESSAGE);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn console_allowed() {
    assert_lint_ok!(
      NoConsole::default(),
      // ignored
      r"// deno-lint-ignore no-console\nconsole.error('Error message');",
      // not global
      r"const console = { log() {} } console.log('Error message');",
      // https://github.com/denoland/deno_lint/issues/1232
      "const x: { console: any } = { console: 21 }; x.console",
      // a `console` imported from somewhere other than `node:console`
      "import console from './my-console.ts';\nconsole.log('hi');",
      // a non-default import from `node:console` (e.g. the `Console` class)
      "import { Console } from 'node:console';\nconst c = new Console(process.stdout);",
    );
  }

  #[test]
  fn no_console_invalid() {
    // Test cases where console is present
    assert_lint_err!(
        NoConsole::default(),
        r#"console.log('Debug message');"#: [{
            col: 0,
            message: MESSAGE,
        }],
        r#"if (debug) { console.log('Debugging'); }"#: [{
            col: 13,
            message: MESSAGE,
        }],
        r#"function log() { console.log('Log'); }"#: [{
            col: 17,
            message: MESSAGE,
        }],
        r#"function log() { console.debug('Log'); }"#: [{
            col: 17,
            message: MESSAGE,
        }],
        r#"console;"#: [{
            col: 0,
            message: MESSAGE,
        }],
        r#"console.warn("test");"#: [{
            col: 0,
            message: MESSAGE,
        }],
        // https://github.com/denoland/deno_lint/issues/1316
        // `console` imported from `node:console`.
        "import console from \"node:console\";\nconsole.log(\"hi\");": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
        // imported under a different local name.
        "import myConsole from \"node:console\";\nmyConsole.log(\"hi\");": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
        // bare reference to the imported binding.
        "import console from \"node:console\";\nconsole;": [{
            line: 2,
            col: 0,
            message: MESSAGE,
        }],
    );
  }

  fn with_allow(methods: &[&str]) -> NoConsole {
    NoConsole {
      allowed: methods.iter().map(|s| s.to_string()).collect(),
    }
  }

  #[test]
  fn no_console_allow_option_valid() {
    // Methods listed in `allow` are permitted.
    assert_lint_ok!(
      with_allow(&["warn", "error"]),
      r#"console.warn("ok");"#,
      r#"console.error("ok");"#,
    );
  }

  #[test]
  fn no_console_allow_option_invalid() {
    // Methods not listed in `allow` are still flagged.
    assert_lint_err!(
      with_allow(&["warn", "error"]),
      r#"console.log("nope");"#: [{
        col: 0,
        message: MESSAGE,
      }],
      // bare `console` has no method, so `allow` doesn't apply.
      r#"console;"#: [{
        col: 0,
        message: MESSAGE,
      }]
    );
  }

  #[test]
  fn no_console_configure_parses_allow() {
    let default_rule = (NoConsole::def().configure_options)(None).unwrap();
    assert_eq!(default_rule.code(), "no-console");

    let configured = (NoConsole::def().configure_options)(Some(
      &serde_json::json!({ "allow": ["warn"] }),
    ))
    .unwrap();
    assert_eq!(configured.code(), "no-console");
  }
}
