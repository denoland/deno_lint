use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::Tags;

use deno_ast::oxc::ast::ast::{
  BindingPattern, ExportSpecifier, ImportSpecifier, ModuleExportName,
  ObjectPattern, Program, PropertyKey,
};
use deno_ast::oxc::span::GetSpan;

#[derive(Debug)]
pub struct NoUselessRename;

const MESSAGE: &str = "The original name is exactly the same as the new name.";
const HINT: &str = "Remove the rename operation.";
const CODE: &str = "no-useless-rename";

impl LintRule for NoUselessRename {
  fn tags(&self) -> Tags {
    &[]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoUselessRenameHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoUselessRenameHandler;

impl Handler<'_> for NoUselessRenameHandler {
  fn import_specifier(&mut self, node: &ImportSpecifier, ctx: &mut Context) {
    // In OXC, ImportSpecifier always has `imported` and `local`.
    // If they have the same span, there's no rename (e.g., `import { foo }` has
    // imported = foo and local = foo with the same span).
    // If they differ in span but have the same name, it's a useless rename.
    let imported_name = match &node.imported {
      ModuleExportName::IdentifierName(name) => name.name.as_str(),
      ModuleExportName::IdentifierReference(ident) => ident.name.as_str(),
      ModuleExportName::StringLiteral(s) => s.value.as_str(),
    };
    // If imported and local have the same span, there was no explicit rename
    if node.imported.span() == node.local.span {
      return;
    }
    if imported_name == node.local.name.as_str() {
      ctx.add_diagnostic_with_hint(node.span, CODE, MESSAGE, HINT);
    }
  }

  fn object_pattern(&mut self, node: &ObjectPattern, ctx: &mut Context) {
    for prop in &node.properties {
      if prop.shorthand {
        continue;
      }

      let PropertyKey::StaticIdentifier(prop_key) = &prop.key else {
        continue;
      };

      let BindingPattern::BindingIdentifier(prop_value) = &prop.value else {
        continue;
      };

      if prop_value.name.as_str() == prop_key.name.as_str() {
        ctx.add_diagnostic_with_hint(node.span, CODE, MESSAGE, HINT);
      }
    }
  }

  fn export_specifier(&mut self, node: &ExportSpecifier, ctx: &mut Context) {
    // In OXC, ExportSpecifier always has `local` and `exported`.
    // If they have the same span, there's no explicit rename.
    if node.local.span() == node.exported.span() {
      return;
    }

    let local_name = match &node.local {
      ModuleExportName::IdentifierName(name) => name.name.as_str(),
      ModuleExportName::IdentifierReference(ident) => ident.name.as_str(),
      ModuleExportName::StringLiteral(s) => s.value.as_str(),
    };

    let exported_name = match &node.exported {
      ModuleExportName::IdentifierName(name) => name.name.as_str(),
      ModuleExportName::IdentifierReference(ident) => ident.name.as_str(),
      ModuleExportName::StringLiteral(s) => s.value.as_str(),
    };

    if local_name == exported_name {
      ctx.add_diagnostic_with_hint(node.span, CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn console_allowed() {
    assert_lint_ok!(
      NoUselessRename,
      r#"import { foo as bar } from "foo";"#,
      r#"const { foo: bar } = obj;"#,
      r#"export { foo as bar };"#,
    );
  }

  #[test]
  fn no_console_invalid() {
    assert_lint_err!(
        NoUselessRename,
        r#"import { foo as foo } from "foo";"#: [{
          col: 9,
          message: MESSAGE,
          hint: HINT,
        }],
        r#"const { foo: foo } = obj;"#: [{
          col: 6,
          message: MESSAGE,
          hint: HINT,
        }],
        r#"export { foo as foo };"#: [{
          col: 9,
          message: MESSAGE,
          hint: HINT,
        }]
    );
  }
}
