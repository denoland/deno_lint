// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use derive_more::Display;
use std::cmp::Ordering;
use std::collections::BTreeMap;
use swc_common::Span;
use swc_ecmascript::ast::{
  BigInt, Bool, Class, ClassMethod, ComputedPropName, Expr, Ident, Lit,
  MethodKind, Null, Number, PropName, Str, Tpl,
};
use swc_ecmascript::visit::{noop_visit_type, Node, Visit, VisitWith};

pub struct NoDupeClassMembers;

const CODE: &str = "no-dupe-class-members";

#[derive(Display)]
enum NoDupeClassMembersMessage {
  #[display(fmt = "Duplicate name '{}'", _0)]
  Duplicate(String),
}

#[derive(Display)]
enum NoDupeClassMembersHint {
  #[display(fmt = "Rename or remove the function with the duplicated name")]
  RenameOrRemove,
}

impl LintRule for NoDupeClassMembers {
  fn new() -> Box<Self> {
    Box::new(NoDupeClassMembers)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoDupeClassMembersVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows using a class member function name more than once

Declaring a function of the same name twice in a class will cause the previous
declaration(s) to be overwritten, causing unexpected behaviors.
    
### Invalid:
```typescript
class Foo {
  bar() {}
  bar() {}
}
```

### Valid:
```typescript
class Foo {
  bar() {}
  fizz() {}
}
```
"#
  }
}

struct NoDupeClassMembersVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoDupeClassMembersVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }

  fn add_diagnostic(&mut self, span: Span, name: &str) {
    self.context.add_diagnostic_with_hint(
      span,
      CODE,
      NoDupeClassMembersMessage::Duplicate(name.to_string()),
      NoDupeClassMembersHint::RenameOrRemove,
    );
  }
}

impl<'c> Visit for NoDupeClassMembersVisitor<'c> {
  noop_visit_type!();

  fn visit_class(&mut self, class: &Class, _: &dyn Node) {
    let mut visitor = ClassVisitor::new(self);
    class.visit_children_with(&mut visitor);
    visitor.aggregate_dupes();
  }
}

struct ClassVisitor<'a, 'b> {
  root_visitor: &'b mut NoDupeClassMembersVisitor<'a>,
  appeared_methods: BTreeMap<MethodToCheck, Vec<(Span, String)>>,
}

impl<'a, 'b> ClassVisitor<'a, 'b> {
  fn new(root_visitor: &'b mut NoDupeClassMembersVisitor<'a>) -> Self {
    Self {
      root_visitor,
      appeared_methods: BTreeMap::new(),
    }
  }

  fn aggregate_dupes(&mut self) {
    let root_visitor = &mut self.root_visitor;

    self
      .appeared_methods
      .values()
      .filter(|m| m.len() >= 2)
      .flatten()
      .for_each(|(span, name)| {
        root_visitor.add_diagnostic(*span, name);
      });
  }
}

impl<'a, 'b> Visit for ClassVisitor<'a, 'b> {
  noop_visit_type!();

  fn visit_class(&mut self, class: &Class, _: &dyn Node) {
    let mut visitor = ClassVisitor::new(self.root_visitor);
    class.visit_children_with(&mut visitor);
    visitor.aggregate_dupes();
  }

  fn visit_class_method(&mut self, class_method: &ClassMethod, _: &dyn Node) {
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
    class_method.visit_children_with(self);
  }
}

