// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::scopes::Scope;
use swc_common::Span;
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::{
  ComputedPropName, Expr, ExprOrSpread, Ident, Lit, MemberExpr, PatOrExpr,
  PrivateName, Prop, PropName, PropOrSpread, Str, Tpl,
};
use swc_ecmascript::utils::{find_ids, ident::IdentLike};
use swc_ecmascript::visit::Fold;

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

impl Key for Ident {
  fn get_key(&self) -> Option<String> {
    Some(self.sym.to_string())
  }
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

impl Key for PrivateName {
  fn get_key(&self) -> Option<String> {
    Some(self.id.sym.to_string())
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

impl<K: Key> Key for Option<K> {
  fn get_key(&self) -> Option<String> {
    self.as_ref().and_then(|k| k.get_key())
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
