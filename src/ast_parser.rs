// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use dprint_swc_ecma_ast_view::TokenAndSpan;
use std::cell::RefCell;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use swc_common::comments::SingleThreadedComments;
use swc_common::errors::Diagnostic;
use swc_common::errors::DiagnosticBuilder;
use swc_common::errors::Emitter;
use swc_common::errors::Handler;
use swc_common::errors::HandlerFlags;
use swc_common::FileName;
use swc_common::Globals;
use swc_common::Mark;
use swc_common::SourceMap;
use swc_common::Span;
use swc_ecmascript::ast;
use swc_ecmascript::parser::lexer::Lexer;
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
pub struct SwcDiagnosticBuffer {
  pub diagnostics: Vec<String>,
}

impl Error for SwcDiagnosticBuffer {}

impl fmt::Display for SwcDiagnosticBuffer {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    let msg = self.diagnostics.join(",");

    f.pad(&msg)
  }
}

impl SwcDiagnosticBuffer {
  pub(crate) fn from_swc_error(
    error_buffer: SwcErrorBuffer,
    parser: &AstParser,
  ) -> Self {
    let s = error_buffer.0.borrow().clone();

    let diagnostics = s
      .iter()
      .map(|d| {
        let mut msg = d.message();

        if let Some(span) = d.span.primary_span() {
          let location = parser.get_span_location(span);
          let filename = match &location.file.name {
            FileName::Custom(n) => n,
            _ => unreachable!(),
          };
          msg = format!(
            "{} at {}:{}:{}",
            msg, filename, location.line, location.col_display
          );
        }

        msg
      })
      .collect::<Vec<String>>();

    Self { diagnostics }
  }
}

#[derive(Clone)]
pub(crate) struct SwcErrorBuffer(Rc<RefCell<Vec<Diagnostic>>>);

impl SwcErrorBuffer {
  pub(crate) fn default() -> Self {
    Self(Rc::new(RefCell::new(vec![])))
  }
}

impl Emitter for SwcErrorBuffer {
  fn emit(&mut self, db: &DiagnosticBuilder) {
    self.0.borrow_mut().push((**db).clone());
  }
}

/// Low-level utility structure with common AST parsing functions.
///
/// Allows to build more complicated parser by providing a callback
/// to `parse_module`.
pub(crate) struct AstParser {
  pub(crate) buffered_error: SwcErrorBuffer,
  pub(crate) source_map: Rc<SourceMap>,
  pub(crate) handler: Handler,
  pub(crate) globals: Globals,
  /// The marker passed to the resolver (from swc).
  ///
  /// This mark is applied to top level bindings and unresolved references.
  pub(crate) top_level_mark: Mark,
}

impl AstParser {
  pub(crate) fn new() -> Self {
    let buffered_error = SwcErrorBuffer::default();

    let handler = Handler::with_emitter_and_flags(
      Box::new(buffered_error.clone()),
      HandlerFlags {
        dont_buffer_diagnostics: true,
        can_emit_warnings: true,
        ..Default::default()
      },
    );

    let globals = Globals::new();
    let top_level_mark =
      swc_common::GLOBALS.set(&globals, || Mark::fresh(Mark::root()));

    AstParser {
      buffered_error,
      source_map: Rc::new(SourceMap::default()),
      handler,
      globals,
      top_level_mark,
    }
  }

  pub(crate) fn set_source_map(&mut self, source_map: Rc<SourceMap>) {
    self.source_map = source_map;
  }

  pub(crate) fn parse_program(
    &self,
    file_name: &str,
    syntax: Syntax,
    source_code: &str,
  ) -> Result<ParsedData, SwcDiagnosticBuffer> {
    // NOTE: calling `self.source_map.new_source_file` mutates `source_map`
    // even though it's of type `Rc`.
    let swc_source_file = self.source_map.new_source_file(
      FileName::Custom(file_name.to_string()),
      source_code.to_string(),
    );

    let buffered_err = self.buffered_error.clone();

    let comments = SingleThreadedComments::default();
    let lexer = Lexer::new(
      syntax,
      JscTarget::Es2019,
      StringInput::from(&*swc_source_file),
      Some(&comments),
    );

    let tokens: Vec<TokenAndSpan> = lexer.clone().into_iter().collect();

    let mut parser = Parser::new_from(lexer);

    let parse_result = parser.parse_program().map_err(move |err| {
      let mut diagnostic_builder = err.into_diagnostic(&self.handler);
      diagnostic_builder.emit();
      SwcDiagnosticBuffer::from_swc_error(buffered_err, self)
    });

    let parse_result = parse_result.map(|program| {
      swc_common::GLOBALS.set(&self.globals, || {
        program.fold_with(&mut ts_resolver(self.top_level_mark))
      })
    });

    parse_result.map(|program| ParsedData {
      program,
      comments,
      tokens,
    })
  }

  pub(crate) fn get_span_location(&self, span: Span) -> swc_common::Loc {
    self.source_map.lookup_char_pos(span.lo())
  }
}

impl Default for AstParser {
  fn default() -> Self {
    Self::new()
  }
}

pub(crate) struct ParsedData {
  pub(crate) program: ast::Program,
  pub(crate) comments: SingleThreadedComments,
  pub(crate) tokens: Vec<TokenAndSpan>,
}
