// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::scopes::Scope;
use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::RwLock;
use swc_common::comments::SingleThreadedComments;
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
use swc_common::{Mark, GLOBALS};
use swc_ecmascript::ast::{
  ComputedPropName, Expr, ExprOrSpread, Ident, Lit, MemberExpr, PatOrExpr,
  Prop, PropName, PropOrSpread, Str, Tpl,
};
use swc_ecmascript::parser::lexer::Lexer;
use swc_ecmascript::parser::EsConfig;
use swc_ecmascript::parser::JscTarget;
use swc_ecmascript::parser::Parser;
use swc_ecmascript::parser::StringInput;
use swc_ecmascript::parser::Syntax;
use swc_ecmascript::parser::TsConfig;
use swc_ecmascript::transforms::resolver::ts_resolver;
use swc_ecmascript::visit::Fold;
use swc_ecmascript::{
  utils::{find_ids, ident::IdentLike, Id},
  visit::FoldWith,
};

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

  pub fn parse_module(
    &self,
    file_name: &str,
    syntax: Syntax,
    source_code: &str,
  ) -> (
    Result<swc_ecmascript::ast::Module, SwcDiagnosticBuffer>,
    SingleThreadedComments,
  ) {
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

    let parse_result = parse_result.map(|module| {
      GLOBALS.set(&self.globals, || {
        let mark = Mark::fresh(Mark::root());
        module.fold_with(&mut ts_resolver(mark))
      })
    });

    (parse_result, comments)
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
  scope: &Scope,
  expr_ident: &Ident,
  expr_args: &[ExprOrSpread],
) -> Option<String> {
  if expr_ident.sym != *"RegExp" {
    return None;
  }

  if scope.var(&expr_ident.to_id()).is_some() {
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

impl Key for Lit {
  fn get_key(&self) -> Option<String> {
    use swc_ecmascript::ast::BigInt;
    use swc_ecmascript::ast::Bool;
    use swc_ecmascript::ast::JSXText;
    use swc_ecmascript::ast::Number;
    use swc_ecmascript::ast::Regex;
    match self {
      Lit::Str(Str { ref value, .. }) => Some(value.to_string()),
      Lit::Bool(Bool { ref value, .. }) => {
        let str_val = if *value { "true" } else { "false" };
        Some(str_val.to_string())
      }
      Lit::Null(_) => Some("null".to_string()),
      Lit::Num(Number { ref value, .. }) => Some(value.to_string()),
      Lit::BigInt(BigInt { ref value, .. }) => Some(value.to_string()),
      Lit::Regex(Regex { ref exp, .. }) => Some(format!("/{}/", exp)),
      Lit::JSXText(JSXText { ref raw, .. }) => Some(raw.to_string()),
    }
  }
}

impl Key for Tpl {
  fn get_key(&self) -> Option<String> {
    if self.exprs.is_empty() {
      self.quasis.get(0).map(|q| q.raw.value.to_string())
    } else {
      None
    }
  }
}

impl Key for Expr {
  fn get_key(&self) -> Option<String> {
    match self {
      Expr::Ident(ident) => Some(ident.sym.to_string()),
      Expr::Lit(lit) => lit.get_key(),
      Expr::Tpl(tpl) => tpl.get_key(),
      _ => None,
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
        Expr::Lit(lit) => lit.get_key(),
        Expr::Tpl(tpl) => tpl.get_key(),
        _ => None,
      },
    }
  }
}

impl Key for MemberExpr {
  fn get_key(&self) -> Option<String> {
    if let Expr::Ident(ident) = &*self.prop {
      if !self.computed {
        return Some(ident.sym.to_string());
      }
    }

    (&*self.prop).get_key()
  }
}

/// Find [Id]s in the lhs of an assigmnet expression.
pub(crate) fn find_lhs_ids(n: &PatOrExpr) -> Vec<Id> {
  match &n {
    PatOrExpr::Expr(e) => match &**e {
      Expr::Ident(i) => vec![i.to_id()],
      _ => vec![],
    },
    PatOrExpr::Pat(p) => find_ids(p),
  }
}
