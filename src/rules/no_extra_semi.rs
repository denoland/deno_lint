// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::program_ref;
use super::{Context, LintRule};
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::{
  DoWhileStmt, EmptyStmt, ForInStmt, ForOfStmt, ForStmt, IfStmt, LabeledStmt,
  Stmt, WhileStmt, WithStmt,
};
use deno_ast::swc::visit::{noop_visit_type, Visit, VisitWith};
use deno_ast::SourceRangedForSpanned;
use derive_more::Display;
use std::sync::Arc;

#[derive(Debug)]
pub struct NoExtraSemi;

const CODE: &str = "no-extra-semi";

#[derive(Display)]
enum NoExtraSemiMessage {
  #[display(fmt = "Unnecessary semicolon.")]
  Unnecessary,
}

#[derive(Display)]
enum NoExtraSemiHint {
  #[display(fmt = "Remove the extra (and unnecessary) semi-colon")]
  Remove,
}

impl LintRule for NoExtraSemi {
  fn new() -> Arc<Self> {
    Arc::new(NoExtraSemi)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    let program = program_ref(program);
    let mut visitor = NoExtraSemiVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_extra_semi.md")
  }
}

struct NoExtraSemiVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoExtraSemiVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl<'c, 'view> Visit for NoExtraSemiVisitor<'c, 'view> {
  noop_visit_type!();

  fn visit_empty_stmt(&mut self, empty_stmt: &EmptyStmt) {
    self.context.add_diagnostic_with_hint(
      empty_stmt.range(),
      CODE,
      NoExtraSemiMessage::Unnecessary,
      NoExtraSemiHint::Remove,
    );
  }

  fn visit_for_stmt(&mut self, for_stmt: &ForStmt) {
    if matches!(&*for_stmt.body, Stmt::Empty(_)) {
      if let Some(ref init) = for_stmt.init {
        init.visit_with(self);
      }
      if let Some(ref test) = for_stmt.test {
        test.visit_with(self);
      }
      if let Some(ref update) = for_stmt.update {
        update.visit_with(self);
      }
    } else {
      for_stmt.visit_children_with(self);
    }
  }

  fn visit_while_stmt(&mut self, while_stmt: &WhileStmt) {
    if matches!(&*while_stmt.body, Stmt::Empty(_)) {
      while_stmt.test.visit_with(self);
    } else {
      while_stmt.visit_children_with(self);
    }
  }

  fn visit_do_while_stmt(&mut self, do_while_stmt: &DoWhileStmt) {
    if matches!(&*do_while_stmt.body, Stmt::Empty(_)) {
      do_while_stmt.test.visit_with(self);
    } else {
      do_while_stmt.visit_children_with(self);
    }
  }

  fn visit_with_stmt(&mut self, with_stmt: &WithStmt) {
    if matches!(&*with_stmt.body, Stmt::Empty(_)) {
      with_stmt.obj.visit_with(self);
    } else {
      with_stmt.visit_children_with(self);
    }
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt) {
    if matches!(&*for_of_stmt.body, Stmt::Empty(_)) {
      for_of_stmt.left.visit_with(self);
      for_of_stmt.right.visit_with(self);
    } else {
      for_of_stmt.visit_children_with(self);
    }
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt) {
    if matches!(&*for_in_stmt.body, Stmt::Empty(_)) {
      for_in_stmt.left.visit_with(self);
      for_in_stmt.right.visit_with(self);
    } else {
      for_in_stmt.visit_children_with(self);
    }
  }

  fn visit_if_stmt(&mut self, if_stmt: &IfStmt) {
    if_stmt.test.visit_with(self);
    match &*if_stmt.cons {
      Stmt::Empty(_) => {}
      cons => {
        cons.visit_with(self);
      }
    }
    match if_stmt.alt.as_deref() {
      None | Some(Stmt::Empty(_)) => {}
      Some(alt) => {
        alt.visit_with(self);
      }
    }
  }

  fn visit_labeled_stmt(&mut self, labeled_stmt: &LabeledStmt) {
    labeled_stmt.label.visit_with(self);
    match &*labeled_stmt.body {
      Stmt::Empty(_) => {}
      body => {
        body.visit_with(self);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_extra_semi_valid() {
    assert_lint_ok! {
      NoExtraSemi,
      "var x = 5;",
      "function foo(){}",
      "for(;;);",
      "while(0);",
      "do;while(0);",
      "for(a in b);",
      "for(a of b);",
      "if(true);",
      "if(true); else;",
      "foo: ;",
      "foo: bar: ;",
      "with(foo);",
      "class A { }",
      "var A = class { };",
      "class A { a() { this; } }",
      "var A = class { a() { this; } };",
      "class A { } a;",
    };
  }

  #[test]
  fn no_extra_semi_invalid() {
    assert_lint_err! {
      NoExtraSemi,
      "var x = 5;;": [
        {
          col: 10,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "function foo(){};": [
        {
          col: 16,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "for(;;);;": [
        {
          col: 8,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "while(0);;": [
        {
          col: 9,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "do;while(0);;": [
        {
          col: 12,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "for(a in b);;": [
        {
          col: 12,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "for(a of b);;": [
        {
          col: 12,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "if(true);;": [
        {
          col: 9,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "if(true){} else;;": [
        {
          col: 16,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "if(true){;} else {;}": [
        {
          col: 9,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        },
        {
          col: 18,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "foo:;;": [
        {
          col: 5,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "with(foo);;": [
        {
          col: 10,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "with(foo){;}": [
        {
          col: 10,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "class A { ; }": [
        {
          col: 10,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "class A { /*a*/; }": [
        {
          col: 15,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "class A { ; a() {} }": [
        {
          col: 10,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "class A { a() {}; }": [
        {
          col: 16,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "class A { a() {}; b() {} }": [
        {
          col: 16,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "class A {; a() {}; b() {}; }": [
        {
          col: 9,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        },
        {
          col: 17,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        },
        {
          col: 25,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      "class A { a() {}; get b() {} }": [
        {
          col: 16,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
for (let i = 0; i < n; i++) {
  for (;;);;
}
"#: [
        {
          line: 3,
          col: 11,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
while (a) {
  while (b);;
}
"#: [
        {
          line: 3,
          col: 12,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
do {
  do {
    ;
  } while(a);
} while(b);
"#: [
        {
          line: 4,
          col: 4,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
with(a) {
  with(b) {
    ;
  }
}
"#: [
        {
          line: 4,
          col: 4,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
for (const a of b) {
  for (const c of d) {
    ;
  }
}
"#: [
        {
          line: 4,
          col: 4,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
for (const a in b) {
  for (const c in d) {
    ;
  }
}
"#: [
        {
          line: 4,
          col: 4,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
if (a) {
  if (b) {
    ;
  } else;
}
"#: [
        {
          line: 4,
          col: 4,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
foo: {
  bar: {
    ;
  }
}
"#: [
        {
          line: 4,
          col: 4,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ],
      r#"
class A {
  foo() {
    class B { ; }
  }
}
"#: [
        {
          line: 4,
          col: 14,
          message: NoExtraSemiMessage::Unnecessary,
          hint: NoExtraSemiHint::Remove,
        }
      ]
    };
  }
}
