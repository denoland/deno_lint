// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use derive_more::Display;
use swc_ecmascript::ast::TsInterfaceDecl;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoEmptyInterface;

const CODE: &str = "no-empty-interface";

#[derive(Display)]
enum NoEmptyInterfaceMessage {
  #[display(fmt = "An empty interface is equivalent to `{{}}`.")]
  EmptyObject,
  #[display(
    fmt = "An interface declaring no members is equivalent to its supertype."
  )]
  Supertype,
}

#[derive(Display)]
enum NoEmptyInterfaceHint {
  #[display(fmt = "Remove this interface or add members to this interface.")]
  RemoveOrAddMember,
  #[display(
    fmt = "Use the supertype instead, or add members to this interface."
  )]
  UseSuperTypeOrAddMember,
}

impl LintRule for NoEmptyInterface {
  fn new() -> Box<Self> {
    Box::new(NoEmptyInterface)
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
    let mut visitor = NoEmptyInterfaceVisitor::new(context);
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the declaration of an empty interface

An interface with no members serves no purpose.  Either the interface extends
another interface, in which case the supertype can be used, or it does not
extend a supertype in which case it is the equivalent to an empty object.  This
rule will capture these situations as either unnecessary code or a mistaken
empty implementation.
    
### Invalid:
```typescript
interface Foo {}
interface Foo extends Bar {}
```

### Valid:
```typescript
interface Foo {
  name: string;
}

interface Bar {
  age: number;
}

// Using an empty interface as a union type is allowed
interface Baz extends Foo, Bar {}
```
"#
  }
}

struct NoEmptyInterfaceVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoEmptyInterfaceVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoEmptyInterfaceVisitor<'c> {
  fn visit_ts_interface_decl(
    &mut self,
    interface_decl: &TsInterfaceDecl,
    _parent: &dyn Node,
  ) {
    if interface_decl.extends.len() <= 1 && interface_decl.body.body.is_empty()
    {
      self.context.add_diagnostic_with_hint(
        interface_decl.span,
        CODE,
        if interface_decl.extends.is_empty() {
          NoEmptyInterfaceMessage::EmptyObject
        } else {
          NoEmptyInterfaceMessage::Supertype
        },
        if interface_decl.extends.is_empty() {
          NoEmptyInterfaceHint::RemoveOrAddMember
        } else {
          NoEmptyInterfaceHint::UseSuperTypeOrAddMember
        },
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_interface_valid() {
    assert_lint_ok! {
      NoEmptyInterface,
      "interface Foo { a: string }",
      "interface Foo { a: number }",

      // This is valid because an interface with more than one supertype
      // can be used as a replacement of a union type.
      "interface Foo extends Bar, Baz {}",
    };
  }

  #[test]
  fn no_empty_interface_invalid() {
    assert_lint_err! {
      NoEmptyInterface,
      "interface Foo {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::EmptyObject,
          hint: NoEmptyInterfaceHint::RemoveOrAddMember,
        }
      ],
      "interface Foo extends {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::EmptyObject,
          hint: NoEmptyInterfaceHint::RemoveOrAddMember,
        }
      ],
      r#"
interface Foo {
  a: string;
}

interface Bar extends Foo {}
"#: [
        {
          line: 6,
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      "interface Foo extends Array<number> {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      "interface Foo extends Array<number | {}> {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      r#"
interface Foo {
  a: string;
}

interface Bar extends Array<Foo> {}
"#: [
        {
          line: 6,
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      r#"
type R = Record<string, unknown>;
interface Foo extends R {}
"#: [
        {
          line: 3,
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      "interface Foo<T> extends Bar<T> {}": [
        {
          col: 0,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ],
      r#"
declare module FooBar {
  type Baz = typeof baz;
  export interface Bar extends Baz {}
}
"#: [
        {
          line: 4,
          col: 9,
          message: NoEmptyInterfaceMessage::Supertype,
          hint: NoEmptyInterfaceHint::UseSuperTypeOrAddMember,
        }
      ]
    };
  }
}
