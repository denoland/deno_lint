use super::program_ref;
use super::{Context, LintRule};
use crate::tags::Tags;
use crate::Program;
use crate::ProgramRef;
use deno_ast::swc::ast::PrivateMethod;
use deno_ast::swc::ecma_visit::{Visit, VisitWith};
use deno_ast::SourceRangedForSpanned;
use derive_more::Display;

#[derive(Debug)]
pub struct NoRuntimePrivateMethods;

const CODE: &str = "no-runtime-private-methods";

#[derive(Display)]
enum NoRuntimePrivateMethodsMessage {
  #[display(fmt = "Avoid prefixing methods with `#`")]
  Default,
}

#[derive(Display)]
enum NoRuntimePrivateMethodsHint {
  #[display(fmt = "Use `private {}`", _0)]
  Default(String),
}

impl LintRule for NoRuntimePrivateMethods {
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
    let mut visitor = NoRuntimePrivateMethodsVisitor::new(context);
    match program {
      ProgramRef::Module(m) => m.visit_with(&mut visitor),
      ProgramRef::Script(s) => s.visit_with(&mut visitor),
    }
  }
}

struct NoRuntimePrivateMethodsVisitor<'c, 'view> {
  context: &'c mut Context<'view>,
}

impl<'c, 'view> NoRuntimePrivateMethodsVisitor<'c, 'view> {
  fn new(context: &'c mut Context<'view>) -> Self {
    Self { context }
  }
}

impl Visit for NoRuntimePrivateMethodsVisitor<'_, '_> {
  fn visit_private_method(&mut self, method: &PrivateMethod) {
    self.context.add_diagnostic_with_hint(
      method.range(),
      CODE,
      NoRuntimePrivateMethodsMessage::Default,
      NoRuntimePrivateMethodsHint::Default(method.key.name.to_string()),
    );
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_runtime_private_methods_valid() {
    assert_lint_ok! {
         NoRuntimePrivateMethods,
         "class Test { method() {}}",
         "class Test { _method() {}}",
         "class Test { private method() {}}",
         "class Test { public method() {}}",
         "class Test { protected method() {}}",
    };
  }

  #[test]
  fn no_runtime_private_methods_invalid() {
    assert_lint_err! {
         NoRuntimePrivateMethods,
         "class Test {
    #runtimePrivate() {}}":
         [
           {
             line: 2,
             col: 4,
             message: variant!(NoRuntimePrivateMethodsMessage, Default),
             hint: variant!(NoRuntimePrivateMethodsHint, Default, "runtimePrivate"),
           }
         ],
         "class Test {
    private #keywordAndRuntimePrivate() {}}":
         [
           {
             line: 2,
             col: 4,
             message: variant!(NoRuntimePrivateMethodsMessage, Default),
             hint: variant!(NoRuntimePrivateMethodsHint, Default, "keywordAndRuntimePrivate"),
           }
         ]
    }
  }
}
