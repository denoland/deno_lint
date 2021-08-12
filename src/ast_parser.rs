// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use ast_view::SourceFile;
use ast_view::SourceFileTextInfo;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use swc_common::comments::SingleThreadedComments;
use swc_common::comments::SingleThreadedCommentsMapInner;
use swc_common::BytePos;
use swc_common::Globals;
use swc_common::Mark;
use swc_common::Spanned;
use swc_ecmascript::ast;
use swc_ecmascript::parser::lexer::Lexer;
use swc_ecmascript::parser::token::TokenAndSpan;
use swc_ecmascript::parser::Capturing;
use swc_ecmascript::parser::EsConfig;
use swc_ecmascript::parser::JscTarget;
use swc_ecmascript::parser::Parser;
use swc_ecmascript::parser::StringInput;
use swc_ecmascript::parser::Syntax;
use swc_ecmascript::parser::TsConfig;
use swc_ecmascript::transforms::resolver::ts_resolver;
use swc_ecmascript::visit::FoldWith;

#[allow(unused)]
pub fn get_default_es_config() -> Syntax {
  let config = EsConfig {
    num_sep: true,
    class_private_props: false,
    class_private_methods: false,
    class_props: false,
    export_default_from: true,
    export_namespace_from: true,
    dynamic_import: true,
    nullish_coalescing: true,
    optional_chaining: true,
    import_meta: true,
    top_level_await: true,
    ..Default::default()
  };
  Syntax::Es(config)
}

pub fn get_default_ts_config() -> Syntax {
  let ts_config = TsConfig {
    dynamic_import: true,
    decorators: true,
    ..Default::default()
  };
  Syntax::Typescript(ts_config)
}

#[derive(Clone, Debug)]
pub struct SwcDiagnostic {
  pub filename: String,
  pub line_display: usize,
  pub column_display: usize,
  pub message: String,
}

impl Error for SwcDiagnostic {}

impl fmt::Display for SwcDiagnostic {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    f.write_str(&format!(
      "{} at {}:{}:{}",
      self.message, self.filename, self.line_display, self.column_display
    ))
  }
}

impl SwcDiagnostic {
  pub(crate) fn from_swc_error(
    filename: &str,
    source_file: &impl SourceFile,
    err: swc_ecmascript::parser::error::Error,
  ) -> Self {
    let span = err.span();
    let line_and_column = source_file.line_and_column_index(span.lo);

    SwcDiagnostic {
      line_display: line_and_column.line_index + 1,
      column_display: line_and_column.column_index + 1,
      filename: filename.to_string(),
      message: err.kind().msg().to_string(),
    }
  }
}

/// Low-level utility structure with common AST parsing functions.
///
/// Allows to build more complicated parser by providing a callback
/// to `parse_module`.
pub(crate) struct AstParser {
  pub(crate) globals: Globals,
  /// The marker passed to the resolver (from swc).
  ///
  /// This mark is applied to top level bindings and unresolved references.
  pub(crate) top_level_mark: Mark,
}

impl AstParser {
  pub(crate) fn new() -> Self {
    let globals = Globals::new();
    let top_level_mark =
      swc_common::GLOBALS.set(&globals, || Mark::fresh(Mark::root()));

    AstParser {
      globals,
      top_level_mark,
    }
  }

  pub(crate) fn parse_program(
    &self,
    file_name: &str,
    syntax: Syntax,
    source_code: &str,
  ) -> Result<ParsedData, SwcDiagnostic> {
    let source_file =
      SourceFileTextInfo::new(BytePos(0), source_code.to_string());
    let string_input = StringInput::new(
      source_code,
      BytePos(0),
      BytePos(source_code.len() as u32),
    );

    let comments = SingleThreadedComments::default();
    let lexer = Capturing::new(Lexer::new(
      syntax,
      JscTarget::Es2019,
      string_input,
      Some(&comments),
    ));

    let mut parser = Parser::new_from(lexer);
    let program = parser.parse_program().map_err(|err| {
      SwcDiagnostic::from_swc_error(file_name, &source_file, err)
    })?;

    let program = swc_common::GLOBALS.set(&self.globals, || {
      program.fold_with(&mut ts_resolver(self.top_level_mark))
    });

    let tokens = parser.input().take();

    // take out the comment maps because that's what dprint-swc-ast-view
    // uses and what we use in deno's language server because it is Sync.
    let (leading, trailing) = comments.take_all();
    let leading_comments = Rc::try_unwrap(leading).unwrap().into_inner();
    let trailing_comments = Rc::try_unwrap(trailing).unwrap().into_inner();

    Ok(ParsedData {
      source_file,
      program,
      leading_comments,
      trailing_comments,
      tokens,
    })
  }
}

impl Default for AstParser {
  fn default() -> Self {
    Self::new()
  }
}

pub(crate) struct ParsedData {
  pub(crate) source_file: SourceFileTextInfo,
  pub(crate) program: ast::Program,
  pub(crate) leading_comments: SingleThreadedCommentsMapInner,
  pub(crate) trailing_comments: SingleThreadedCommentsMapInner,
  pub(crate) tokens: Vec<TokenAndSpan>,
}
