// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::ast_parser;
use crate::diagnostic::LintDiagnostic;
use crate::linter::LintFileOptions;
use crate::linter::LinterBuilder;
use crate::rules::LintRule;
use deno_ast::diagnostics::Diagnostic;
use deno_ast::view as ast_view;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_ast::ParsedSource;
use deno_ast::SourceTextInfo;
use deno_ast::TextChange;

#[macro_export]
macro_rules! assert_lint_ok {
  (
    $rule:expr,
    filename: $filename:expr,
    $($src:literal),+
    $(,)?
  ) => {
    $(
      $crate::test_util::assert_lint_ok(&$rule, $src, $filename);
    )*
  };
  ($rule:expr, $($src:literal),+ $(,)?) => {
    assert_lint_ok! {
      $rule,
      filename: "file:///deno_lint_ok_test.ts",
      $($src,)*
    };
  };
}

#[macro_export]
macro_rules! assert_lint_err {
  (
    $rule:expr,
    filename: $filename:expr,
    $($src:literal : $test:tt),+
    $(,)?
  ) => {
    $(
      let errors = parse_err_test!($test);
      let tester = $crate::test_util::LintErrTester::new(
        &$rule,
        $src,
        errors,
        $filename,
      );
      tester.run();
    )*
  };
  (
    $rule:expr,
    $($src:literal : $test:tt),+
    $(,)?
  ) => {
    assert_lint_err! {
      $rule,
      filename: "file:///deno_lint_err_test.ts",
      $($src: $test,)*
    }
  };

  (
    $rule: expr,
    $message: expr,
    $hint: expr,
    filename: $filename:expr,
    $($src:literal : $test:tt),+
    $(,)?
  ) => {
    $(
      let errors = parse_err_test!($message, $hint, $test);
      let tester = $crate::test_util::LintErrTester::new(
        &$rule,
        $src,
        errors,
        $filename,
      );
      tester.run();
    )*
  };
  (
    $rule: expr,
    $message: expr,
    $hint: expr,
    $($src:literal : $test:tt),+
    $(,)?
  ) => {
    assert_lint_err! {
      $rule,
      $message,
      $hint,
      filename: "file:///deno_lint_err_test.ts",
      $($src: $test,)*
    }
  };
}

#[macro_export]
macro_rules! variant {
  ($enum:ident, $variant:ident) => {{
    $enum::$variant
  }};
  ($enum:ident, $variant:ident, $($value:expr),* $(,)?) => {{
    $enum::$variant(
      $(
        $value.to_string(),
      )*
    )
  }};
}

macro_rules! parse_err_test {
  (
    [
      $(
        {
          $($field:ident : $value:expr),* $(,)?
        }
      ),* $(,)?
    ]
  ) => {{
    let mut errors = Vec::new();
    $(
      let mut builder = $crate::test_util::LintErrBuilder::new();
      $(
        builder.$field($value);
      )*
      let e = builder.build();
      errors.push(e);
    )*
    errors
  }};

  (
    {
      filename : $filename:expr,
      errors : $errors:tt $(,)?
    }
  ) => {{
    let (errors, _) = parse_err_test!($errors);
    (errors, $filename)
  }};

  (
    $message: expr,
    $hint: expr,
    [
      $(
        {
          $($field:ident : $value:expr),* $(,)?
        }
      ),* $(,)?
    ]
  ) => {{
    let errors = parse_err_test!(
      $(
        [
          {
            message: $message,
            hint: $hint,
            $(
              $field: $value,
            )*
          },
        ]
      )*
    );
    errors
  }};
}

pub struct LintErrTester {
  src: &'static str,
  errors: Vec<LintErr>,
  filename: &'static str,
  rule: &'static dyn LintRule,
}

impl LintErrTester {
  pub fn new(
    rule: &'static dyn LintRule,
    src: &'static str,
    errors: Vec<LintErr>,
    filename: &'static str,
  ) -> Self {
    Self {
      src,
      errors,
      filename,
      rule,
    }
  }

  #[track_caller]
  pub fn run(self) {
    let rule_code = self.rule.code();
    let (parsed_source, diagnostics) = lint(self.rule, self.src, self.filename);
    if self.errors.len() != diagnostics.len() {
      eprintln!(
        "Actual diagnostics:\n{:#?}",
        diagnostics
          .iter()
          .map(|d| d.message.to_string())
          .collect::<Vec<_>>()
      );
      assert_eq!(
        self.errors.len(),
        diagnostics.len(),
        "{} diagnostics expected, but got {}.\n\nsource:\n{}\n",
        self.errors.len(),
        diagnostics.len(),
        self.src,
      );
    }

    for (error, diagnostic) in self.errors.iter().zip(&diagnostics) {
      let LintErr {
        line,
        col,
        message,
        hint,
        fixes,
      } = error;
      assert_diagnostic_2(
        diagnostic,
        rule_code,
        *line,
        *col,
        self.src,
        message,
        hint.as_deref(),
        fixes,
        parsed_source.text_info(),
      );
    }
  }
}

#[derive(Debug, PartialEq, Eq)]
pub struct LintErrFix {
  pub description: String,
  pub fixed_code: String,
}

#[derive(Default)]
pub struct LintErr {
  pub line: usize,
  pub col: usize,
  pub message: String,
  pub hint: Option<String>,
  pub fixes: Vec<LintErrFix>,
}

#[derive(Default)]
pub struct LintErrBuilder {
  line: Option<usize>,
  col: Option<usize>,
  message: Option<String>,
  hint: Option<String>,
  fixes: Vec<LintErrFix>,
}

