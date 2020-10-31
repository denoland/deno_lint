// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::scopes::Scope;
use std::error::Error;
use std::fmt;
use std::rc::Rc;
use std::sync::RwLock;
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
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::{
  ComputedPropName, Expr, ExprOrSpread, Ident, Lit, MemberExpr, PatOrExpr,
  PrivateName, Prop, PropName, PropOrSpread, Str, Tpl,
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
  utils::{find_ids, ident::IdentLike},
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
  pub(crate) fn from_swc_error(
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
pub(crate) struct SwcErrorBuffer(Rc<RwLock<Vec<Diagnostic>>>);

impl SwcErrorBuffer {
  pub(crate) fn default() -> Self {
    Self(Rc::new(RwLock::new(vec![])))
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

  pub(crate) fn parse_script(
    &self,
    file_name: &str,
    syntax: Syntax,
    source_code: &str,
  ) -> (
    Result<swc_ecmascript::ast::Script, SwcDiagnosticBuffer>,
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

    let parse_result = parser.parse_script().map_err(move |err| {
      let mut diagnostic_builder = err.into_diagnostic(&self.handler);
      diagnostic_builder.emit();
      SwcDiagnosticBuffer::from_swc_error(buffered_err, self)
    });

    let parse_result = parse_result.map(|script| {
      swc_common::GLOBALS.set(&self.globals, || {
        script.fold_with(&mut ts_resolver(self.top_level_mark))
      })
    });

    (parse_result, comments)
  }

  pub(crate) fn parse_module(
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
      swc_common::GLOBALS.set(&self.globals, || {
        module.fold_with(&mut ts_resolver(self.top_level_mark))
      })
    });

    (parse_result, comments)
  }

  pub(crate) fn get_span_location(&self, span: Span) -> swc_common::Loc {
    self.source_map.lookup_char_pos(span.lo())
  }

  // pub(crate) fn get_span_comments(
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

pub(crate) trait KeyDisplay {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display>;
}

impl<T: KeyDisplay> Key for T {
  fn get_key(&self) -> Option<String> {
    self.get_key_ref().map(ToString::to_string)
  }
}

impl KeyDisplay for Ident {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    Some(&self.sym.as_ref())
  }
}

impl KeyDisplay for PropOrSpread {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    use PropOrSpread::*;
    match self {
      Prop(p) => (&**p).get_key_ref(),
      Spread(_) => None,
    }
  }
}

impl KeyDisplay for Prop {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    use Prop::*;
    match self {
      KeyValue(key_value) => key_value.key.get_key_ref(),
      Getter(getter) => getter.key.get_key_ref(),
      Setter(setter) => setter.key.get_key_ref(),
      Method(method) => method.key.get_key_ref(),
      Shorthand(_) => None,
      Assign(_) => None,
    }
  }
}

impl KeyDisplay for Lit {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    use swc_ecmascript::ast::BigInt;
    use swc_ecmascript::ast::Bool;
    use swc_ecmascript::ast::JSXText;
    use swc_ecmascript::ast::Number;
    use swc_ecmascript::ast::Regex;
    match self {
      Lit::Str(Str { ref value, .. }) => Some(value),
      Lit::Bool(Bool { ref value, .. }) => {
        let str_val = if *value { "true" } else { "false" };
        Some(&str_val)
      }
      Lit::Null(_) => Some(&"null"),
      Lit::Num(Number { ref value, .. }) => Some(value),
      Lit::BigInt(BigInt { ref value, .. }) => Some(value),
      Lit::Regex(Regex { ref exp, .. }) => Some(exp),
      Lit::JSXText(JSXText { ref raw, .. }) => Some(raw),
    }
  }
}

impl KeyDisplay for Tpl {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    if self.exprs.is_empty() {
      self
        .quasis
        .get(0)
        .map(|q| &q.raw.value.as_ref() as &dyn fmt::Display)
    } else {
      None
    }
  }
}

impl KeyDisplay for Expr {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    match self {
      Expr::Ident(ident) => Some(&ident.sym.as_ref()),
      Expr::Lit(lit) => lit.get_key_ref(),
      Expr::Tpl(tpl) => tpl.get_key_ref(),
      _ => None,
    }
  }
}

impl KeyDisplay for PropName {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    match self {
      PropName::Ident(identifier) => Some(&identifier.sym.as_ref()),
      PropName::Str(str) => Some(&str.value.as_ref()),
      PropName::Num(num) => Some(&num.value),
      PropName::Computed(ComputedPropName { ref expr, .. }) => match &**expr {
        Expr::Lit(lit) => lit.get_key_ref(),
        Expr::Tpl(tpl) => tpl.get_key_ref(),
        _ => None,
      },
    }
  }
}

impl KeyDisplay for PrivateName {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    Some(&self.id.sym.as_ref())
  }
}

impl KeyDisplay for MemberExpr {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    if let Expr::Ident(ident) = &*self.prop {
      if !self.computed {
        return Some(&ident.sym.as_ref());
      }
    }

    (&*self.prop).get_key_ref()
  }
}

impl<K: KeyDisplay> KeyDisplay for Option<K> {
  fn get_key_ref(&self) -> Option<&dyn fmt::Display> {
    self.as_ref().and_then(|k| k.get_key_ref())
  }
}

/// Find [Id]s in the lhs of an assigmnet expression.
pub(crate) fn find_lhs_ids<I>(n: &PatOrExpr) -> Vec<I>
where
  I: IdentLike,
{
  match &n {
    PatOrExpr::Expr(e) => match &**e {
      Expr::Ident(i) => vec![I::from_ident(i)],
      _ => vec![],
    },
    PatOrExpr::Pat(p) => find_ids(p),
  }
}
