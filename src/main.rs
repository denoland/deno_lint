#![feature(specialization)]

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
use swc_common::Span;
use swc_common::{Visit, VisitWith};
use swc_ecma_ast;
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::JscTarget;
use swc_ecma_parser::Parser;
use swc_ecma_parser::Session;
use swc_ecma_parser::SourceFileInput;
use swc_ecma_parser::Syntax;
use swc_ecma_parser::TsConfig;

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
  comments: Comments,
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
      comments: Comments::default(),
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
        Some(&self.comments),
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

  fn lint_module(&self, file_name: String, module: swc_ecma_ast::Module) {
    let context = Context {
      file_name,
      diagnostics: Arc::new(Mutex::new(vec![])),
      source_map: self.source_map.clone(),
    };
    let mut any_finder = ExplicitAnyFinder {
      context: context.clone(),
    };
    module.visit_with(&mut any_finder);
    let mut debugger_finder = DebuggerFinder {
      context: context.clone(),
    };
    module.visit_with(&mut debugger_finder);
    let mut var_finder = VarFinder {
      context: context.clone(),
    };
    module.visit_with(&mut var_finder);
    let mut single_var = SingleVarDeclaratorFinder {
      context: context.clone(),
    };
    module.visit_with(&mut single_var);

    let diags = context.diagnostics.lock().unwrap();
    for d in diags.iter() {
      eprintln!(
        "error: {} at {}:{}:{}",
        d.message, d.location.filename, d.location.line, d.location.col
      );
    }
  }
}

#[derive(Debug, Clone)]
pub struct Location {
  pub filename: String,
  pub line: usize,
  pub col: usize,
}

impl Into<Location> for swc_common::Loc {
  fn into(self) -> Location {
    use swc_common::FileName::*;

    let filename = match &self.file.name {
      Real(path_buf) => path_buf.to_string_lossy().to_string(),
      Custom(str_) => str_.to_string(),
      _ => panic!("invalid filename"),
    };

    Location {
      filename,
      line: self.line,
      col: self.col_display,
    }
  }
}

#[derive(Debug)]
struct LintDiagnotic {
  location: Location,
  message: String,
  code: String,
}

#[derive(Clone)]
struct Context {
  file_name: String,
  diagnostics: Arc<Mutex<Vec<LintDiagnotic>>>,
  source_map: Arc<SourceMap>,
}

impl Context {
  pub fn add_diagnostic(&self, span: Span, code: &str, message: &str) {
    let location = self.source_map.lookup_char_pos(span.lo());
    let mut diags = self.diagnostics.lock().unwrap();
    diags.push(LintDiagnotic {
      location: location.into(),
      message: message.to_string(),
      code: code.to_string(),
    });
  }
}

struct ExplicitAnyFinder {
  context: Context,
}

impl<T> Visit<T> for ExplicitAnyFinder
where
  T: VisitWith<Self>,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::TsTypeAnn> for ExplicitAnyFinder {
  fn visit(&mut self, node: &swc_ecma_ast::TsTypeAnn) {
    use swc_ecma_ast::TsKeywordTypeKind::*;
    use swc_ecma_ast::TsType::*;

    match &*node.type_ann {
      TsKeywordType(keyword_type) => match keyword_type.kind {
        TsAnyKeyword => {
          self.context.add_diagnostic(
            node.span,
            "noExplicitAny",
            "`any` type is not allowed",
          );
        }
        _ => {}
      },
      _ => {}
    }
  }
}
struct DebuggerFinder {
  context: Context,
}

impl<T> Visit<T> for DebuggerFinder
where
  T: VisitWith<Self>,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::DebuggerStmt> for DebuggerFinder {
  fn visit(&mut self, node: &swc_ecma_ast::DebuggerStmt) {
    self.context.add_diagnostic(
      node.span,
      "noDebugger",
      "`debugger` statement is not allowed",
    );
  }
}

struct VarFinder {
  context: Context,
}

impl<T> Visit<T> for VarFinder
where
  T: VisitWith<Self> + std::fmt::Debug,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::VarDecl> for VarFinder {
  fn visit(&mut self, node: &swc_ecma_ast::VarDecl) {
    if node.kind == swc_ecma_ast::VarDeclKind::Var {
      self.context.add_diagnostic(
        node.span,
        "noVar",
        "`var` keyword is not allowed",
      );
    }
  }
}

struct SingleVarDeclaratorFinder {
  context: Context,
}

impl<T> Visit<T> for SingleVarDeclaratorFinder
where
  T: VisitWith<Self> + std::fmt::Debug,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::VarDecl> for SingleVarDeclaratorFinder {
  fn visit(&mut self, node: &swc_ecma_ast::VarDecl) {
    if node.decls.len() > 1 {
      self.context.add_diagnostic(
        node.span,
        "singleVarDeclarator",
        "Multiple variable declarators are not allowed",
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

  let file_name = args[1].to_string();
  let source_code =
    std::fs::read_to_string(&file_name).expect("Failed to read file");
  let mut linter = Linter::new();
  linter.lint(file_name, source_code).expect("Failed to lint");
}
