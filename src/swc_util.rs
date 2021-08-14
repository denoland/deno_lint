// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::scopes::Scope;
use dprint_swc_ecma_ast_view as ast_view;
use swc_ecmascript::ast::{
  BigInt, Bool, ComputedPropName, Expr, ExprOrSpread, Ident, JSXText, Lit,
  MemberExpr, Null, Number, PatOrExpr, PrivateName, Prop, PropName,
  PropOrSpread, Regex, Str, Tpl,
};
use swc_ecmascript::utils::{find_ids, ident::IdentLike};

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

pub(crate) trait StringRepr {
  fn string_repr(&self) -> Option<String>;
}

impl StringRepr for Str {
  fn string_repr(&self) -> Option<String> {
    Some(self.value.to_string())
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

impl StringRepr for PropOrSpread {
  fn string_repr(&self) -> Option<String> {
    use PropOrSpread::*;
    match self {
      Prop(p) => (&**p).string_repr(),
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
      self.quasis.get(0).and_then(|q| q.raw.string_repr())
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
    self.id.string_repr()
  }
}

impl StringRepr for MemberExpr {
  fn string_repr(&self) -> Option<String> {
    if let Expr::Ident(ident) = &*self.prop {
      if !self.computed {
        return ident.string_repr();
      }
    }

    (&*self.prop).string_repr()
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

impl<'view> StringRepr for ast_view::PropOrSpread<'view> {
  fn string_repr(&self) -> Option<String> {
    use ast_view::PropOrSpread::*;
    match self {
      Prop(p) => p.string_repr(),
      Spread(_) => None,
    }
  }
}

impl<'view> StringRepr for ast_view::Prop<'view> {
  fn string_repr(&self) -> Option<String> {
    use ast_view::Prop::*;
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

impl<'view> StringRepr for ast_view::Lit<'view> {
  fn string_repr(&self) -> Option<String> {
    use ast_view::Lit::*;
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

impl<'view> StringRepr for ast_view::Expr<'view> {
  fn string_repr(&self) -> Option<String> {
    use ast_view::Expr::*;
    match self {
      Ident(ident) => ident.string_repr(),
      Lit(lit) => lit.string_repr(),
      Tpl(tpl) => tpl.string_repr(),
      _ => None,
    }
  }
}

impl<'view> StringRepr for ast_view::PropName<'view> {
  fn string_repr(&self) -> Option<String> {
    use ast_view::PropName::*;
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
