// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::diagnostic::LintDiagnostic;
use crate::diagnostic::Location;
use crate::rules::LintRule;
use crate::scopes::{analyze, Scope};
use crate::swc_util::get_default_ts_config;
use crate::swc_util::AstParser;
use crate::swc_util::SwcDiagnosticBuffer;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::Mutex;
use std::time::Instant;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::comments::SingleThreadedComments;
use swc_common::BytePos;
use swc_common::SourceMap;
use swc_common::Span;
use swc_ecmascript::parser::Syntax;

lazy_static! {
  static ref IGNORE_COMMENT_CODE_RE: regex::Regex =
    regex::Regex::new(r",\s*|\s").unwrap();
}

#[derive(Clone)]
pub struct Context {
  pub file_name: String,
  pub diagnostics: Arc<Mutex<Vec<LintDiagnostic>>>,
  pub source_map: Arc<SourceMap>,
  pub leading_comments: HashMap<BytePos, Vec<Comment>>,
  pub trailing_comments: HashMap<BytePos, Vec<Comment>>,
  pub ignore_directives: Vec<IgnoreDirective>,
  /// Arc as it's not modified
  pub scope: Arc<Scope>,
}

impl Context {
  pub fn create_diagnostic(
    &self,
    span: Span,
    code: &str,
    message: &str,
  ) -> LintDiagnostic {
    let start = Instant::now();
    let location = self.source_map.lookup_char_pos(span.lo());
    let line_src = self
      .source_map
      .lookup_source_file(span.lo())
      .get_line(location.line - 1)
      .expect("error loading line soruce")
      .to_string();

    let snippet_length = self
      .source_map
      .span_to_snippet(self.source_map.span_until_char(span, '\n'))
      .expect("error loading snippet")
      .len();

    let diagnostic = LintDiagnostic {
      location: location.into(),
      filename: self.file_name.clone(),
      message: message.to_string(),
      code: code.to_string(),
      line_src,
      snippet_length,
    };

    let end = Instant::now();
    debug!("Context::create_diagnostic took {:?}", end - start);
    diagnostic
  }

