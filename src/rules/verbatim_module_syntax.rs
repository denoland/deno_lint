// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::diagnostic::{LintQuickFix, LintQuickFixChange};
use crate::Program;
use deno_ast::swc::ast::{
  BindingIdent, ExportNamedSpecifier, Id, Ident, ImportDecl, JSXElementName,
  NamedExport, TsEntityName, TsExprWithTypeArgs, TsImportEqualsDecl,
  TsModuleRef,
};
use deno_ast::swc::common::collections::AHashSet;
use deno_ast::swc::visit::{noop_visit_type, Visit, VisitWith};
use deno_ast::view::NodeTrait;
use deno_ast::{
  view as ast_view, SourceRange, SourceRanged, SourceRangedForSpanned,
};
use derive_more::Display;

#[derive(Debug)]
pub struct VerbatimModuleSyntax;

const CODE: &str = "verbatim-module-syntax";
const FIX_DESC: &str = "Add a type keyword";

#[derive(Display)]
enum Message {
  #[display(fmt = "All import identifiers are used in type positions")]
  AllIdentsUsedInTypePositions,
  #[display(fmt = "Import identifier only used in type positions")]
  IdentUsedInTypePositions,
}

#[derive(Display)]
enum Hint {
  #[display(
    fmt = "Change `import` to `import type` and optionally add an explicit side effect import"
  )]
  ChangeImportToImportType,
  #[display(fmt = "Add a `type` keyword before the identifier")]
  AddTypeKeyword,
}

impl LintRule for VerbatimModuleSyntax {
  fn tags(&self) -> &'static [&'static str] {
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
    let module = match program.program() {
      Program::Module(module) => module,
      Program::Script(_) => return,
    };
    let mut usages = UsageCollect::default();
    module.inner.visit_with(&mut usages);

    for child in module.body {
      match child {
        ast_view::ModuleItem::ModuleDecl(module_decl) => match module_decl {
          ast_view::ModuleDecl::Import(import) => {
            if !import.type_only() && !import.specifiers.is_empty() {
              let mut type_only_usage =
                Vec::with_capacity(import.specifiers.len());
              let mut type_only_named_import =
                Vec::with_capacity(import.specifiers.len());
              for specifier in import.specifiers {
                match specifier {
                  ast_view::ImportSpecifier::Named(named) => {
                    if named.is_type_only() {
                      type_only_named_import.push(named);
                    } else if !usages.id_usage.contains(&named.local.to_id()) {
                      type_only_usage.push(specifier);
                    }
                  }
                  ast_view::ImportSpecifier::Default(default) => {
                    if !usages.id_usage.contains(&default.local.to_id()) {
                      type_only_usage.push(specifier);
                    }
                  }
                  ast_view::ImportSpecifier::Namespace(namespace) => {
                    if !usages.id_usage.contains(&namespace.local.to_id()) {
                      type_only_usage.push(specifier);
                    }
                  }
                }
              }
              if import.specifiers.len()
                == type_only_usage.len() + type_only_named_import.len()
              {
                let import_token_range = import.tokens_fast(program)[0].range();
                let mut changes =
                  Vec::with_capacity(1 + type_only_named_import.len());
                changes.push(LintQuickFixChange {
                  new_text: " type".into(),
                  range: import_token_range.end().range(),
                });
                for named_import in type_only_named_import {
                  // remove `type` from all these
                  let tokens = named_import.tokens_fast(program);
                  let range =
                    SourceRange::new(tokens[0].start(), tokens[1].start());
                  changes.push(LintQuickFixChange {
                    new_text: "".into(),
                    range,
                  });
                }
                context.add_diagnostic_with_quick_fixes(
                  import_token_range,
                  CODE,
                  Message::AllIdentsUsedInTypePositions,
                  Some(Hint::ChangeImportToImportType.to_string()),
                  vec![LintQuickFix {
                    description: FIX_DESC.into(),
                    changes,
                  }],
                );
              } else {
                for specifier in type_only_usage {
                  context.add_diagnostic_with_quick_fixes(
                    specifier.range(),
                    CODE,
                    Message::IdentUsedInTypePositions,
                    Some(Hint::AddTypeKeyword.to_string()),
                    vec![LintQuickFix {
                      description: FIX_DESC.into(),
                      changes: vec![LintQuickFixChange {
                        new_text: "type ".into(),
                        range: specifier.start().range(),
                      }],
                    }],
                  );
                }
              }
            }
          }
          ast_view::ModuleDecl::ExportNamed(_)
          | ast_view::ModuleDecl::ExportDefaultDecl(_)
          | ast_view::ModuleDecl::ExportDefaultExpr(_)
          | ast_view::ModuleDecl::ExportAll(_)
          | ast_view::ModuleDecl::TsImportEquals(_)
          | ast_view::ModuleDecl::TsExportAssignment(_)
          | ast_view::ModuleDecl::TsNamespaceExport(_)
          | ast_view::ModuleDecl::ExportDecl(_) => {}
        },
        ast_view::ModuleItem::Stmt(_) => {}
      }
    }
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/verbatim_module_syntax.md")
  }
}

