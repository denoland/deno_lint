// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::diagnostic::LintDiagnostic;
use crate::diagnostic::Location;
use crate::rules::LintRule;
use crate::swc_util::AstParser;
use crate::swc_util::SwcDiagnosticBuffer;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::comments::CommentMap;
use swc_common::comments::Comments;
use swc_common::SourceMap;
use swc_common::Span;

#[derive(Clone)]
pub struct Context {
  pub file_name: String,
  pub diagnostics: Arc<Mutex<Vec<LintDiagnostic>>>,
  pub source_map: Arc<SourceMap>,
  pub leading_comments: CommentMap,
  pub trailing_comments: CommentMap,
  pub ignore_directives: Vec<IgnoreDirective>,
}

impl Context {
  pub fn create_diagnostic(
    &self,
    span: Span,
    code: &str,
    message: &str,
  ) -> LintDiagnostic {
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

    LintDiagnostic {
      location: location.into(),
      message: message.to_string(),
      code: code.to_string(),
      line_src,
      snippet_length,
    }
  }

  pub fn add_diagnostic(&self, span: Span, code: &str, message: &str) {
    let diagnostic = self.create_diagnostic(span, code, message);
    let mut diags = self.diagnostics.lock().unwrap();
    diags.push(diagnostic);
  }
}

#[derive(Clone, Debug)]
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
    if self.location.filename != diagnostic.location.filename {
      return false;
    }

    if self.location.line != diagnostic.location.line - 1 {
      return false;
    }

    let should_ignore = self.codes.contains(&diagnostic.code);

    if should_ignore {
      *self.used_codes.get_mut(&diagnostic.code).unwrap() = true;
    }

    should_ignore
  }
}

pub struct Linter {
  pub ast_parser: AstParser,
  pub ignore_file_directive: String,
  pub ignore_diagnostic_directives: Vec<String>,
  pub lint_unused_ignore_directives: bool,
}

impl Linter {
  pub fn default() -> Self {
    let ast_parser = AstParser::new();

    Linter {
      ast_parser,
      ignore_file_directive: "deno-lint-ignore-file".to_string(),
      ignore_diagnostic_directives: vec!["deno-lint-ignore".to_string()],
      lint_unused_ignore_directives: true,
    }
  }

  pub fn lint(
    &mut self,
    file_name: String,
    source_code: String,
    syntax: swc_ecma_parser::Syntax,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Result<Vec<LintDiagnostic>, SwcDiagnosticBuffer> {
    self.ast_parser.parse_module(
      &file_name,
      syntax,
      &source_code,
      |parse_result, comments| {
        let module = parse_result?;
        let diagnostics =
          self.lint_module(file_name.clone(), module, comments, rules);
        Ok(diagnostics)
      },
    )
  }

  fn has_ignore_file_directive(
    &self,
    comments: &Comments,
    module: &swc_ecma_ast::Module,
  ) -> bool {
    if let Some(module_leading_comments) =
      comments.take_leading_comments(module.span.lo())
    {
      for comment in module_leading_comments.iter() {
        if comment.kind == CommentKind::Line
          && comment.text.trim() == self.ignore_file_directive
        {
          return true;
        }
      }
      comments.add_leading(module.span.lo(), module_leading_comments);
    }

    false
  }

  fn parse_ignore_comment(&self, comment: &Comment) -> Option<IgnoreDirective> {
    if comment.kind != CommentKind::Line {
      return None;
    }

    let comment_text = comment.text.trim();

    for ignore_dir in &self.ignore_diagnostic_directives {
      if comment_text.starts_with(ignore_dir) {
        let codes = comment_text
          .split(' ')
          // TODO(bartlomieju): this is make-shift, make it configurable
          .map(|s| s.trim_start_matches("@typescript-eslint/"))
          .map(String::from)
          .skip(1)
          .collect::<Vec<String>>();

        let location = self
          .ast_parser
          .source_map
          .lookup_char_pos(comment.span.lo());

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

    return None;
  }

  fn parse_ignore_directives(
    &self,
    leading: &CommentMap,
  ) -> Vec<IgnoreDirective> {
    let mut ignore_directives = vec![];

    leading.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
        if let Some(ignore) = self.parse_ignore_comment(comment) {
          ignore_directives.push(ignore);
        }
      }
    });

    ignore_directives
  }

  fn filter_diagnostics(&self, context: Context) -> Vec<LintDiagnostic> {
    let mut ignore_directives = context.ignore_directives.clone();
    let diagnostics = context.diagnostics.lock().unwrap();

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

    if self.lint_unused_ignore_directives {
      for ignore_directive in ignore_directives {
        for (code, used) in ignore_directive.used_codes.iter() {
          if !used {
            let diagnostic = context.create_diagnostic(
              ignore_directive.span,
              "ban-unused-ignore",
              &format!("Ignore for code \"{}\" was not used.", code),
            );
            filtered_diagnostics.push(diagnostic);
          }
        }
      }
    }

    filtered_diagnostics.sort_by(|a, b| a.location.line.cmp(&b.location.line));

    filtered_diagnostics
  }

  fn lint_module(
    &self,
    file_name: String,
    module: swc_ecma_ast::Module,
    comments: Comments,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Vec<LintDiagnostic> {
    if self.has_ignore_file_directive(&comments, &module) {
      return vec![];
    }

    let (leading, trailing) = comments.take_all();

    let ignore_directives = self.parse_ignore_directives(&leading);

    let context = Context {
      file_name,
      diagnostics: Arc::new(Mutex::new(vec![])),
      source_map: self.ast_parser.source_map.clone(),
      leading_comments: leading,
      trailing_comments: trailing,
      ignore_directives,
    };

    for rule in rules {
      rule.lint_module(context.clone(), module.clone());
    }

    self.filter_diagnostics(context)
  }
}
