// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

use crate::ast_parser;
use crate::diagnostic::LintDiagnostic;
use crate::linter::LinterBuilder;
use crate::rules::LintRule;
use dprint_swc_ecma_ast_view::TokenAndSpan;
use std::marker::PhantomData;
use std::rc::Rc;
use swc_common::comments::SingleThreadedComments;
use swc_common::SourceMap;
use swc_ecmascript::ast::Program;
use swc_ecmascript::parser::{Syntax, TsConfig};

#[macro_export]
macro_rules! assert_lint_ok {
  ($rule:ty, $($test:tt),+ $(,)?) => {
    $(
      let (src, filename) = parse_ok_test!($test);
      $crate::test_util::assert_lint_ok::<$rule>(src, filename);
    )*
  };
}

#[macro_export]
macro_rules! assert_lint_err {
  (
    $rule:ty,
    $($src:literal : $test:tt),+
    $(,)?
  ) => {
    $(
      let (errors, filename) = parse_err_test!($test);
      let tester = $crate::test_util::LintErrTester::<$rule>::new(
        $src,
        errors,
        filename,
      );
      tester.run();
    )*
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

macro_rules! parse_ok_test {
  ($src:literal) => {{
    ($src, None)
  }};
  ({ src : $src:literal, filename : $filename:literal $(,)? }) => {{
    ($src, Some($filename))
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
    let filename = std::option::Option::<&str>::None;
    let mut errors = Vec::new();
    $(
      let mut builder = $crate::test_util::LintErrBuilder::new();
      $(
        builder.$field($value);
      )*
      let e = builder.build();
      errors.push(e);
    )*
    (errors, filename)
  }};
  (
    {
      filename : $filename:literal,
      errors : $errors:tt $(,)?
    }
  ) => {{
    let (errors, _) = parse_err_test!($errors);
    (errors, $filename)
  }};
}

#[derive(Default)]
pub struct LintErrTester<T: LintRule + 'static> {
  src: &'static str,
  errors: Vec<LintErr>,
  filename: String,
  rule: PhantomData<T>,
}

impl<T: LintRule + 'static> LintErrTester<T> {
  pub fn new(
    src: &'static str,
    errors: Vec<LintErr>,
    filename: Option<&str>,
  ) -> Self {
    Self {
      src,
      errors,
      filename: match filename {
        Some(f) => f.to_string(),
        None => "deno_lint_err_test.ts".to_string(),
      },
      rule: PhantomData,
    }
  }

  pub fn run(self) {
    let rule = T::new();
    let rule_code = rule.code();
    let diagnostics = lint(rule, self.src, self.filename);
    assert_eq!(
      self.errors.len(),
      diagnostics.len(),
      "{} diagnostics expected, but got {}.\n\nsource:\n{}\n",
      self.errors.len(),
      diagnostics.len(),
      self.src,
    );

    for (error, diagnostic) in self.errors.iter().zip(&diagnostics) {
      let LintErr {
        line,
        col,
        message,
        hint,
      } = error;
      assert_diagnostic_2(
        diagnostic,
        rule_code,
        *line,
        *col,
        self.src,
        message,
        hint.as_deref(),
      );
    }
  }
}

#[derive(Default)]
pub struct LintErr {
  pub line: usize,
  pub col: usize,
  pub message: String,
  pub hint: Option<String>,
}

#[derive(Default)]
pub struct LintErrBuilder {
  line: Option<usize>,
  col: Option<usize>,
  message: Option<String>,
  hint: Option<String>,
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

  pub fn build(self) -> LintErr {
    LintErr {
      line: self.line.unwrap_or(1),
      col: self.col.unwrap_or(0),
      message: self.message.unwrap_or_else(|| "".to_string()),
      hint: self.hint,
    }
  }
}

fn get_ts_config_with_tsx() -> Syntax {
  let ts_config = TsConfig {
    dynamic_import: true,
    decorators: true,
    tsx: true,
    ..Default::default()
  };
  Syntax::Typescript(ts_config)
}

fn lint(
  rule: Box<dyn LintRule>,
  source: &str,
  filename: String,
) -> Vec<LintDiagnostic> {
  let linter = LinterBuilder::default()
    .lint_unused_ignore_directives(false)
    .lint_unknown_rules(false)
    .syntax(if filename.ends_with(".tsx") {
      get_ts_config_with_tsx()
    } else {
      ast_parser::get_default_ts_config()
    })
    .rules(vec![rule])
    .build();

  let diagnostics = match linter.lint(filename, source.to_string()) {
    Ok((_, diagnostics)) => diagnostics,
    Err(e) => panic!(
      "Failed to lint.\n[cause]\n{}\n\n[source code]\n{}",
      e, source
    ),
  };
  diagnostics
}

pub fn assert_diagnostic(
  diagnostic: &LintDiagnostic,
  code: &str,
  line: usize,
  col: usize,
  source: &str,
) {
  if diagnostic.code == code
    && diagnostic.range.start.line == line
    && diagnostic.range.start.col == col
  {
    return;
  }
  panic!(
    "expect diagnostics {} at {}:{} to be {} at {}:{}\n\nsource:\n{}\n",
    diagnostic.code,
    diagnostic.range.start.line,
    diagnostic.range.start.col,
    code,
    line,
    col,
    source,
  );
}

fn assert_diagnostic_2(
  diagnostic: &LintDiagnostic,
  code: &str,
  line: usize,
  col: usize,
  source: &str,
  message: &str,
  hint: Option<&str>,
) {
  assert_eq!(
    code, diagnostic.code,
    "Rule code is expected to be \"{}\", but got \"{}\"\n\nsource:\n{}\n",
    code, diagnostic.code, source
  );
  assert_eq!(
    line, diagnostic.range.start.line,
    "Line is expected to be \"{}\", but got \"{}\"\n\nsource:\n{}\n",
    line, diagnostic.range.start.line, source
  );
  assert_eq!(
    col, diagnostic.range.start.col,
    "Column is expected to be \"{}\", but got \"{}\"\n\nsource:\n{}\n",
    col, diagnostic.range.start.col, source
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
}

pub fn assert_lint_ok<T: LintRule + 'static>(
  source: &str,
  filename: Option<&str>,
) {
  let rule = T::new();
  let filename = match filename {
    Some(f) => f.to_string(),
    None => "deno_lint_ok_test.ts".to_string(),
  };
  let diagnostics = lint(rule, source, filename);
  if !diagnostics.is_empty() {
    panic!(
      "Unexpected diagnostics found:\n{:#?}\n\nsource:\n{}\n",
      diagnostics, source
    );
  }
}

pub fn parse(
  source_code: &str,
) -> (
  Program,
  SingleThreadedComments,
  Rc<SourceMap>,
  Vec<TokenAndSpan>,
) {
  let ast_parser = ast_parser::AstParser::new();
  let syntax = ast_parser::get_default_ts_config();
  let ast_parser::ParsedData {
    program,
    comments,
    tokens,
  } = ast_parser
    .parse_program("lint_test.ts", syntax, source_code)
    .unwrap();
  let source_map = Rc::clone(&ast_parser.source_map);
  (program, comments, source_map, tokens)
}
