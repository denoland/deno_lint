// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::swc::ast::{
  ArrowExpr, BigInt, BindingIdent, BlockStmt, Bool, CallExpr, Class,
  ComputedPropName, Constructor, Expr, Function, Ident, IdentName, JSXText,
  Lit, MemberExpr, MemberProp, Null, Number, PrivateName, Prop, PropName,
  PropOrSpread, Regex, Str, Tpl,
};
use deno_ast::swc::common::DUMMY_SP;
use deno_ast::swc::ecma_visit::{VisitMut, VisitMutWith};
use deno_ast::swc::utils::{find_pat_ids, ident::IdentLike};
use deno_ast::view::{self as ast_view};
use deno_ast::Scope;

/// Extracts regex string from an expression, using ScopeManager.
/// If the passed expression is not regular expression, this will return `None`.
pub(crate) fn extract_regex(
  scope: &Scope,
  expr_ident: &ast_view::Ident,
  expr_args: &[&ast_view::ExprOrSpread],
) -> Option<String> {
  if expr_ident.inner.sym != *"RegExp" {
    return None;
  }

  if scope.var(&expr_ident.inner.to_id()).is_some() {
    return None;
  }

  match expr_args.first() {
    Some(first_arg) => match first_arg.expr {
      ast_view::Expr::Lit(ast_view::Lit::Str(literal)) => {
        Some(literal.inner.value.to_string_lossy().into_owned())
      }
      ast_view::Expr::Lit(ast_view::Lit::Regex(regex)) => {
        Some(regex.inner.exp.to_string())
      }
      _ => None,
    },
    None => None,
  }
}

pub(crate) trait StringRepr {
  fn string_repr(&self) -> Option<String>;
}

impl StringRepr for Str {
  fn string_repr(&self) -> Option<String> {
    Some(self.value.to_string_lossy().into_owned())
  }
}

impl StringRepr for Bool {
  fn string_repr(&self) -> Option<String> {
    let s = if self.value { "true" } else { "false" };
    Some(s.to_string())
  }
}

impl StringRepr for Null {
  fn string_repr(&self) -> Option<String> {
    Some("null".to_string())
  }
}

impl StringRepr for Number {
  fn string_repr(&self) -> Option<String> {
    Some(self.value.to_string())
  }
}

impl StringRepr for BigInt {
  fn string_repr(&self) -> Option<String> {
    Some(self.value.to_string())
  }
}

impl StringRepr for Regex {
  fn string_repr(&self) -> Option<String> {
    Some(format!("/{}/", self.exp))
  }
}

impl StringRepr for JSXText {
  fn string_repr(&self) -> Option<String> {
    Some(self.raw.to_string())
  }
}

impl StringRepr for Ident {
  fn string_repr(&self) -> Option<String> {
    Some(self.sym.to_string())
  }
}

impl StringRepr for IdentName {
  fn string_repr(&self) -> Option<String> {
    Some(self.sym.to_string())
  }
}

impl StringRepr for PropOrSpread {
  fn string_repr(&self) -> Option<String> {
    use PropOrSpread::*;
    match self {
      Prop(p) => (**p).string_repr(),
      Spread(_) => None,
    }
  }
}

impl StringRepr for Prop {
  fn string_repr(&self) -> Option<String> {
    use Prop::*;
    match self {
      KeyValue(key_value) => key_value.key.string_repr(),
      Getter(getter) => getter.key.string_repr(),
      Setter(setter) => setter.key.string_repr(),
      Method(method) => method.key.string_repr(),
      Shorthand(_) => None,
      Assign(_) => None,
    }
  }
}

impl StringRepr for Lit {
  fn string_repr(&self) -> Option<String> {
    match self {
      Lit::Str(s) => s.string_repr(),
      Lit::Bool(b) => b.string_repr(),
      Lit::Null(n) => n.string_repr(),
      Lit::Num(n) => n.string_repr(),
      Lit::BigInt(b) => b.string_repr(),
      Lit::Regex(r) => r.string_repr(),
      Lit::JSXText(j) => j.string_repr(),
    }
  }
}

impl StringRepr for Tpl {
  fn string_repr(&self) -> Option<String> {
    if self.exprs.is_empty() {
      self.quasis.first().map(|q| q.raw.to_string())
    } else {
      None
    }
  }
}

impl StringRepr for Expr {
  fn string_repr(&self) -> Option<String> {
    match self {
      Expr::Ident(ident) => ident.string_repr(),
      Expr::Lit(lit) => lit.string_repr(),
      Expr::Tpl(tpl) => tpl.string_repr(),
      _ => None,
    }
  }
}

impl StringRepr for PropName {
  fn string_repr(&self) -> Option<String> {
    match self {
      PropName::Ident(i) => i.string_repr(),
      PropName::Str(s) => s.string_repr(),
      PropName::Num(n) => n.string_repr(),
      PropName::BigInt(b) => b.string_repr(),
      PropName::Computed(ComputedPropName { ref expr, .. }) => match &**expr {
        Expr::Lit(lit) => lit.string_repr(),
        Expr::Tpl(tpl) => tpl.string_repr(),
        _ => None,
      },
    }
  }
}

impl StringRepr for PrivateName {
  fn string_repr(&self) -> Option<String> {
    Some(self.name.to_string())
  }
}

