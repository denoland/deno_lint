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

    use rules::LintRule;
    let rules: Vec<Box<dyn LintRule>> = vec![
      // rules::NoExplicitAny::new(),
      // rules::NoDebugger::new(),
      // rules::NoVar::new(),
      // rules::SingleVarDeclarator::new(),
      rules::ExplicitFunctionReturnType::new(),
      // rules::NoEval::new(),
      // rules::NoEmptyInterface::new(),
      // rules::NoDeleteVar::new(),
      // rules::UseIsNaN::new(),
      // rules::NoEmptyFunction::new(),
      // rules::NoAsyncPromiseExecutor::new(),
      // rules::NoSparseArray::new(),
      // rules::NoDuplicateCase::new(),
      // rules::NoDupeArgs::new(),
    ];

    for mut rule in rules {
      rule.lint_module(context.clone(), module.clone());
    }

    // let ban_ts = rules::BanTsIgnore::new(context.clone());
    // ban_ts.lint_comments();
    // let ban_todo = rules::BanUntaggedTodo::new(context.clone());
    // ban_todo.lint_comments();

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
    let mut linter = Linter::default();
    linter.lint(file_name, source_code).expect("Failed to lint");
  }
}
