// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::import_config::fix_with_deno_config_package;
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{Tags, RECOMMENDED};
use crate::Program;
use deno_ast::view::{CallExpr, Callee, Expr, ImportDecl, Lit};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct NoUnversionedImport;

const CODE: &str = "no-unversioned-import";
const MESSAGE: &str = "Missing version in specifier";
const HINT: &str = "Add a version requirement after the package name";
const FIX_DESCRIPTION: &str = "Use the versioned dependency from deno.json";

impl LintRule for NoUnversionedImport {
  fn tags(&self) -> Tags {
    &[RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoUnversionedImportHandler.traverse(program, context);
  }
}

struct NoUnversionedImportHandler;

impl Handler for NoUnversionedImportHandler {
  fn import_decl(&mut self, node: &ImportDecl, ctx: &mut Context) {
    let specifier_text = node.src.value().to_string_lossy();
    if is_unversioned(&specifier_text) {
      ctx.add_warning_with_fixes(
        node.src.range(),
        CODE,
        MESSAGE,
        Some(HINT.to_string()),
        fix_with_deno_config_package(
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
          if is_unversioned(&specifier_text) {
            ctx.add_warning_with_fixes(
              arg.range(),
              CODE,
              MESSAGE,
              Some(HINT.to_string()),
              fix_with_deno_config_package(
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

fn is_unversioned(s: &str) -> bool {
  if let Some(req_ref) = get_package_req_ref(s) {
    req_ref.req.version_req.version_text() == "*"
  } else {
    false
  }
}

fn get_package_req_ref(
  s: &str,
) -> Option<deno_semver::package::PackageReqReference> {
  if let Ok(req_ref) = deno_semver::npm::NpmPackageReqReference::from_str(s) {
    Some(req_ref.into_inner())
  } else if let Ok(req_ref) =
    deno_semver::jsr::JsrPackageReqReference::from_str(s)
  {
    Some(req_ref.into_inner())
  } else {
    None
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::diagnostic::LintDiagnosticSeverity;
  use crate::linter::{LintConfig, LintFileOptions, Linter, LinterOptions};
  use crate::rules::no_import_prefix::NoImportPrefix;
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
      "deno_lint_no_unversioned_import_{}_{}",
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

  fn lint_with_rules(
    source: &str,
    filename: &str,
    rules: Vec<Box<dyn crate::rules::LintRule>>,
    all_rule_codes: HashSet<Cow<'static, str>>,
  ) -> Vec<crate::diagnostic::LintDiagnostic> {
    let linter = Linter::new(LinterOptions {
      rules,
      all_rule_codes,
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
      NoUnversionedImport,
      r#"import foo from "foo";"#,
      r#"import foo from "@foo/bar";"#,
      r#"import foo from "./foo";"#,
      r#"import foo from "../foo";"#,
      r#"import foo from "~/foo";"#,
      r#"import foo from "npm:foo@1.2.3";"#,
      r#"import foo from "npm:foo@latest";"#,
      r#"import foo from "npm:foo@^1.2.3";"#,
      r#"import foo from "npm:@foo/bar@1.2.3";"#,
      r#"import foo from "npm:@foo/bar@^1.2.3";"#,
      r#"import foo from "jsr:@foo/bar@1.2.3";"#,
      r#"import foo from "jsr:@foo/bar@^1.2.3";"#,
      r#"import("foo")"#,
      r#"import("@foo/bar")"#,
      r#"import("./foo")"#,
      r#"import("../foo")"#,
      r#"import("~/foo")"#,
      r#"import("npm:foo@1.2.3")"#,
      r#"import("npm:foo@^1.2.3")"#,
      r#"import("npm:@foo/bar@1.2.3")"#,
      r#"import("npm:@foo/bar@^1.2.3")"#,
      r#"import("jsr:@foo/bar@1.2.3")"#,
      r#"import("jsr:@foo/bar@^1.2.3")"#,
    }
  }

  #[test]
  fn no_with_invalid() {
    assert_lint_err! {
      NoUnversionedImport,
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
      r#"import foo from "npm:@foo/bar";"#: [{
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
      r#"import("npm:@foo/bar");"#: [{
        col: 7,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning
      }],
    }
  }

  #[test]
  fn no_unversioned_import_is_autofixable_from_deno_json_imports() {
    let filename = filename_with_project(
      r#"{
  "imports": {
    "@std/assert": "jsr:@std/assert@^1.0.0"
  }
}"#,
      None,
    );

    assert_lint_err! {
      NoUnversionedImport,
      filename: filename,
      r#"import foo from "jsr:@std/assert";"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT,
        severity: LintDiagnosticSeverity::Warning,
        fix: (
          FIX_DESCRIPTION,
          r#"import foo from "@std/assert";"#,
        )
      }],
    }
  }

  #[test]
  fn no_unversioned_import_does_not_autofix_without_lockfile() {
    let filename = filename_with_project("{}\n", None);
    let diagnostics = lint_with_rules(
      r#"import foo from "jsr:@std/expect";"#,
      &filename,
      vec![Box::new(NoUnversionedImport)],
      HashSet::from([Cow::Borrowed(CODE)]),
    );
    assert!(diagnostics
      .iter()
      .all(|diagnostic| diagnostic.details.fixes.is_empty()));
  }

  #[test]
  fn no_unversioned_import_adds_missing_import_to_deno_json() {
    let filename = filename_with_project(
      "{}\n",
      Some(
        r#"{
  "version": "5",
  "specifiers": {
    "jsr:@std/expect@*": "jsr:@std/expect@1.0.0"
  }
}"#,
      ),
    );
    let source = r#"import foo from "jsr:@std/expect";"#;
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

    let diagnostics = lint_with_rules(
      source,
      &filename,
      vec![Box::new(NoUnversionedImport)],
      HashSet::from([Cow::Borrowed(CODE)]),
    );
    let updated = apply_first_fixes(
      &[
        (filename.as_str(), source),
        (config_specifier.as_str(), "{}\n"),
      ],
      &diagnostics,
    );

    assert_eq!(updated[&filename], r#"import foo from "@std/expect";"#);
    assert!(updated[&config_specifier]
      .contains("\"@std/expect\": \"jsr:@std/expect@1.0.0\""));
  }

  #[test]
  fn combined_warnings_apply_cleanly() {
    let filename = filename_with_project(
      "{}\n",
      Some(
        r#"{
  "version": "5",
  "specifiers": {
    "jsr:@std/expect@*": "jsr:@std/expect@1.0.0"
  }
}"#,
      ),
    );
    let source = r#"import foo from "jsr:@std/expect";"#;
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

    let diagnostics = lint_with_rules(
      source,
      &filename,
      vec![Box::new(NoImportPrefix), Box::new(NoUnversionedImport)],
      HashSet::from([Cow::Borrowed("no-import-prefix"), Cow::Borrowed(CODE)]),
    );

    assert_eq!(diagnostics.len(), 2);
    assert_eq!(diagnostics[0].severity(), LintDiagnosticSeverity::Warning);
    assert_eq!(diagnostics[1].severity(), LintDiagnosticSeverity::Warning);
    assert!(diagnostics
      .iter()
      .any(|diagnostic| diagnostic.details.fixes.is_empty()));
    assert!(diagnostics.iter().any(|diagnostic| {
      diagnostic
        .details
        .fixes
        .first()
        .is_some_and(|fix| fix.changes.len() == 2)
    }));

    let updated = apply_first_fixes(
      &[
        (filename.as_str(), source),
        (config_specifier.as_str(), "{}\n"),
      ],
      &diagnostics,
    );

    assert_eq!(updated[&filename], r#"import foo from "@std/expect";"#);
    assert!(updated[&config_specifier]
      .contains("\"@std/expect\": \"jsr:@std/expect@1.0.0\""));
  }
}
