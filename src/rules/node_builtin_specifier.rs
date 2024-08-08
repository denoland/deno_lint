// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::handler::Traverse;
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
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
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

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/node_builtin_specifier.md")
  }
}

struct NodeBuiltinsSpecifierGlobalHandler;

impl NodeBuiltinsSpecifierGlobalHandler {
  fn add_diagnostic(&self, ctx: &mut Context, src: &str, range: SourceRange) {
    let specifier = format!(r#""node:{}""#, src);

    ctx.add_diagnostic_with_fixes(
      range,
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
  }
}

impl Handler for NodeBuiltinsSpecifierGlobalHandler {
  fn import_decl(&mut self, decl: &ast_view::ImportDecl, ctx: &mut Context) {
    let src = decl.src.inner.value.as_str();
    if is_bare_node_builtin(&src) {
      self.add_diagnostic(ctx, &src, decl.src.range());
    }
  }

  fn call_expr(&mut self, expr: &ast_view::CallExpr, ctx: &mut Context) {
    match expr.callee {
      ast_view::Callee::Import(_) => {
        if let Some(src_expr) = expr.args.first() {
          match src_expr.expr {
            ast_view::Expr::Lit(lit) => match lit {
              ast_view::Lit::Str(str_value) => {
                let src = str_value.inner.value.as_str();
                if is_bare_node_builtin(&src) {
                  self.add_diagnostic(ctx, &src, lit.range());
                }
              }
              _ => {}
            },
            _ => {}
          }
        }
      }
      _ => {}
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
