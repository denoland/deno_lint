// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::diagnostic::LintDiagnosticSeverity;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::tags;
use crate::tags::Tags;
use crate::Program;

use deno_ast::view as ast_view;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NodeBuiltinsSpecifier;

const CODE: &str = "node-builtin-specifier";
const MESSAGE: &str = "built-in Node modules need the \"node:\" specifier";
const HINT: &str = "Add \"node:\" prefix in front of the import specifier";
const FIX_DESC: &str = "Add \"node:\" prefix";

impl LintRule for NodeBuiltinsSpecifier {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NodeBuiltinsSpecifierGlobalHandler.traverse(program, context);
  }
}

struct NodeBuiltinsSpecifierGlobalHandler;

impl NodeBuiltinsSpecifierGlobalHandler {
  fn add_diagnostic(&self, ctx: &mut Context, src: &str, range: SourceRange) {
    let specifier = format!(r#""node:{}""#, src);

    let diagnostic_range = ctx.create_diagnostic_range(range);
    let details = ctx.create_diagnostic_details(
      CODE,
      MESSAGE,
      Some(HINT.to_string()),
      vec![LintFix {
        description: FIX_DESC.into(),
        changes: vec![LintFixChange {
          new_text: specifier.into(),
          range,
        }],
      }],
    );
    // This rule defaults to a warning rather than an error so that existing
    // code importing Node built-ins without the `node:` prefix keeps working.
    ctx.add_diagnostic_details_with_severity(
      Some(diagnostic_range),
      details,
      LintDiagnosticSeverity::Warning,
    );
  }
}

impl Handler for NodeBuiltinsSpecifierGlobalHandler {
  fn import_decl(&mut self, decl: &ast_view::ImportDecl, ctx: &mut Context) {
    let src = decl.src.value().to_string_lossy();
    if is_bare_node_builtin(&src) {
      self.add_diagnostic(ctx, &src, decl.src.range());
    }
  }

  fn call_expr(&mut self, expr: &ast_view::CallExpr, ctx: &mut Context) {
    if let ast_view::Callee::Import(_) = expr.callee {
      if let Some(src_expr) = expr.args.first() {
        if let ast_view::Expr::Lit(ast_view::Lit::Str(str_value)) =
          src_expr.expr
        {
          let src = str_value.value().to_string_lossy();
          if is_bare_node_builtin(&src) {
            self.add_diagnostic(ctx, &src, str_value.range());
          }
        }
      }
    }
  }
}

// Should match https://nodejs.org/api/module.html#modulebuiltinmodules
fn is_bare_node_builtin(src: &str) -> bool {
  matches!(
    src,
    "assert"
      | "assert/strict"
      | "async_hooks"
      | "buffer"
      | "child_process"
      | "cluster"
      | "console"
      | "constants"
      | "crypto"
      | "dgram"
      | "diagnostics_channel"
      | "dns"
      | "dns/promises"
      | "domain"
      | "events"
      | "fs"
      | "fs/promises"
      | "http"
      | "http2"
      | "https"
      | "inspector"
      | "inspector/promises"
      | "module"
      | "net"
      | "os"
      | "path"
      | "path/posix"
      | "path/win32"
      | "perf_hooks"
      | "process"
      | "punycode"
      | "querystring"
      | "readline"
      | "readline/promises"
      | "repl"
      | "stream"
      | "stream/consumers"
      | "stream/promises"
      | "stream/web"
      | "string_decoder"
      | "sys"
      | "timers"
      | "timers/promises"
      | "tls"
      | "trace_events"
      | "tty"
      | "url"
      | "util"
      | "util/types"
      | "v8"
      | "vm"
      | "wasi"
      | "worker_threads"
      | "zlib"
  )
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn node_specifier_valid() {
    assert_lint_ok! {
      NodeBuiltinsSpecifier,
      r#"import "node:path";"#,
      r#"import "node:fs";"#,
      r#"import "node:fs/promises";"#,

      r#"import * as fs from "node:fs";"#,
      r#"import * as fsPromises from "node:fs/promises";"#,
      r#"import fsPromises from "node:fs/promises";"#,

      r#"await import("node:fs");"#,
      r#"await import("node:fs/promises");"#,
    };
  }

  #[test]
  fn node_specifier_invalid() {
    assert_lint_err! {
      NodeBuiltinsSpecifier,
      MESSAGE,
      HINT,
      r#"import "path";"#: [
        {
          col: 7,
          fix: (FIX_DESC, r#"import "node:path";"#),
        }
      ],
      r#"import "fs";"#: [
        {
          col: 7,
          fix: (FIX_DESC, r#"import "node:fs";"#),
        }
      ],
      r#"import "fs/promises";"#: [
        {
          col: 7,
          fix: (FIX_DESC, r#"import "node:fs/promises";"#),
        }
      ],

      r#"import * as fs from "fs";"#: [
        {
          col: 20,
          fix: (FIX_DESC, r#"import * as fs from "node:fs";"#),
        }
      ],
      r#"import * as fsPromises from "fs/promises";"#: [
        {
          col: 28,
          fix: (FIX_DESC, r#"import * as fsPromises from "node:fs/promises";"#),
        }
      ],
      r#"import fsPromises from "fs/promises";"#: [
        {
          col: 23,
          fix: (FIX_DESC, r#"import fsPromises from "node:fs/promises";"#),
        }
      ],

      r#"await import("fs");"#: [
        {
          col: 13,
          fix: (FIX_DESC, r#"await import("node:fs");"#),
        }
      ],
      r#"await import("fs/promises");"#: [
        {
          col: 13,
          fix: (FIX_DESC, r#"await import("node:fs/promises");"#),
        }
      ]
    };
  }
}
