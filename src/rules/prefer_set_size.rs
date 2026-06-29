// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::{self, Tags};
use crate::Program;
use deno_ast::swc::ast::Id;
use deno_ast::swc::ast::VarDeclKind;
use deno_ast::view::{
  CallExpr, Callee, Expr, MemberExpr, MemberProp, Node, Pat, VarDecl,
};
use deno_ast::SourceRanged;
use derive_more::Display;
use std::collections::HashSet;

#[derive(Debug)]
pub struct PreferSetSize;

const CODE: &str = "prefer-set-size";

#[derive(Display)]
enum PreferSetSizeMessage {
  #[display(
    fmt = "Use `Set#size` instead of converting a `Set` to an array and using its `length` property."
  )]
  Unexpected,
}

#[derive(Display)]
enum PreferSetSizeHint {
  #[display(fmt = "Replace array conversion with direct `Set.size` access")]
  UseSize,
}

impl LintRule for PreferSetSize {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    let mut collector = SetVarCollector::default();
    collector.traverse(program, context);
    let mut handler = PreferSetSizeHandler {
      set_vars: collector.set_vars,
    };
    handler.traverse(program, context);
  }
}

/// Strips any enclosing parentheses from an expression.
fn unwrap_parens(mut expr: Expr) -> Expr {
  while let Expr::Paren(paren) = expr {
    expr = paren.expr;
  }
  expr
}

/// Returns `true` if the expression is a direct `new Set(...)` call.
fn is_new_set(expr: &Expr) -> bool {
  let Expr::New(new_expr) = expr else {
    return false;
  };
  let Expr::Ident(callee) = &new_expr.callee else {
    return false;
  };
  callee.sym() == "Set"
}

/// Pre-pass that collects the identifiers of `const` variables that are
/// initialized directly with `new Set(...)` and bound to a simple identifier
/// (not a destructuring pattern).
#[derive(Default)]
struct SetVarCollector {
  set_vars: HashSet<Id>,
}

impl Handler for SetVarCollector {
  fn var_decl(&mut self, var_decl: &VarDecl, _ctx: &mut Context) {
    if var_decl.decl_kind() != VarDeclKind::Const {
      return;
    }
    for declarator in var_decl.decls {
      let Pat::Ident(binding) = &declarator.name else {
        continue;
      };
      let Some(init) = declarator.init else {
        continue;
      };
      if is_new_set(&unwrap_parens(init)) {
        self.set_vars.insert(binding.id.inner.to_id());
      }
    }
  }
}

struct PreferSetSizeHandler {
  set_vars: HashSet<Id>,
}

impl PreferSetSizeHandler {
  /// Returns `true` if the expression is a `Set`: either a direct
  /// `new Set(...)`, or an identifier referring to a qualifying `const`
  /// `new Set(...)` variable.
  fn is_set(&self, expr: &Expr) -> bool {
    if is_new_set(expr) {
      return true;
    }
    let Expr::Ident(ident) = expr else {
      return false;
    };
    self.set_vars.contains(&ident.inner.to_id())
  }
}

/// If `expr` is a conversion of a value to an array (`[...value]` or
/// `Array.from(value)`), returns the converted value.
fn get_set_node<'a>(expr: Expr<'a>) -> Option<Expr<'a>> {
  match expr {
    // `[...set]`
    Expr::Array(array_lit) => {
      if array_lit.elems.len() != 1 {
        return None;
      }
      let elem = array_lit.elems[0]?;
      if elem.spread().is_none() {
        return None;
      }
      Some(elem.expr)
    }
    // `Array.from(set)`
    Expr::Call(call_expr) => {
      if !is_array_from_call(call_expr) {
        return None;
      }
      let arg = call_expr.args.first()?;
      if arg.spread().is_some() {
        return None;
      }
      Some(arg.expr)
    }
    _ => None,
  }
}

/// Returns `true` if `call_expr` is a non-optional, non-computed
/// `Array.from(...)` call with exactly one argument.
fn is_array_from_call(call_expr: &CallExpr) -> bool {
  let Callee::Expr(callee) = &call_expr.callee else {
    return false;
  };
  let Expr::Member(member) = callee else {
    return false;
  };
  let Expr::Ident(obj) = &member.obj else {
    return false;
  };
  if obj.sym() != "Array" {
    return false;
  }
  let MemberProp::Ident(prop) = &member.prop else {
    return false;
  };
  if prop.sym() != "from" {
    return false;
  }
  call_expr.args.len() == 1
}

