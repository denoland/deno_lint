// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::RwLock;
use swc_common;
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
use swc_ecma_visit;

mod rules;
mod traverse;

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

pub struct Linter {
  buffered_error: BufferedError,
  pub source_map: Arc<SourceMap>,
  pub handler: Handler,
  // After parsing module `comments` are taken from
  // `Linter` as passed to `Context`
  comments: Option<Comments>,
}

impl Linter {
  pub fn new() -> Self {
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
  ) -> Result<(), SwcDiagnostics> {
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

      self.lint_module(file_name, module);
      Ok(())
    })
  }

  fn lint_module(&mut self, file_name: String, module: swc_ecma_ast::Module) {
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

    let rules: Vec<Box<dyn swc_ecma_visit::Visit>> = vec![
      Box::new(rules::NoExplicitAny::new(context.clone())),
      Box::new(rules::NoDebugger::new(context.clone())),
      Box::new(rules::NoVar::new(context.clone())),
      Box::new(rules::SingleVarDeclarator::new(context.clone())),
      Box::new(rules::ExplicitFunctionReturnType::new(context.clone())),
      Box::new(rules::NoEval::new(context.clone())),
      Box::new(rules::NoEmptyInterface::new(context.clone())),
      Box::new(rules::NoDeleteVar::new(context.clone())),
      Box::new(rules::UseIsNaN::new(context.clone())),
    ];

    for mut rule in rules {
      rule.visit_module(&module, &module);
    }

    let ban_ts = rules::BanTsIgnore::new(context.clone());
    ban_ts.lint_comments();
    let ban_todo = rules::BanUntaggedTodo::new(context.clone());
    ban_todo.lint_comments();

    let diags = context.diagnostics.lock().unwrap();
    for d in diags.iter() {
      eprintln!(
        "error: {} at {}:{}:{}",
        d.message, d.location.filename, d.location.line, d.location.col
      );
    }
  }
}

fn main() {
  let args: Vec<String> = std::env::args().collect();

  if args.len() < 2 {
    eprintln!("Missing file name");
    std::process::exit(1);
  }

  let file_names: Vec<String> = args[1..].to_vec();

  for file_name in file_names {
    let source_code =
      std::fs::read_to_string(&file_name).expect("Failed to read file");
    let mut linter = Linter::new();
    linter.lint(file_name, source_code).expect("Failed to lint");
  }
}
