use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::tags::Tags;
use crate::Program;

use deno_ast::view::{
  ExportNamedSpecifier, ImportNamedSpecifier, ModuleExportName, ObjectPat,
  ObjectPatProp, Pat, PropName,
};
use deno_ast::SourceRanged;

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

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoUselessRenameHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_useless_rename.md")
  }
}

struct NoUselessRenameHandler;

impl Handler for NoUselessRenameHandler {
  fn import_named_specifier(
    &mut self,
    node: &ImportNamedSpecifier,
    ctx: &mut Context,
  ) {
    if let Some(ModuleExportName::Ident(imported_name)) = node.imported {
      if imported_name.sym() == node.local.sym() {
        ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
      }
    }
  }

  fn object_pat(&mut self, node: &ObjectPat, ctx: &mut Context) {
    for prop in node.props {
      let ObjectPatProp::KeyValue(key_val) = prop else {
        return;
      };

      let PropName::Ident(prop_key) = key_val.key else {
        return;
      };

      let Pat::Ident(prop_value) = key_val.value else {
        return;
      };

      if prop_value.id.sym() == prop_key.sym() {
        ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
      }
    }
  }

  fn export_named_specifier(
    &mut self,
    node: &ExportNamedSpecifier,
    ctx: &mut Context,
  ) {
    let Some(exported) = node.exported else {
      return;
    };

    let ModuleExportName::Ident(exported_id) = exported else {
      return;
    };

    let ModuleExportName::Ident(original) = node.orig else {
      return;
    };

    if exported_id.sym() == original.sym() {
      ctx.add_diagnostic_with_hint(node.range(), CODE, MESSAGE, HINT);
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