  pub fn add_diagnostic(&self, span: Span, code: &str, message: &str) {
    let diagnostic = self.create_diagnostic(span, code, message);
    let mut diags = self.diagnostics.lock().unwrap();
    diags.push(diagnostic);
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct IgnoreDirective {
  pub location: Location,
  pub span: Span,
  pub codes: Vec<String>,
  pub used_codes: HashMap<String, bool>,
}

impl IgnoreDirective {
  /// Check if `IgnoreDirective` supresses given `diagnostic` and if so
  /// mark the directive as used
  pub fn maybe_ignore_diagnostic(
    &mut self,
    diagnostic: &LintDiagnostic,
  ) -> bool {
    if self.location.line != diagnostic.location.line - 1 {
      return false;
    }

    let mut should_ignore = false;
    for code in self.codes.iter() {
      // `ends_with` allows to skip `@typescript-eslint` prefix - not ideal
      // but works for now
      if code.ends_with(&diagnostic.code) {
        should_ignore = true;
        *self.used_codes.get_mut(code).unwrap() = true;
      }
    }

    should_ignore
  }
}

pub struct LinterBuilder {
  ignore_file_directives: Vec<String>,
  ignore_diagnostic_directives: Vec<String>,
  lint_unused_ignore_directives: bool,
  lint_unknown_rules: bool,
  syntax: swc_ecmascript::parser::Syntax,
  rules: Vec<Box<dyn LintRule>>,
}

impl LinterBuilder {
  pub fn default() -> Self {
    Self {
      ignore_file_directives: vec!["deno-lint-ignore-file".to_string()],
      ignore_diagnostic_directives: vec!["deno-lint-ignore".to_string()],
      lint_unused_ignore_directives: true,
      lint_unknown_rules: true,
      syntax: get_default_ts_config(),
      rules: vec![],
    }
  }

  pub fn build(self) -> Linter {
    Linter::new(
      self.ignore_file_directives,
      self.ignore_diagnostic_directives,
      self.lint_unused_ignore_directives,
      self.lint_unknown_rules,
      self.syntax,
      self.rules,
    )
  }

  pub fn ignore_file_directives(mut self, directives: Vec<&str>) -> Self {
    self.ignore_file_directives =
      directives.iter().map(|s| s.to_string()).collect();
    self
  }

  pub fn ignore_diagnostic_directives(mut self, directives: Vec<&str>) -> Self {
    self.ignore_diagnostic_directives =
      directives.iter().map(|s| s.to_string()).collect();
    self
  }

  pub fn lint_unused_ignore_directives(
    mut self,
    lint_unused_ignore_directives: bool,
  ) -> Self {
    self.lint_unused_ignore_directives = lint_unused_ignore_directives;
    self
  }

  pub fn lint_unknown_rules(mut self, lint_unknown_rules: bool) -> Self {
    self.lint_unknown_rules = lint_unknown_rules;
    self
  }

  pub fn syntax(mut self, syntax: Syntax) -> Self {
    self.syntax = syntax;
    self
  }

  pub fn rules(mut self, rules: Vec<Box<dyn LintRule>>) -> Self {
    self.rules = rules;
    self
  }
}

pub struct Linter {
  has_linted: bool,
  ast_parser: AstParser,
  ignore_file_directives: Vec<String>,
  ignore_diagnostic_directives: Vec<String>,
  lint_unused_ignore_directives: bool,
  lint_unknown_rules: bool,
  syntax: Syntax,
  rules: Vec<Box<dyn LintRule>>,
}

impl Linter {
  fn new(
    ignore_file_directives: Vec<String>,
    ignore_diagnostic_directives: Vec<String>,
    lint_unused_ignore_directives: bool,
    lint_unknown_rules: bool,
    syntax: Syntax,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Self {
    Linter {
      has_linted: false,
      ast_parser: AstParser::new(),
      ignore_file_directives,
      ignore_diagnostic_directives,
      lint_unused_ignore_directives,
      lint_unknown_rules,
      syntax,
      rules,
    }
  }

  pub fn lint(
    &mut self,
    file_name: String,
    source_code: String,
  ) -> Result<Vec<LintDiagnostic>, SwcDiagnosticBuffer> {
    assert!(
      !self.has_linted,
      "Linter can be used only on a single module."
    );
    self.has_linted = true;
    let start = Instant::now();
    let (parse_result, comments) =
      self
        .ast_parser
        .parse_module(&file_name, self.syntax, &source_code);
    let end_parse_module = Instant::now();
    debug!(
      "ast_parser.parse_module took {:#?}",
      end_parse_module - start
    );
    let module = parse_result?;
    let diagnostics = self.lint_module(file_name, module, comments);
    let end = Instant::now();
    debug!("Linter::lint took {:#?}", end - start);
    Ok(diagnostics)
  }

  fn has_ignore_file_directive(
    &self,
    comments: &SingleThreadedComments,
    module: &swc_ecmascript::ast::Module,
  ) -> bool {
    comments.with_leading(module.span.lo(), |module_leading_comments| {
      for comment in module_leading_comments.iter() {
        if comment.kind == CommentKind::Line {
          let text = comment.text.trim();
          if self
            .ignore_file_directives
            .iter()
            .any(|directive| directive == text)
          {
            return true;
          }
        }
      }
      false
    })
  }

  fn filter_diagnostics(
    &self,
    context: Arc<Context>,
    rules: &[Box<dyn LintRule>],
  ) -> Vec<LintDiagnostic> {
    let start = Instant::now();
    let mut ignore_directives = context.ignore_directives.clone();
    let diagnostics = context.diagnostics.lock().unwrap();

    let rule_codes = rules
      .iter()
      .map(|r| r.code().to_string())
      .collect::<Vec<String>>();

    let mut filtered_diagnostics: Vec<LintDiagnostic> = diagnostics
      .as_slice()
      .iter()
      .cloned()
      .filter(|diagnostic| {
        !ignore_directives.iter_mut().any(|ignore_directive| {
          ignore_directive.maybe_ignore_diagnostic(&diagnostic)
        })
      })
      .collect();

    if self.lint_unused_ignore_directives || self.lint_unknown_rules {
      for ignore_directive in ignore_directives {
        for (code, used) in ignore_directive.used_codes.iter() {
          if self.lint_unused_ignore_directives
            && !used
            && rule_codes.contains(code)
          {
            let diagnostic = context.create_diagnostic(
              ignore_directive.span,
              "ban-unused-ignore",
              &format!("Ignore for code \"{}\" was not used.", code),
            );
            filtered_diagnostics.push(diagnostic);
          }

          if self.lint_unknown_rules && !rule_codes.contains(code) {
            filtered_diagnostics.push(context.create_diagnostic(
              ignore_directive.span,
              "ban-unknown-rule-code",
              &format!("Unknown rule for code \"{}\"", code),
            ))
          }
        }
      }
    }

    filtered_diagnostics.sort_by(|a, b| a.location.line.cmp(&b.location.line));

    let end = Instant::now();
    debug!("Linter::filter_diagnostics took {:#?}", end - start);

    filtered_diagnostics
  }

  fn lint_module(
    &self,
    file_name: String,
    module: swc_ecmascript::ast::Module,
    comments: SingleThreadedComments,
  ) -> Vec<LintDiagnostic> {
    if self.has_ignore_file_directive(&comments, &module) {
      return vec![];
    }
    let start = Instant::now();

    let (leading, trailing) = comments.take_all();
    let leading_coms = Rc::try_unwrap(leading)
      .expect("Failed to get leading comments")
      .into_inner();
    let leading = leading_coms.into_iter().collect();
    let trailing_coms = Rc::try_unwrap(trailing)
      .expect("Failed to get leading comments")
      .into_inner();
    let trailing = trailing_coms.into_iter().collect();

    let ignore_directives = parse_ignore_directives(
      &self.ignore_diagnostic_directives,
      &self.ast_parser.source_map,
      &leading,
    );

    let scope = Arc::new(analyze(&module));

    let context = Arc::new(Context {
      file_name,
      diagnostics: Arc::new(Mutex::new(vec![])),
      source_map: self.ast_parser.source_map.clone(),
      leading_comments: leading,
      trailing_comments: trailing,
      ignore_directives,
      scope,
    });

    for rule in &self.rules {
      rule.lint_module(context.clone(), &module);
    }

    let d = self.filter_diagnostics(context, &self.rules);
    let end = Instant::now();
    debug!("Linter::lint_module took {:#?}", end - start);

    d
  }
}

fn parse_ignore_directives(
  ignore_diagnostic_directives: &[String],
  source_map: &SourceMap,
  comments: &HashMap<BytePos, Vec<Comment>>,
) -> Vec<IgnoreDirective> {
  let mut ignore_directives = vec![];

  comments.values().for_each(|comments| {
    for comment in comments {
      if let Some(ignore) =
        parse_ignore_comment(&ignore_diagnostic_directives, source_map, comment)
      {
        ignore_directives.push(ignore);
      }
    }
  });

  ignore_directives
    .sort_by(|a, b| a.location.line.partial_cmp(&b.location.line).unwrap());
  ignore_directives
}

fn parse_ignore_comment(
  ignore_diagnostic_directives: &[String],
  source_map: &SourceMap,
  comment: &Comment,
) -> Option<IgnoreDirective> {
  if comment.kind != CommentKind::Line {
    return None;
  }

  let comment_text = comment.text.trim();

  for ignore_dir in ignore_diagnostic_directives {
    if comment_text.starts_with(ignore_dir) {
      let comment_text = comment_text.strip_prefix(ignore_dir).unwrap();
      let comment_text = IGNORE_COMMENT_CODE_RE.replace_all(comment_text, ",");
      let codes = comment_text
        .split(',')
        .filter(|code| !code.is_empty())
        .map(|code| String::from(code.trim()))
        .collect::<Vec<String>>();

      let location = source_map.lookup_char_pos(comment.span.lo());

      let mut used_codes = HashMap::new();
      codes.iter().for_each(|code| {
        used_codes.insert(code.to_string(), false);
      });

      return Some(IgnoreDirective {
        location: location.into(),
        span: comment.span,
        codes,
        used_codes,
      });
    }
  }

  None
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::swc_util;

  #[test]
  fn test_parse_ignore_comments() {
    let source_code = r#"
// deno-lint-ignore no-explicit-any no-empty no-debugger
function foo(): any {}

// not-deno-lint-ignore no-explicit-any
function foo(): any {}

// deno-lint-ignore no-explicit-any, no-empty, no-debugger
function foo(): any {}

// deno-lint-ignore no-explicit-any,no-empty,no-debugger
function foo(): any {}
"#;
    let ast_parser = AstParser::new();
    let (parse_result, comments) = ast_parser.parse_module(
      "test.ts",
      swc_util::get_default_ts_config(),
      &source_code,
    );
    parse_result.expect("Failed to parse");
    let (leading, _) = comments.take_all();
    let leading_coms = Rc::try_unwrap(leading)
      .expect("Failed to get leading comments")
      .into_inner();
    let leading = leading_coms.into_iter().collect();
    let directives = parse_ignore_directives(
      &["deno-lint-ignore".to_string()],
      &ast_parser.source_map,
      &leading,
    );

    assert_eq!(directives.len(), 3);
    let d = &directives[0];
    assert_eq!(d.location, Location { line: 2, col: 0 });
    assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
    let d = &directives[1];
    assert_eq!(d.location, Location { line: 8, col: 0 });
    assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
    let d = &directives[2];
    assert_eq!(d.location, Location { line: 11, col: 0 });
    assert_eq!(d.codes, vec!["no-explicit-any", "no-empty", "no-debugger"]);
  }
}