/// This struct is lifted and adapted from:
/// https://github.com/swc-project/swc/blob/d8186fb94efb150b50d96519f0b8c5740d15b92f/crates/swc_ecma_transforms_typescript/src/strip_import_export.rs#L9C1-L100C2
#[derive(Debug, Default)]
struct UsageCollect {
  id_usage: AHashSet<Id>,
}

impl Visit for UsageCollect {
  noop_visit_type!();

  fn visit_ident(&mut self, n: &Ident) {
    self.id_usage.insert(n.to_id());
  }

  fn visit_binding_ident(&mut self, _: &BindingIdent) {
    // skip
  }

  // todo: remove once https://github.com/swc-project/swc/pull/8677 has landed
  fn visit_ts_expr_with_type_args(&mut self, _: &TsExprWithTypeArgs) {}

  fn visit_import_decl(&mut self, _: &ImportDecl) {
    // skip
  }

  fn visit_ts_import_equals_decl(&mut self, n: &TsImportEqualsDecl) {
    if n.is_type_only {
      return;
    }

    // skip id visit

    let TsModuleRef::TsEntityName(ts_entity_name) = &n.module_ref else {
      return;
    };

    get_module_ident(ts_entity_name).visit_with(self);
  }

  fn visit_export_named_specifier(&mut self, n: &ExportNamedSpecifier) {
    if n.is_type_only {
      return;
    }

    n.orig.visit_with(self);
  }

  fn visit_named_export(&mut self, n: &NamedExport) {
    if n.type_only || n.src.is_some() {
      return;
    }

    n.visit_children_with(self);
  }

  fn visit_jsx_element_name(&mut self, n: &JSXElementName) {
    if matches!(n, JSXElementName::Ident(i) if i.sym.starts_with(|c: char| c.is_ascii_lowercase()) )
    {
      return;
    }

    n.visit_children_with(self);
  }
}

fn get_module_ident(ts_entity_name: &TsEntityName) -> &Ident {
  match ts_entity_name {
    TsEntityName::TsQualifiedName(ts_qualified_name) => {
      get_module_ident(&ts_qualified_name.left)
    }
    TsEntityName::Ident(ident) => ident,
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid() {
    assert_lint_ok! {
      VerbatimModuleSyntax,
      "import type { Type } from 'module'; type Test = Type;",
      "import type { Type, Other } from 'module'; type Test = Type | Other;",
      "import { type Type, value } from 'module'; type Test = Type; value();",
      "import * as value from 'module'; value();",
      "import type * as value from 'module'; type Test = typeof value;",
      "import value from 'module'; value();",
      "import type value from 'module'; type Test = typeof value;",
    };
  }

  #[test]
  fn invalid() {
    assert_lint_err! {
      VerbatimModuleSyntax,
      "import { Type } from 'module'; type Test = Type;": [
        {
          col: 0,
          message: Message::AllIdentsUsedInTypePositions,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { Type } from 'module'; type Test = Type;"),
        }
      ],
      "import { Type, Other } from 'module'; type Test = Type | Other;": [
        {
          col: 0,
          message: Message::AllIdentsUsedInTypePositions,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type { Type, Other } from 'module'; type Test = Type | Other;"),
        }
      ],
      "import { Type, value } from 'module'; type Test = Type; value();": [
        {
          col: 9,
          message: Message::IdentUsedInTypePositions,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { type Type, value } from 'module'; type Test = Type; value();"),
        }
      ],
      "import { Type, Other, value } from 'module'; type Test = Type | Other; value();": [
        {
          col: 9,
          message: Message::IdentUsedInTypePositions,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { type Type, Other, value } from 'module'; type Test = Type | Other; value();"),
        },
        {
          col: 15,
          message: Message::IdentUsedInTypePositions,
          hint: Hint::AddTypeKeyword,
          fix: (FIX_DESC, "import { Type, type Other, value } from 'module'; type Test = Type | Other; value();"),
        }
      ],
      "import * as value from 'module'; type Test = typeof value;": [
        {
          col: 0,
          message: Message::AllIdentsUsedInTypePositions,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type * as value from 'module'; type Test = typeof value;"),
        }
      ],
      "import value from 'module'; type Test = typeof value;": [
        {
          col: 0,
          message: Message::AllIdentsUsedInTypePositions,
          hint: Hint::ChangeImportToImportType,
          fix: (FIX_DESC, "import type value from 'module'; type Test = typeof value;"),
        }
      ],
    };
  }
}
