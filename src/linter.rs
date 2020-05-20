// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use swc_common;
use swc_common::comments::Comment;
use swc_common::comments::CommentKind;
use swc_common::comments::Comments;
use swc_common::errors::Diagnostic;
use swc_common::errors::DiagnosticBuilder;
use swc_common::errors::Emitter;
use swc_common::errors::Handler;
use swc_common::errors::HandlerFlags;
use swc_common::FileName;
use swc_common::SourceMap;
use swc_ecma_ast;
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::JscTarget;
use swc_ecma_parser::Parser;
use swc_ecma_parser::Session;
use swc_ecma_parser::SourceFileInput;
use swc_ecma_parser::Syntax;
use swc_ecma_parser::TsConfig;

use crate::rules;
use crate::rules::LintDiagnostic;
use crate::rules::LintRule;

pub type SwcDiagnostics = Vec<Diagnostic>;

#[derive(Clone, Default)]
pub(crate) struct BufferedError(Arc<RwLock<SwcDiagnostics>>);

impl Emitter for BufferedError {
  fn emit(&mut self, db: &DiagnosticBuilder) {
    self.0.write().unwrap().push((**db).clone());
  }
}

impl From<BufferedError> for Vec<Diagnostic> {
  fn from(buf: BufferedError) -> Self {
    let s = buf.0.read().unwrap();
    s.clone()
  }
}

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
  buffered_error: BufferedError,
  pub source_map: Arc<SourceMap>,
  pub handler: Handler,
  // After parsing module `comments` are taken from
  // `Linter` and passed to `Context`
  comments: Option<Comments>,
}

impl Linter {
  pub fn default() -> Self {
    let buffered_error = BufferedError::default();

    let handler = Handler::with_emitter_and_flags(
      Box::new(buffered_error.clone()),
      HandlerFlags {
        dont_buffer_diagnostics: true,
        can_emit_warnings: true,
        ..Default::default()
      },
    );

    Linter {
      buffered_error,
      source_map: Arc::new(SourceMap::default()),
      handler,
      comments: Some(Comments::default()),
    }
  }

  pub fn lint(
    &mut self,
    file_name: String,
    source_code: String,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Result<Vec<LintDiagnostic>, SwcDiagnostics> {
    swc_common::GLOBALS.set(&swc_common::Globals::new(), || {
      let swc_source_file = self
        .source_map
        .new_source_file(FileName::Custom(file_name.clone()), source_code);

      let buffered_err = self.buffered_error.clone();
      let session = Session {
        handler: &self.handler,
      };

      let mut ts_config = TsConfig::default();
      ts_config.dynamic_import = true;
      let syntax = Syntax::Typescript(ts_config);

      let lexer = Lexer::new(
        session,
        syntax,
        JscTarget::Es2019,
        SourceFileInput::from(&*swc_source_file),
        self.comments.as_ref(),
      );

      let mut parser = Parser::new_from(session, lexer);

      let module =
        parser
          .parse_module()
          .map_err(move |mut err: DiagnosticBuilder| {
            err.cancel();
            SwcDiagnostics::from(buffered_err)
          })?;

      Ok(self.lint_module(file_name, module, rules))
    })
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

    let location = self.source_map.lookup_char_pos(comment.span.lo());

    Some(IgnoreDirective {
      location: location.into(),
      codes,
    })
  }

  fn lint_module(
    &mut self,
    file_name: String,
    module: swc_ecma_ast::Module,
    rules: Vec<Box<dyn LintRule>>,
  ) -> Vec<LintDiagnostic> {
    let (leading, trailing) = self
      .comments
      .take()
      .expect("Comments already taken")
      .take_all();

    let context = rules::Context {
      file_name,
      diagnostics: Arc::new(Mutex::new(vec![])),
      source_map: self.source_map.clone(),
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
