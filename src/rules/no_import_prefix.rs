// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::import_config::fix_with_deno_config_import;
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{Tags, WORKSPACE};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, ImportDecl, Lit};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoImportPrefix;

const CODE: &str = "no-import-prefix";
const MESSAGE: &str =
  "Inline 'npm:', 'jsr:' or 'https:' dependency not allowed";
const HINT: &str = "Add it as a dependency in a deno.json or package.json instead and reference it here via its bare specifier";
const FIX_DESCRIPTION: &str = "Use the dependency declared in deno.json";

impl LintRule for NoImportPrefix {
  fn tags(&self) -> Tags {
    &[WORKSPACE]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoImportPrefixHandler.traverse(program, context);
  }
}

struct NoImportPrefixHandler;

impl Handler for NoImportPrefixHandler {
  fn import_decl(&mut self, node: &ImportDecl, ctx: &mut Context) {
    let specifier_text = node.src.value().to_string_lossy();
    if is_non_bare(&specifier_text) {
      ctx.add_warning_with_fixes(
        node.src.range(),
        CODE,
        MESSAGE,
        Some(HINT.to_string()),
        fix_with_deno_config_import(
          ctx,
          node.src.range(),
          &specifier_text,
          FIX_DESCRIPTION,
        )
        .into_iter()
        .collect(),
      );
    }
  }

  fn call_expr(&mut self, node: &CallExpr, ctx: &mut Context) {
    if let Callee::Import(_) = node.callee {
      if let Some(arg) = node.args.first() {
        if let Expr::Lit(Lit::Str(lit)) = arg.expr {
          let specifier_text = lit.value().to_string_lossy();
          if is_non_bare(&specifier_text) {
            ctx.add_warning_with_fixes(
              arg.range(),
              CODE,
              MESSAGE,
              Some(HINT.to_string()),
              fix_with_deno_config_import(
                ctx,
                arg.range(),
                &specifier_text,
                FIX_DESCRIPTION,
              )
              .into_iter()
              .collect(),
            );
          }
        }
      }
    }
  }
}

fn is_non_bare(s: &str) -> bool {
  s.starts_with("npm:")
    || s.starts_with("jsr:")
    || s.starts_with("http:")
    || s.starts_with("https:")
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostic::LintDiagnosticSeverity;
  use crate::linter::{LintConfig, LintFileOptions, Linter, LinterOptions};
  use crate::test_util::apply_first_fixes;
  use deno_ast::MediaType;
  use deno_ast::ModuleSpecifier;
  use std::borrow::Cow;
  use std::collections::HashSet;
  use std::sync::atomic::{AtomicUsize, Ordering};

  fn filename_with_project(
    config_text: &str,
    lockfile_text: Option<&str>,
  ) -> String {
    static NEXT_ID: AtomicUsize = AtomicUsize::new(1);

    let dir = std::env::temp_dir().join(format!(
      "deno_lint_no_import_prefix_{}_{}",
      std::process::id(),
      NEXT_ID.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("deno.json"), config_text).unwrap();
    if let Some(lockfile_text) = lockfile_text {
      std::fs::write(dir.join("deno.lock"), lockfile_text).unwrap();
    }
    ModuleSpecifier::from_file_path(dir.join("mod.ts"))
      .unwrap()
      .to_string()
  }

  fn lint_source(
    source: &str,
    filename: &str,
  ) -> Vec<crate::diagnostic::LintDiagnostic> {
    let linter = Linter::new(LinterOptions {
      rules: vec![Box::new(NoImportPrefix)],
      all_rule_codes: HashSet::from([Cow::Borrowed(CODE)]),
      custom_ignore_file_directive: None,
      custom_ignore_diagnostic_directive: None,
    });
    linter
      .lint_file(LintFileOptions {
        specifier: ModuleSpecifier::parse(filename).unwrap(),
        source_code: source.to_string(),
        media_type: MediaType::TypeScript,
        config: LintConfig {
          default_jsx_factory: None,
          default_jsx_fragment_factory: None,
        },
        external_linter: None,
      })
      .unwrap()
      .1
  }

  #[test]
  fn no_with_valid() {
    assert_lint_ok! {
      NoImportPrefix,
      r#"import foo from "foo";"#,
      r#"import foo from "@foo/bar";"#,
      r#"import foo from "./foo";"#,
      r#"import foo from "../foo";"#,
      r#"import foo from "~/foo";"#,
      r#"import("foo")"#,
      r#"import("@foo/bar")"#,
      r#"import("./foo")"#,
      r#"import("../foo")"#,
      r#"import("~/foo")"#,
    }
  }

  #[test]
  fn no_with_invalid() {
    assert_lint_err! {
      NoImportPrefix,
      r#"import foo from "jsr:@foo/foo";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
      r#"import foo from "npm:foo";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
      r#"import foo from "http://example.com/foo";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
      r#"import foo from "https://example.com/foo";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
      r#"import("jsr:@foo/foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
      r#"import("npm:foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
      r#"import("http://example.com/foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
      r#"import("https://example.com/foo");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
    }
  }

  #[test]
  fn no_import_prefix_is_autofixable_from_deno_json_imports() {
    let filename = filename_with_project(
      r#"{
  "imports": {
    "@std/assert/": "jsr:@std/assert@^1.0.0/"
  }
}"#,
      None,
    );

    assert_lint_err! {
      NoImportPrefix,
      filename: filename,
      r#"import foo from "jsr:@std/assert@^1.0.0/fmt/colors.ts";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning,
        fix: (
          FIX_DESCRIPTION,
          r#"import foo from "@std/assert/fmt/colors.ts";"#,
        )
      }],
    }
  }

  #[test]
  fn no_import_prefix_does_not_autofix_without_lockfile() {
    let filename = filename_with_project("{}\n", None);
    let diagnostics =
      lint_source(r#"import foo from "jsr:@std/expect@^1";"#, &filename);
    assert!(diagnostics
      .iter()
      .all(|diagnostic| diagnostic.details.fixes.is_empty()));
  }

  #[test]
  fn no_import_prefix_adds_missing_import_to_deno_json() {
    let filename = filename_with_project(
      "{}\n",
      Some(
        r#"{
  "version": "5",
  "specifiers": {
    "jsr:@std/expect@^1": "jsr:@std/expect@1.0.0"
  }
}"#,
      ),
    );
    let source = r#"import foo from "jsr:@std/expect@^1";"#;
    let config_specifier = ModuleSpecifier::parse(&filename)
      .unwrap()
      .to_file_path()
      .unwrap()
      .parent()
      .unwrap()
      .join("deno.json");
    let config_specifier = ModuleSpecifier::from_file_path(config_specifier)
      .unwrap()
      .to_string();

    let diagnostics = lint_source(source, &filename);
    let updated = apply_first_fixes(
      &[
        (filename.as_str(), source),
        (config_specifier.as_str(), "{}\n"),
      ],
      &diagnostics,
    );

    assert_eq!(updated[&filename], r#"import foo from "@std/expect";"#);
    assert!(updated[&config_specifier].contains("\"imports\": {"));
    assert!(updated[&config_specifier]
      .contains("\"@std/expect\": \"jsr:@std/expect@^1\""));
  }
}
