// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;
use swc_common::comments::Comments;
use swc_common::errors::Diagnostic;
use swc_common::errors::DiagnosticBuilder;
use swc_common::errors::Emitter;
use swc_common::errors::Handler;
use swc_common::errors::HandlerFlags;
use swc_common::FileName;
use swc_common::Globals;
use swc_common::SourceMap;
use swc_common::Span;
use swc_common::DUMMY_SP;
use swc_ecma_ast::{BinaryOp, Expr, ParenExpr};
use swc_ecma_parser::lexer::Lexer;
use swc_ecma_parser::EsConfig;
use swc_ecma_parser::JscTarget;
use swc_ecma_parser::Parser;
use swc_ecma_parser::Session;
use swc_ecma_parser::SourceFileInput;
use swc_ecma_parser::Syntax;
use swc_ecma_parser::TsConfig;
use swc_ecma_visit::Fold;

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
    F: FnOnce(Result<swc_ecma_ast::Module, SwcDiagnosticBuffer>, Comments) -> R,
  {
    swc_common::GLOBALS.set(&self.globals, || {
      let swc_source_file = self.source_map.new_source_file(
        FileName::Custom(file_name.to_string()),
        source_code.to_string(),
      );

      let buffered_err = self.buffered_error.clone();
      let session = Session {
        handler: &self.handler,
      };

      let comments = Comments::default();
      let lexer = Lexer::new(
        session,
        syntax,
        JscTarget::Es2019,
        SourceFileInput::from(&*swc_source_file),
        Some(&comments),
      );

      let mut parser = Parser::new_from(session, lexer);

      let parse_result =
        parser
          .parse_module()
          .map_err(move |mut err: DiagnosticBuilder| {
            err.emit();
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

/// A struct that has span-dropped ast node.
#[derive(Debug, Eq, PartialEq)]
pub(crate) struct SpanDropped<T: std::fmt::Debug + Eq + PartialEq>(T);

impl<T: std::fmt::Debug + Eq + PartialEq> SpanDropped<T> {
  pub(crate) fn as_ref(&self) -> SpanDropped<&T> {
    let SpanDropped(ref inner) = self;
    SpanDropped(inner)
  }
}

/// Determines whether the two given `Expr`s are considered to be equal in if-else condition
/// context.
pub(crate) fn equal_in_if_else(
  one: &SpanDropped<&Expr>,
  other: &SpanDropped<&Expr>,
) -> bool {
  let SpanDropped(ref expr1) = one;
  let SpanDropped(ref expr2) = other;

  use swc_ecma_ast::Expr::*;
  match (expr1, expr2) {
    (Bin(ref bin1), Bin(ref bin2))
      if matches!(bin1.op, BinaryOp::LogicalOr | BinaryOp::LogicalAnd)
        && bin1.op == bin2.op =>
    {
      let b1_left = SpanDropped(&*bin1.left);
      let b2_left = SpanDropped(&*bin2.left);
      let b1_right = SpanDropped(&*bin1.right);
      let b2_right = SpanDropped(&*bin2.right);
      equal_in_if_else(&b1_left, &b2_left)
        && equal_in_if_else(&b1_right, &b2_right)
        || equal_in_if_else(&b1_left, &b2_right)
          && equal_in_if_else(&b1_right, &b2_left)
    }
    (Paren(ParenExpr { ref expr, .. }), _) => {
      equal_in_if_else(&SpanDropped(&**expr), other)
    }
    (_, Paren(ParenExpr { ref expr, .. })) => {
      equal_in_if_else(one, &SpanDropped(&**expr))
    }
    (This(_), This(_))
    | (Array(_), Array(_))
    | (Object(_), Object(_))
    | (Fn(_), Fn(_))
    | (Unary(_), Unary(_))
    | (Update(_), Update(_))
    | (Bin(_), Bin(_))
    | (Assign(_), Member(_))
    | (Cond(_), Cond(_))
    | (Call(_), Call(_))
    | (New(_), New(_))
    | (Seq(_), Seq(_))
    | (Ident(_), Ident(_))
    | (Lit(_), Lit(_))
    | (Tpl(_), Tpl(_))
    | (TaggedTpl(_), TaggedTpl(_))
    | (Arrow(_), Arrow(_))
    | (Class(_), Class(_))
    | (Yield(_), Yield(_))
    | (MetaProp(_), MetaProp(_))
    | (Await(_), Await(_))
    | (JSXMember(_), JSXMember(_))
    | (JSXNamespacedName(_), JSXNamespacedName(_))
    | (JSXEmpty(_), JSXEmpty(_))
    | (JSXElement(_), JSXElement(_))
    | (JSXFragment(_), JSXFragment(_))
    | (TsTypeAssertion(_), TsTypeAssertion(_))
    | (TsConstAssertion(_), TsConstAssertion(_))
    | (TsNonNull(_), TsNonNull(_))
    | (TsTypeCast(_), TsTypeCast(_))
    | (TsAs(_), TsAs(_))
    | (PrivateName(_), PrivateName(_))
    | (OptChain(_), OptChain(_))
    | (Invalid(_), Invalid(_)) => expr1 == expr2,
    _ => false,
  }
}

impl<T> PartialOrd for SpanDropped<T>
where
  T: std::fmt::Debug + Eq + PartialEq,
{
  fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
    Some(self.cmp(other))
  }
}

impl<T> Ord for SpanDropped<T>
where
  T: std::fmt::Debug + Eq + PartialEq,
{
  fn cmp(&self, other: &Self) -> std::cmp::Ordering {
    let SpanDropped(ref self_inner) = self;
    let SpanDropped(ref other_inner) = other;
    let self_debug = format!("{:?}", self_inner);
    let other_debug = format!("{:?}", other_inner);
    self_debug.cmp(&other_debug)
  }
}

/// Provides a additional method to drop spans.
pub(crate) trait DropSpan {
  fn drop_span(self) -> SpanDropped<Self>
  where
    Self: Sized + std::fmt::Debug + Eq + PartialEq;
}

impl DropSpan for Expr {
  fn drop_span(self) -> SpanDropped<Self> {
    let mut dropper = SpanDropper;
    let dropped = dropper.fold_expr(self);
    SpanDropped(dropped)
  }
}
