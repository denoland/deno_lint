use super::program_ref;
use super::{Context, LintRule};
use crate::tags::Tags;
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::{PrivateMethod, PrivateProp};
use deno_ast::swc::ecma_visit::{Visit, VisitWith};
use deno_ast::SourceRangedForSpanned;
use derive_more::Display;

#[derive(Debug)]
pub struct NoRuntimePrivateFields;

const CODE: &str = "no-runtime-private-fields";

#[derive(Display)]
enum NoRuntimePrivateFieldsMessage {
  #[display(fmt = "Avoid prefixing fields with `#`")]
  Default,
}

#[derive(Display)]
enum NoRuntimePrivateFieldsHint {
  #[display(fmt = "Use `private {}`", _0)]
  Default(String),
  #[display(fmt = "Use `static {}`", _0)]
  Static(String),
}

impl LintRule for NoRuntimePrivateFields {
  fn tags(&self) -> Tags {
    &[]
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
    let mut visitor = NoRuntimePrivateFieldsVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }
}

struct NoRuntimePrivateFieldsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoRuntimePrivateFieldsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl Visit for NoRuntimePrivateFieldsVisitor<'_, '_> {
  fn visit_private_method(&mut self, method: &PrivateMethod) {
    self.context.add_diagnostic_with_hint(
      method.range(),
      CODE,
      NoRuntimePrivateFieldsMessage::Default,
      NoRuntimePrivateFieldsHint::Default(method.key.name.to_string()),
    );
  }

  fn visit_private_prop(&mut self, prop: &PrivateProp) {
    if prop.is_static {
      self.context.add_diagnostic_with_hint(
        prop.range(),
        CODE,
        NoRuntimePrivateFieldsMessage::Default,
        NoRuntimePrivateFieldsHint::Static(prop.key.name.to_string()),
      );
      return;
    }

    self.context.add_diagnostic_with_hint(
      prop.range(),
      CODE,
      NoRuntimePrivateFieldsMessage::Default,
      NoRuntimePrivateFieldsHint::Default(prop.key.name.to_string()),
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_runtime_private_fields_valid() {
    assert_lint_ok! {
         NoRuntimePrivateFields,
         "class Test { testField = 0; }",
         "class Test { _testField = 1; }",
         "class Test { private testField = 2; }",
         "class Test { public testField = 3; }",
         "class Test { protected testField = 4; }",
         "class Test { static staticField = 5; }",
    };
  }

  #[test]
  fn no_runtime_private_fields_invalid() {
    assert_lint_err! {
         NoRuntimePrivateFields,
         "class Test {
    #runtimePrivate = 2;}":
         [
           {
             line: 2,
             col: 4,
             message: variant!(NoRuntimePrivateFieldsMessage, Default),
             hint: variant!(NoRuntimePrivateFieldsHint, Default, "runtimePrivate"),
           }
         ],
         "class Test {
    private #keywordAndRuntimePrivate = 3;}":
         [
           {
             line: 2,
             col: 4,
             message: variant!(NoRuntimePrivateFieldsMessage, Default),
             hint: variant!(NoRuntimePrivateFieldsHint, Default, "keywordAndRuntimePrivate"),
           }
         ],
         "class Test {
    static #STATIC_FIELD = 3;}":
         [
           {
             line: 2,
             col: 4,
             message: variant!(NoRuntimePrivateFieldsMessage, Default),
             hint: variant!(NoRuntimePrivateFieldsHint, Static, "STATIC_FIELD"),
           }
         ]
    }
  }
}