impl LintErrBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn line(&mut self, line: usize) -> &mut Self {
    // Line is 1-based in deno_lint
    assert!(line >= 1);
    self.line = Some(line);
    self
  }

  pub fn col(&mut self, col: usize) -> &mut Self {
    self.col = Some(col);
    self
  }

  pub fn message(&mut self, message: impl ToString) -> &mut Self {
    self.message = Some(message.to_string());
    self
  }

  pub fn hint(&mut self, hint: impl ToString) -> &mut Self {
    self.hint = Some(hint.to_string());
    self
  }

  pub fn fix(&mut self, value: (&'static str, &'static str)) -> &mut Self {
    self.fixes.push(LintErrFix {
      description: value.0.to_string(),
      fixed_code: value.1.to_string(),
    });
    self
  }

  pub fn build(self) -> LintErr {
    LintErr {
      line: self.line.unwrap_or(1),
      col: self.col.unwrap_or(0),
      message: self.message.unwrap_or_default(),
      hint: self.hint,
      fixes: self.fixes,
    }
  }
}

#[track_caller]
fn lint(
  rule: &'static dyn LintRule,
  source: &str,
  specifier: &str,
) -> (ParsedSource, Vec<LintDiagnostic>) {
  let linter = LinterBuilder::default().rules(vec![rule]).build();

  let specifier = ModuleSpecifier::parse(specifier).unwrap();
  let media_type = MediaType::from_specifier(&specifier);
  let lint_result = linter.lint_file(LintFileOptions {
    specifier,
    source_code: source.to_string(),
    media_type,
  });
  match lint_result {
    Ok((source, diagnostics)) => (source, diagnostics),
    Err(e) => panic!(
      "Failed to lint.\n[cause]\n{}\n\n[source code]\n{}",
      e, source
    ),
  }
}

pub fn assert_diagnostic(
  diagnostic: &LintDiagnostic,
  code: &str,
  line: usize,
  col: usize,
  source: &str,
) {
  let line_and_column = diagnostic
    .text_info
    .line_and_column_index(diagnostic.range.start);
  if diagnostic.code == code
    // todo(dsherret): we should change these to be consistent (ex. both 1-indexed)
    && line_and_column.line_index + 1 == line
    && line_and_column.column_index == col
  {
    return;
  }
  panic!(
    "expect diagnostics {} at {}:{} to be {} at {}:{}\n\nsource:\n{}\n",
    diagnostic.code,
    line_and_column.line_index + 1,
    line_and_column.column_index,
    code,
    line,
    col,
    source,
  );
}

#[allow(clippy::too_many_arguments)]
#[track_caller]
fn assert_diagnostic_2(
  diagnostic: &LintDiagnostic,
  code: &str,
  line: usize,
  col: usize,
  source: &str,
  message: &str,
  hint: Option<&str>,
  fixes: &[LintErrFix],
  text_info: &SourceTextInfo,
) {
  let line_and_column = diagnostic
    .text_info
    .line_and_column_index(diagnostic.range.start);
  assert_eq!(
    code, diagnostic.code,
    "Rule code is expected to be \"{}\", but got \"{}\"\n\nsource:\n{}\n",
    code, diagnostic.code, source
  );
  assert_eq!(
    line,
    line_and_column.line_index + 1,
    "Line is expected to be \"{}\", but got \"{}\"\n\nsource:\n{}\n",
    line,
    line_and_column.line_index + 1,
    source
  );
  assert_eq!(
    col, line_and_column.column_index,
    "Column is expected to be \"{}\", but got \"{}\"\n\nsource:\n{}\n",
    col, line_and_column.column_index, source
  );
  assert_eq!(
    message, &diagnostic.message,
    "Diagnostic message is expected to be \"{}\", but got \"{}\"\n\nsource:\n{}\n",
    message, &diagnostic.message, source
  );
  assert_eq!(
    hint,
    diagnostic.hint.as_deref(),
    "Diagnostic hint is expected to be \"{:?}\", but got \"{:?}\"\n\nsource:\n{}\n",
    hint,
    diagnostic.hint.as_deref(),
    source
  );
  let actual_fixes = diagnostic
    .fixes
    .iter()
    .map(|fix| LintErrFix {
      description: fix.description.to_string(),
      fixed_code: deno_ast::apply_text_changes(
        text_info.text_str(),
        fix
          .changes
          .iter()
          .map(|change| TextChange {
            range: change.range.as_byte_range(text_info.range().start),
            new_text: change.new_text.to_string(),
          })
          .collect(),
      ),
    })
    .collect::<Vec<_>>();
  assert_eq!(actual_fixes, fixes, "Quick fixes did not match.");
}

#[track_caller]
pub fn assert_lint_ok(
  rule: &'static dyn LintRule,
  source: &str,
  specifier: &'static str,
) {
  let (_parsed_source, diagnostics) = lint(rule, source, specifier);
  if !diagnostics.is_empty() {
    eprintln!("filename {:?}", specifier);
    panic!(
      "Unexpected diagnostics found:\n{:#?}\n\nsource:\n{}\n",
      diagnostics.iter().map(|d| d.message()).collect::<Vec<_>>(),
      source
    );
  }
}

/// Just run the specified lint on the source code to make sure it doesn't panic.
pub fn assert_lint_not_panic(rule: &'static dyn LintRule, source: &str) {
  let _result = lint(rule, source, TEST_FILE_NAME);
}

const TEST_FILE_NAME: &str = "file:///lint_test.ts";

pub fn parse(source_code: &str) -> ParsedSource {
  ast_parser::parse_program(
    ModuleSpecifier::parse(TEST_FILE_NAME).unwrap(),
    MediaType::TypeScript,
    source_code.to_string(),
  )
  .unwrap()
}

pub fn parse_and_then(source_code: &str, test: impl Fn(ast_view::Program)) {
  let parsed_source = parse(source_code);
  parsed_source.with_view(|pg| {
    test(pg);
  });
}
