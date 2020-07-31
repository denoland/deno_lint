// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::swc_common::Span;
use crate::swc_ecma_ast;
use crate::swc_ecma_ast::{
  BigInt, Bool, Class, ClassMethod, ComputedPropName, Expr, Ident, Lit,
  MethodKind, Null, Number, PropName, Str, Tpl,
};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

use std::sync::Arc;

pub struct NoDupeClassMembers;

impl LintRule for NoDupeClassMembers {
  fn new() -> Box<Self> {
    Box::new(NoDupeClassMembers)
  }

  fn code(&self) -> &'static str {
    "no-dupe-class-members"
  }

  fn lint_module(&self, context: Arc<Context>, module: &swc_ecma_ast::Module) {
    let mut visitor = NoDupeClassMembersVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct NoDupeClassMembersVisitor {
  context: Arc<Context>,
}

impl NoDupeClassMembersVisitor {
  fn new(context: Arc<Context>) -> Self {
    Self { context }
  }

  fn add_diagnostic(&self, span: Span, name: &str) {
    self.context.add_diagnostic(
      span,
      "no-dupe-class-members",
      &format!("Duplicate name '{}'", name),
    );
  }
}

impl Visit for NoDupeClassMembersVisitor {
  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let mut visitor = ClassVisitor::new(self);
    visitor.visit_class(class, parent);
    visitor.aggregate_dupes();
  }
}

struct ClassVisitor<'a> {
  root_visitor: &'a NoDupeClassMembersVisitor,
  appeared_methods: BTreeMap<MethodToCheck, Vec<(Span, String)>>,
}

impl<'a> ClassVisitor<'a> {
  fn new(root_visitor: &'a NoDupeClassMembersVisitor) -> Self {
    Self {
      root_visitor,
      appeared_methods: BTreeMap::new(),
    }
  }

  fn aggregate_dupes(&self) {
    self
      .appeared_methods
      .values()
      .filter(|m| m.len() >= 2)
      .flatten()
      .for_each(|(span, name)| {
        self.root_visitor.add_diagnostic(*span, name);
      });
  }
}

impl<'a> Visit for ClassVisitor<'a> {
  fn visit_class(&mut self, class: &Class, parent: &dyn Node) {
    let mut visitor = ClassVisitor::new(self.root_visitor);
    swc_ecmascript::visit::visit_class(&mut visitor, class, parent);
    visitor.aggregate_dupes();
  }

  fn visit_class_method(
    &mut self,
    class_method: &ClassMethod,
    parent: &dyn Node,
  ) {
    if class_method.function.body.is_some() {
      if let Some(m) = MethodToCheck::new(
        &class_method.key,
        class_method.kind,
        class_method.is_static,
      ) {
        let name = m.normalized_name.clone();
        self
          .appeared_methods
          .entry(m)
          .or_insert_with(Vec::new)
          .push((class_method.span, name));
      }
    }
    swc_ecmascript::visit::visit_class_method(self, class_method, parent);
  }
}

fn normalize_prop_name(name: &PropName) -> Option<String> {
  let normalized = match *name {
    PropName::Ident(Ident { ref sym, .. }) => sym.to_string(),
    PropName::Str(Str { ref value, .. }) => value.to_string(),
    PropName::Num(Number { ref value, .. }) => value.to_string(),
    PropName::Computed(ComputedPropName { ref expr, .. }) => match &**expr {
      Expr::Lit(Lit::Str(Str { ref value, .. })) => value.to_string(),
      Expr::Lit(Lit::Bool(Bool { ref value, .. })) => value.to_string(),
      Expr::Lit(Lit::Null(Null { .. })) => "null".to_string(),
      Expr::Lit(Lit::Num(Number { ref value, .. })) => value.to_string(),
      Expr::Lit(Lit::BigInt(BigInt { ref value, .. })) => value.to_string(),
      Expr::Tpl(Tpl {
        ref quasis,
        ref exprs,
        ..
      }) if exprs.is_empty() => {
        quasis.iter().next().map(|q| q.raw.value.to_string())?
      }
      _ => return None,
    },
  };

  Some(normalized)
}

struct MethodToCheck {
  normalized_name: String,
  kind: MethodKind,
  is_static: bool,
}

impl MethodToCheck {
  fn new(name: &PropName, kind: MethodKind, is_static: bool) -> Option<Self> {
    let normalized_name = normalize_prop_name(name)?;
    Some(Self {
      normalized_name,
      kind,
      is_static,
    })
  }
}