impl StringRepr for MemberExpr {
  fn string_repr(&self) -> Option<String> {
    self.prop.string_repr()
  }
}

impl StringRepr for MemberProp {
  fn string_repr(&self) -> Option<String> {
    match self {
      MemberProp::Ident(ident) => ident.string_repr(),
      MemberProp::PrivateName(name) => name.string_repr(),
      MemberProp::Computed(_) => None,
    }
  }
}

impl<S: StringRepr> StringRepr for Option<S> {
  fn string_repr(&self) -> Option<String> {
    self.as_ref().and_then(|k| k.string_repr())
  }
}

macro_rules! impl_string_repr_for_ast_view {
  ($($i:ident),* $(,)?) => {
    $(
      impl<'view> StringRepr for ast_view::$i<'view> {
        fn string_repr(&self) -> Option<String> {
          self.inner.string_repr()
        }
      }
    )*
  }
}

impl_string_repr_for_ast_view!(
  Ident,
  IdentName,
  Tpl,
  PrivateName,
  MemberExpr,
  Str,
  Bool,
  Null,
  Number,
  BigInt,
  Regex,
  JSXText,
);

impl StringRepr for ast_view::PropOrSpread<'_> {
  fn string_repr(&self) -> Option<String> {
    use deno_ast::view::PropOrSpread::*;
    match self {
      Prop(p) => p.string_repr(),
      Spread(_) => None,
    }
  }
}

impl StringRepr for ast_view::Prop<'_> {
  fn string_repr(&self) -> Option<String> {
    use deno_ast::view::Prop::*;
    match self {
      KeyValue(key_value) => key_value.key.string_repr(),
      Getter(getter) => getter.key.string_repr(),
      Setter(setter) => setter.key.string_repr(),
      Method(method) => method.key.string_repr(),
      Shorthand(_) => None,
      Assign(_) => None,
    }
  }
}

impl StringRepr for ast_view::Lit<'_> {
  fn string_repr(&self) -> Option<String> {
    use deno_ast::view::Lit::*;
    match self {
      Str(s) => s.string_repr(),
      Bool(b) => b.string_repr(),
      Null(n) => n.string_repr(),
      Num(n) => n.string_repr(),
      BigInt(b) => b.string_repr(),
      Regex(r) => r.string_repr(),
      JSXText(j) => j.string_repr(),
    }
  }
}

impl StringRepr for ast_view::Expr<'_> {
  fn string_repr(&self) -> Option<String> {
    use deno_ast::view::Expr::*;
    match self {
      Ident(ident) => ident.string_repr(),
      Lit(lit) => lit.string_repr(),
      Tpl(tpl) => tpl.string_repr(),
      _ => None,
    }
  }
}

impl StringRepr for ast_view::PropName<'_> {
  fn string_repr(&self) -> Option<String> {
    use deno_ast::view::PropName::*;
    match self {
      Ident(i) => i.string_repr(),
      Str(s) => s.string_repr(),
      Num(n) => n.string_repr(),
      BigInt(b) => b.string_repr(),
      Computed(ast_view::ComputedPropName { ref expr, .. }) => match expr {
        ast_view::Expr::Lit(lit) => lit.string_repr(),
        ast_view::Expr::Tpl(tpl) => tpl.string_repr(),
        _ => None,
      },
    }
  }
}

/// Find `Id`s in the lhs of an assigmnet expression.
pub(crate) fn find_lhs_ids<I>(n: &ast_view::AssignTarget) -> Vec<I>
where
  I: IdentLike,
{
  match &n {
    ast_view::AssignTarget::Simple(e) => match e {
      ast_view::SimpleAssignTarget::Ident(i) => vec![I::from_ident(i.id.inner)],
      _ => vec![],
    },
    ast_view::AssignTarget::Pat(p) => match p {
      ast_view::AssignTargetPat::Array(node) => find_pat_ids(node.inner),
      ast_view::AssignTargetPat::Object(node) => find_pat_ids(node.inner),
      ast_view::AssignTargetPat::Invalid(_) => Vec::new(),
    },
  }
}

pub fn span_and_ctx_drop<T>(mut t: T) -> T
where
  T: VisitMutWith<DropSpanAndCtx>,
{
  t.visit_mut_with(&mut DropSpanAndCtx {});
  t
}

pub struct DropSpanAndCtx;
impl VisitMut for DropSpanAndCtx {
  #[allow(clippy::disallowed_types)]
  fn visit_mut_span(&mut self, span: &mut deno_ast::swc::common::Span) {
    *span = DUMMY_SP;
  }

  fn visit_mut_ident(&mut self, node: &mut Ident) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }

  fn visit_mut_binding_ident(&mut self, node: &mut BindingIdent) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }

  fn visit_mut_arrow_expr(&mut self, node: &mut ArrowExpr) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }

  fn visit_mut_block_stmt(&mut self, node: &mut BlockStmt) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }

  fn visit_mut_call_expr(&mut self, node: &mut CallExpr) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }

  fn visit_mut_class(&mut self, node: &mut Class) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }

  fn visit_mut_constructor(&mut self, node: &mut Constructor) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }

  fn visit_mut_function(&mut self, node: &mut Function) {
    node.ctxt = Default::default();
    node.visit_mut_children_with(self);
  }
}