impl Handler for PreferSetSizeHandler {
  fn member_expr(&mut self, member_expr: &MemberExpr, ctx: &mut Context) {
    // `?.length` is represented as an optional chain wrapping this member.
    if matches!(member_expr.parent(), Node::OptChainExpr(_)) {
      return;
    }

    // The accessed property must be the non-computed identifier `length`.
    let MemberProp::Ident(prop) = &member_expr.prop else {
      return;
    };
    if prop.sym() != "length" {
      return;
    }

    let Some(set_expr) = get_set_node(unwrap_parens(member_expr.obj)) else {
      return;
    };

    if !self.is_set(&unwrap_parens(set_expr)) {
      return;
    }

    ctx.add_diagnostic_with_hint(
      member_expr.range(),
      CODE,
      PreferSetSizeMessage::Unexpected,
      PreferSetSizeHint::UseSize,
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/prefer_set_size.rs
  // MIT Licensed.

  #[test]
  fn prefer_set_size_valid() {
    assert_lint_ok! {
      PreferSetSize,
      "new Set(foo).size",
      "for (const foo of bar) console.log([...foo].length)",
      "[...new Set(array), foo].length",
      "[foo, ...new Set(array), ].length",
      "[...new Set(array)].notLength",
      "[...new Set(array)]?.length",
      "[...new Set(array)][length]",
      r#"[...new Set(array)]["length"]"#,
      "[...new NotSet(array)].length",
      "[...Set(array)].length",
      "const foo = new NotSet([]);[...foo].length;",
      "let foo = new Set([]);[...foo].length;",
      "const {foo} = new Set([]);[...foo].length;",
      "const [foo] = new Set([]);[...foo].length;",
      "[...foo].length",
      "var foo = new Set(); var foo = new Set(); [...foo].length",
      "[,].length",
      "Array.from(foo).length",
      "Array.from(new NotSet(array)).length",
      "Array.from(Set(array)).length",
      "Array.from(new Set(array)).notLength",
      "Array.from(new Set(array))?.length",
      "Array.from(new Set(array))[length]",
      r#"Array.from(new Set(array))["length"]"#,
      "Array.from(new Set(array), mapFn).length",
      "Array?.from(new Set(array)).length",
      "Array.from?.(new Set(array)).length",
      "const foo = new NotSet([]);Array.from(foo).length;",
      "let foo = new Set([]);Array.from(foo).length;",
      "const {foo} = new Set([]);Array.from(foo).length;",
      "const [foo] = new Set([]);Array.from(foo).length;",
      "var foo = new Set(); var foo = new Set(); Array.from(foo).length",
      "NotArray.from(new Set(array)).length",
    };
  }

  #[test]
  fn prefer_set_size_invalid() {
    assert_lint_err! {
      PreferSetSize,
      "[...new Set(array)].length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "const foo = new Set([]);
            console.log([...foo].length);": [
        {
          line: 2,
          col: 24,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "function isUnique(array) {
                return[...new Set(array)].length === array.length
            }": [
        {
          line: 2,
          col: 22,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "[...new Set(array),].length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "[...(( new Set(array) ))].length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "(( [...new Set(array)] )).length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "foo
            ;[...new Set(array)].length": [
        {
          line: 2,
          col: 13,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "[/* comment */...new Set(array)].length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "[...new /* comment */ Set(array)].length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "Array.from(new Set(array)).length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "const foo = new Set([]);
            console.log(Array.from(foo).length);": [
        {
          line: 2,
          col: 24,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "Array.from((( new Set(array) ))).length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "(( Array.from(new Set(array)) )).length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "Array.from(/* comment */ new Set(array)).length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "Array.from(new /* comment */ Set(array)).length": [
        {
          col: 0,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ],
      "function isUnique(array) {
                return Array.from(new Set(array)).length === array.length
            }": [
        {
          line: 2,
          col: 23,
          message: PreferSetSizeMessage::Unexpected,
          hint: PreferSetSizeHint::UseSize,
        }
      ]
    };
  }
}