fn normalize_prop_name(name: &PropName) -> Option<String> {
  let normalized = match *name {
    PropName::Ident(Ident { ref sym, .. }) => sym.to_string(),
    PropName::Str(Str { ref value, .. }) => value.to_string(),
    PropName::Num(Number { ref value, .. }) => value.to_string(),
    PropName::BigInt(BigInt { ref value, .. }) => value.to_string(),
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

    !matches!(
      (self.kind, other.kind),
      (MethodKind::Getter, MethodKind::Setter)
        | (MethodKind::Setter, MethodKind::Getter)
    )
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

  #[test]
  fn no_dupe_class_members_valid() {
    assert_lint_ok! {
      NoDupeClassMembers,
      r#"
class Foo {
  bar() {}
  qux() {}
}
      "#,
      r#"
class Foo {
  get bar() {}
  set bar(value: number) {}
}
      "#,
      r#"
class Foo {
  static bar() {}
  bar() {}
}
      "#,
      r#"
class Foo {
  static bar() {}
  get bar() {}
  set bar(value: number) {}
}
      "#,
      r#"
class A { foo() {} }
class B { foo() {} }
      "#,
      r#"
class Foo {
  [bar]() {}
  bar() {}
}
      "#,
      r#"
class Foo {
  'bar'() {}
  'baz'() {}
  qux() {}
}
      "#,
      r#"
class Foo {
  *'bar'() {}
  *'baz'() {}
  *qux() {}
}
      "#,
      r#"
class Foo {
  get 'bar'() {}
  get 'baz'() {}
  get qux() {}
}
      "#,
      r#"
class Foo {
  1() {}
  2() {}
}
      "#,
      r#"
class Foo {
  ['bar']() {}
  ['baz']() {}
}
      "#,
      r#"
class Foo {
  [`bar`]() {}
  [`baz`]() {}
}
      "#,
      r#"
class Foo {
  [12]() {}
  [123]() {}
}
      "#,
      r#"
class Foo {
  [1.0]() {}
  ['1.0']() {}
}
      "#,
      r#"
class Foo {
  [0x1]() {}
  [`0x1`]() {}
}
      "#,
      r#"
class Foo {
  [null]() {}
  ['']() {}
}
      "#,
      r#"
class Foo {
  get ['bar']() {}
  set ['bar'](value: number) {}
}
      "#,
      r#"
class Foo {
  ['bar']() {}
  static ['bar']() {}
}
      "#,
      r#"
class Foo {
  ['constructor']() {}
  constructor() {}
}
      "#,
      r#"
class Foo {
  'constructor'() {}
  [`constructor`]() {}
}
      "#,
      r#"
class Foo {
  contrructor() {}
  get [`constructor`]() {}
}
      "#,
      r#"
class Foo {
  contrructor() {}
  set [`constructor`](value: number) {}
}
      "#,
      r#"
class Foo {
  ['bar' + '']() {}
  ['bar']() {}
}
      "#,
      r#"
class Foo {
  [`bar${''}`]() {}
  [`bar`]() {}
}
      "#,
      r#"
class Foo {
  [-1]() {}
  ['-1']() {}
}
      "#,
      r#"
class Foo {
  [foo]() {}
  [foo]() {}
}
      "#,
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
      r#"
class Foo {
  bar(v1: number): number;
  bar(v1: string, v2: boolean): string;
  bar(v1: number | string, v2?: boolean): number | string {}
}
      "#,
    };
  }

  #[test]
  fn no_dupe_class_members_invalid() {
    assert_lint_err! {
      NoDupeClassMembers,
      r#"
class Foo {
  bar() {}
  bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
!class Foo {
  bar() {}
  bar() {}
};
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  'bar'() {}
  'bar'() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  10() {}
  1e1() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "10"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "10"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  ['bar']() {}
  ['bar']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static ['bar']() {}
  static bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  set 'bar'(value: number) {}
  set ['bar'](val: number) {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  ''() {}
  ['']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, ""),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, ""),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [`bar`]() {}
  [`bar`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static get [`bar`]() {}
  static get ['bar']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar() {}
  [`bar`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  get [`bar`]() {}
  'bar'() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static 'bar'() {}
  static [`bar`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  ['constructor']() {}
  ['constructor']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static [`constructor`]() {}
  static ['constructor']() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "constructor"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [123]() {}
  [123]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [0x10]() {}
  16() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "16"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "16"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [100]() {}
  [1e2]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "100"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "100"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [123.00]() {}
  [`123`]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static '65'() {}
  static [0o101]() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "65"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "65"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [123n]() {}
  123() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "123"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  [null]() {}
  'null'() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "null"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "null"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar() {}
  get bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar() {}
  bar() {}
  get bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 5,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  static bar() {}
  static bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  set bar(value: number) {}
  bar() {}
}
      "#: [
        {
          line: 3,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 4,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  foo() {
    class Bar {
      set bar(value: number) {}
      bar() {}
    }
  }
}
      "#: [
        {
          line: 5,
          col: 6,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 6,
          col: 6,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ],
      r#"
class Foo {
  bar(v1: number): number;
  bar(v1: string, v2: boolean): string;
  bar(v1: number | string, v2?: boolean): number | string {}
  set bar(value: number) {}
}
      "#: [
        {
          line: 5,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        },
        {
          line: 6,
          col: 2,
          message: variant!(NoDupeClassMembersMessage, Duplicate, "bar"),
          hint: NoDupeClassMembersHint::RenameOrRemove,
        }
      ]
    };
  }
}
