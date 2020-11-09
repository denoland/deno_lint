// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use crate::scopes::Scope;
use swc_ecmascript::ast::{
  ComputedPropName, Expr, ExprOrSpread, Ident, Lit, MemberExpr, PatOrExpr,
  PrivateName, Prop, PropName, PropOrSpread, Str, Tpl,
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

impl StringRepr for Tpl {
  fn string_repr(&self) -> Option<String> {
    if self.exprs.is_empty() {
      self.quasis.get(0).map(|q| q.raw.value.to_string())
    } else {
      None
    }
  }
}

impl StringRepr for Expr {
  fn string_repr(&self) -> Option<String> {
    match self {
      Expr::Ident(ident) => Some(ident.sym.to_string()),
      Expr::Lit(lit) => lit.string_repr(),
      Expr::Tpl(tpl) => tpl.string_repr(),
      _ => None,
    }
  }
}

impl StringRepr for PropName {
  fn string_repr(&self) -> Option<String> {
    match self {
      PropName::Ident(identifier) => Some(identifier.sym.to_string()),
      PropName::Str(str) => Some(str.value.to_string()),
      PropName::Num(num) => Some(num.to_string()),
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
    Some(self.id.sym.to_string())
  }
}

impl StringRepr for MemberExpr {
  fn string_repr(&self) -> Option<String> {
    if let Expr::Ident(ident) = &*self.prop {
      if !self.computed {
        return Some(ident.sym.to_string());
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
