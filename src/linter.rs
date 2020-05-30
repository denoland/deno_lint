// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::swc_util;
use crate::swc_util::AstParser;
use crate::swc_util::SwcDiagnosticBuffer;
use std::sync::Arc;
use std::sync::Mutex;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::comments::Comments;

use crate::rules;
use crate::rules::LintDiagnostic;
use crate::rules::LintRule;

#[derive(Debug)]
pub struct IgnoreDirective {
  location: rules::Location,
  codes: Vec<String>,
}

impl IgnoreDirective {
  pub fn should_ignore_diagnostic(&self, diagnostic: &LintDiagnostic) -> bool {
    if self.location.filename != diagnostic.location.filename {
      return false;
    }

    if self.location.line != diagnostic.location.line - 1 {
      return false;
    }

    self.codes.contains(&diagnostic.code)
  }
}

pub struct Linter {
  pub ast_parser: AstParser,
}

impl Linter {
  pub fn default() -> Self {
    let ast_parser = AstParser::new();

    Linter { ast_parser }
  }

  pub fn lint(
    &mut self,
    file_name: String,
    source_code: String,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Result<Vec<LintDiagnostic>, SwcDiagnosticBuffer> {
    let syntax = swc_util::get_default_ts_config();
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

  fn parse_ignore_comment(&self, comment: &Comment) -> Option<IgnoreDirective> {
    if comment.kind != CommentKind::Line {
      return None;
    }

    let comment_text = comment.text.trim();

    let codes: Vec<String> = if comment_text.starts_with("deno-lint-ignore") {
      comment_text
        .split(' ')
        .map(|e| e.to_string())
        .skip(1)
        .collect()
    } else {
      return None;
    };

    let location = self
      .ast_parser
      .source_map
      .lookup_char_pos(comment.span.lo());

    Some(IgnoreDirective {
      location: location.into(),
      codes,
    })
  }

  fn lint_module(
    &self,
    file_name: String,
    module: swc_ecma_ast::Module,
    comments: Comments,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Vec<LintDiagnostic> {
    let (leading, trailing) = comments.take_all();

    let context = rules::Context {
      file_name,
      diagnostics: Arc::new(Mutex::new(vec![])),
      source_map: self.ast_parser.source_map.clone(),
      leading_comments: leading,
      trailing_comments: trailing,
    };

    for rule in rules {
      rule.lint_module(context.clone(), module.clone());
    }

    let mut ignore_directives = vec![];

    context.leading_comments.iter().for_each(|ref_multi| {
      for comment in ref_multi.value() {
        if let Some(ignore) = self.parse_ignore_comment(comment) {
          ignore_directives.push(ignore);
        }
      }
    });

    let diags = context.diagnostics.lock().unwrap();

    let mut filtered_diagnostics: Vec<LintDiagnostic> = diags
      .as_slice()
      .iter()
      .cloned()
      .filter(|diagnostic| {
        !ignore_directives.iter().any(|ignore_directive| {
          ignore_directive.should_ignore_diagnostic(&diagnostic)
        })
      })
      .collect();

    filtered_diagnostics.sort_by(|a, b| a.location.line.cmp(&b.location.line));

    filtered_diagnostics
  }
}
