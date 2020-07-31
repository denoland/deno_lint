// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::scopes::Scope;
use crate::swc_atoms::js_word;
use crate::swc_common;
use crate::swc_common::comments::SingleThreadedComments;
use crate::swc_common::errors::Diagnostic;
use crate::swc_common::errors::DiagnosticBuilder;
use crate::swc_common::errors::Emitter;
use crate::swc_common::errors::Handler;
use crate::swc_common::errors::HandlerFlags;
use crate::swc_common::FileName;
use crate::swc_common::Globals;
use crate::swc_common::SourceMap;
use crate::swc_common::Span;
use crate::swc_common::DUMMY_SP;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::{
  ComputedPropName, Expr, ExprOrSpread, Ident, Lit, Prop, PropName,
  PropOrSpread, Str, Tpl,
};
use crate::swc_ecma_parser::lexer::Lexer;
use crate::swc_ecma_parser::EsConfig;
use crate::swc_ecma_parser::JscTarget;
use crate::swc_ecma_parser::Parser;
use crate::swc_ecma_parser::StringInput;
use crate::swc_ecma_parser::Syntax;
use crate::swc_ecma_parser::TsConfig;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;
use swc_ecmascript::visit::Fold;

#[allow(unused)]
pub fn get_default_es_config() -> Syntax {
  let mut config = EsConfig::default();
  config.num_sep = true;
  config.class_private_props = false;
  config.class_private_methods = false;
  config.class_props = false;
  config.export_default_from = true;
  config.export_namespace_from = true;
  config.dynamic_import = true;
  config.nullish_coalescing = true;
  config.optional_chaining = true;
  config.import_meta = true;
  config.top_level_await = true;
  Syntax::Es(config)
}

#[allow(unused)]
pub fn get_default_ts_config() -> Syntax {
  let mut ts_config = TsConfig::default();
  ts_config.dynamic_import = true;
  ts_config.decorators = true;
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
  pub fn from_swc_error(
    error_buffer: SwcErrorBuffer,
    parser: &AstParser,
  ) -> Self {
    let s = error_buffer.0.read().unwrap().clone();

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
pub struct SwcErrorBuffer(Arc<RwLock<Vec<Diagnostic>>>);

impl SwcErrorBuffer {
  pub fn default() -> Self {
    Self(Arc::new(RwLock::new(vec![])))
  }
}

impl Emitter for SwcErrorBuffer {
  fn emit(&mut self, db: &DiagnosticBuilder) {
    self.0.write().unwrap().push((**db).clone());
  }
}

/// Low-level utility structure with common AST parsing functions.
///
/// Allows to build more complicated parser by providing a callback
/// to `parse_module`.
pub struct AstParser {
  pub buffered_error: SwcErrorBuffer,
  pub source_map: Arc<SourceMap>,
  pub handler: Handler,
  pub globals: Globals,
}

impl AstParser {
  pub fn new() -> Self {
    let buffered_error = SwcErrorBuffer::default();

    let handler = Handler::with_emitter_and_flags(
      Box::new(buffered_error.clone()),
      HandlerFlags {
        dont_buffer_diagnostics: true,
        can_emit_warnings: true,
        ..Default::default()
      },
    );

    AstParser {
      buffered_error,
      source_map: Arc::new(SourceMap::default()),
      handler,
      globals: Globals::new(),
    }
  }

  pub fn parse_module<F, R>(
    &self,
    file_name: &str,
    syntax: Syntax,
    source_code: &str,
    callback: F,
  ) -> R
  where
    F: FnOnce(Result<swc_ecma_ast::Module, SwcDiagnosticBuffer>, SingleThreadedComments) -> R,
  {
    swc_common::GLOBALS.set(&self.globals, || {
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

      let mut parser = Parser::new_from(lexer);

      let parse_result = parser.parse_module().map_err(move |err| {
        let mut diagnostic_builder = err.into_diagnostic(&self.handler);
        diagnostic_builder.emit();
        SwcDiagnosticBuffer::from_swc_error(buffered_err, self)
      });

      callback(parse_result, comments)
    })
  }

  pub fn get_span_location(&self, span: Span) -> swc_common::Loc {
    self.source_map.lookup_char_pos(span.lo())
  }

  // pub fn get_span_comments(
  //   &self,
  //   span: Span,
  // ) -> Vec<swc_common::comments::Comment> {
  //   match self.leading_comments.get(&span.lo()) {
  //     Some(c) => c.clone(),
  //     None => vec![],
  //   }
  // }
}

impl Default for AstParser {
  fn default() -> Self {
    Self::new()
  }
}

/// A folder to drop all spans of a subtree.
struct SpanDropper;

impl Fold for SpanDropper {
  fn fold_span(&mut self, _: Span) -> Span {
    DUMMY_SP
  }
}

/// Provides an additional method to drop spans.
pub(crate) trait DropSpan {
  fn drop_span(self) -> Self;
}

impl DropSpan for Expr {
  fn drop_span(self) -> Self {
    let mut dropper = SpanDropper;
    dropper.fold_expr(self)
  }
}

/// Extracts regex string from an expression, using ScopeManager.
/// If the passed expression is not regular expression, this will return `None`.
pub(crate) fn extract_regex(
  root_scope: &Scope,
  expr_span: Span,
  expr_ident: &Ident,
  expr_args: &[ExprOrSpread],
) -> Option<String> {
  if expr_ident.sym != js_word!("RegExp") {
    return None;
  }

  let scope = root_scope.get_scope_for_span(expr_span);
  if scope.get_binding(&expr_ident.sym).is_some() {
    return None;
  }

  match expr_args.get(0) {
    Some(first_arg) => match &*first_arg.expr {
      Expr::Lit(Lit::Str(literal)) => Some(literal.value.to_string()),
      Expr::Lit(Lit::Regex(regex)) => Some(regex.exp.to_string()),
      _ => None,
    },
    None => None,
  }
}

pub(crate) trait Key {
  fn get_key(&self) -> Option<String>;
}

impl Key for PropOrSpread {
  fn get_key(&self) -> Option<String> {
    use PropOrSpread::*;
    match self {
      Prop(p) => (&**p).get_key(),
      Spread(_) => None,
    }
  }
}

impl Key for Prop {
  fn get_key(&self) -> Option<String> {
    use Prop::*;
    match self {
      KeyValue(key_value) => key_value.key.get_key(),
      Getter(getter) => getter.key.get_key(),
      Setter(setter) => setter.key.get_key(),
      Method(method) => method.key.get_key(),
      Shorthand(_) => None,
      Assign(_) => None,
    }
  }
}

impl Key for PropName {
  fn get_key(&self) -> Option<String> {
    match self {
      PropName::Ident(identifier) => Some(identifier.sym.to_string()),
      PropName::Str(str) => Some(str.value.to_string()),
      PropName::Num(num) => Some(num.to_string()),
      PropName::Computed(ComputedPropName { ref expr, .. }) => match &**expr {
        Expr::Lit(Lit::Str(Str { ref value, .. })) => Some(value.to_string()),
        Expr::Tpl(Tpl {
          ref exprs,
          ref quasis,
          ..
        }) => {
          if exprs.is_empty() {
            quasis.iter().next().map(|q| q.raw.value.to_string())
          } else {
            None
          }
        }
        _ => None,
      },
    }
  }
}