impl PartialEq for MethodToCheck {
  fn eq(&self, other: &Self) -> bool {
    if self.normalized_name != other.normalized_name {
      return false;
    }

    if self.is_static != other.is_static {
      return false;
    }

    match (self.kind, other.kind) {
      (MethodKind::Getter, MethodKind::Setter)
      | (MethodKind::Setter, MethodKind::Getter) => false,
      _ => true,
    }
  }
}

impl Eq for MethodToCheck {}

impl PartialOrd for MethodToCheck {
  fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
    Some(self.cmp(other))
  }
}

impl Ord for MethodToCheck {
  fn cmp(&self, other: &Self) -> Ordering {
    self
      .normalized_name
      .cmp(&other.normalized_name)
      .then(self.is_static.cmp(&other.is_static))
      .then_with(|| match (self.kind, other.kind) {
        (MethodKind::Getter, MethodKind::Setter) => Ordering::Less,
        (MethodKind::Setter, MethodKind::Getter) => Ordering::Greater,
        _ => Ordering::Equal,
      })
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_dupe_class_members_valid() {
    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  bar() {}
  qux() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  get bar() {}
  set bar(value: number) {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  static bar() {}
  bar() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  static bar() {}
  get bar() {}
  set bar(value: number) {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class A { foo() {} }
class B { foo() {} }
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [bar]() {}
  bar() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  'bar'() {}
  'baz'() {}
  qux() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  *'bar'() {}
  *'baz'() {}
  *qux() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  get 'bar'() {}
  get 'baz'() {}
  get qux() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  1() {}
  2() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  ['bar']() {}
  ['baz']() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [`bar`]() {}
  [`baz`]() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [12]() {}
  [123]() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [1.0]() {}
  ['1.0']() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [0x1]() {}
  [`0x1`]() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [null]() {}
  ['']() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  get ['bar']() {}
  set ['bar'](value: number) {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  ['bar']() {}
  static ['bar']() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  ['constructor']() {}
  constructor() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  'constructor'() {}
  [`constructor`]() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  contrructor() {}
  get [`constructor`]() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  contrructor() {}
  set [`constructor`](value: number) {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  ['bar' + '']() {}
  ['bar']() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [`bar${''}`]() {}
  [`bar`]() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [-1]() {}
  ['-1']() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  [foo]() {}
  [foo]() {}
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  foo() {
    class Bar {
      foo() {}
    }
    foo();
  }
}
      "#,
    );

    assert_lint_ok::<NoDupeClassMembers>(
      r#"
class Foo {
  bar(v1: number): number;
  bar(v1: string, v2: boolean): string;
  bar(v1: number | string, v2?: boolean): number | string {}
}
      "#,
    );
  }

  #[test]
  fn no_dupe_class_members_invalid() {
    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  bar() {}
  bar() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
!class Foo {
  bar() {}
  bar() {}
};
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  'bar'() {}
  'bar'() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  10() {}
  1e1() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  ['bar']() {}
  ['bar']() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  static ['bar']() {}
  static bar() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  set 'bar'(value: number) {}
  set ['bar'](val: number) {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  ''() {}
  ['']() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  [`bar`]() {}
  [`bar`]() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  static get [`bar`]() {}
  static get ['bar']() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  bar() {}
  [`bar`]() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  get [`bar`]() {}
  'bar'() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  static 'bar'() {}
  static [`bar`]() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  ['constructor']() {}
  ['constructor']() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  static [`constructor`]() {}
  static ['constructor']() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  [123]() {}
  [123]() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  [0x10]() {}
  16() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  [100]() {}
  [1e2]() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  [123.00]() {}
  [`123`]() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  static '65'() {}
  static [0o101]() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  [123n]() {}
  123() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  [null]() {}
  'null'() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  bar() {}
  get bar() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  bar() {}
  bar() {}
  get bar() {}
}
      "#,
      vec![(3, 2), (4, 2), (5, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  static bar() {}
  static bar() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  set bar(value: number) {}
  bar() {}
}
      "#,
      vec![(3, 2), (4, 2)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  foo() {
    class Bar {
      set bar(value: number) {}
      bar() {}
    }
  }
}
      "#,
      vec![(5, 6), (6, 6)],
    );

    assert_lint_err_on_line_n::<NoDupeClassMembers>(
      r#"
class Foo {
  bar(v1: number): number;
  bar(v1: string, v2: boolean): string;
  bar(v1: number | string, v2?: boolean): number | string {}
  set bar(value: number) {}
}
      "#,
      vec![(5, 2), (6, 2)],
    );
  }
}
