// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::sync::Arc;
use swc_common::Span;
use swc_ecmascript::ast::ImportDecl;
use swc_ecmascript::ast::ImportSpecifier;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

// need to rename vars

struct ImportIdent {
  import_decl: String,
  span: Span,
}

impl ImportIdent {
  fn new(import_decl: String, span: Span) -> ImportIdent {
    ImportIdent { import_decl, span }
  }
}

fn get_err_index(
  import_specifiers: &Vec<ImportIdent>,
  ignore_case: bool,
) -> Option<usize> {
  let get_sortable_name = if ignore_case {
    |specifier: &ImportIdent| specifier.import_decl.to_ascii_lowercase()
  } else {
    |specifier: &ImportIdent| specifier.import_decl.to_string()
  };

  let sorted = import_specifiers
    .iter()
    .map(get_sortable_name)
    .collect::<Vec<String>>();
  let first = sorted.iter();
  let mut first_unsorted_index: Option<usize> = None;
  for (index, sth) in first.enumerate() {
    if index != &import_specifiers.len() - 1 {
      let bs = &sorted[index + 1];
      let mut som: Vec<String> = vec![bs.to_string(), sth.to_string()];
      som.sort();
      if &som[0] != sth {
        first_unsorted_index = Some(index + 1);
        break;
      }
    };
  }
  first_unsorted_index
}

#[allow(dead_code)]
pub struct SortImportsOptions {
  ignore_case: bool,
  ignore_declaration_sort: bool,
  ignore_member_sort: bool,
  member_syntax_sort_order: [String; 4],
  allow_separated_groups: bool,
}

pub struct SortImports;

impl LintRule for SortImports {
  fn new() -> Box<Self> {
    Box::new(SortImports)
  }

  fn code(&self) -> &'static str {
    "sort-imports"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = SortImportsVisitor::default(context);
    visitor.visit_module(module, module);
    visitor.sort_line_imports();
  }
}

struct SortImportsVisitor {
  context: Arc<Context>,
  options: SortImportsOptions,
  line_imports: Vec<ImportIdent>,
}

impl SortImportsVisitor {
  pub fn default(context: Arc<Context>) -> Self {
    Self {
      context,
      options: SortImportsOptions {
        ignore_case: false,
        ignore_declaration_sort: false,
        ignore_member_sort: false,
        member_syntax_sort_order: [
          String::from("none"),
          String::from("all"),
          String::from("multiple"),
          String::from("single"),
        ],
        allow_separated_groups: false,
      },
      line_imports: vec![],
    }
  }

  fn sort_import_decl(&mut self, import_specifiers: &Vec<ImportIdent>) {
    if let Some(n) = get_err_index(&import_specifiers, self.options.ignore_case)
    {
      let mut err_string = String::from("Member '");
      err_string.push_str(&import_specifiers[n].import_decl);
      err_string.push_str(
        "' of the import declaration should be sorted alphabetically.",
      );
      self.context.add_diagnostic(
        import_specifiers[n].span,
        "sort-imports",
        &err_string,
      );
      return;
    }
  }

  fn sort_line_imports(&mut self) {
    if let Some(n) = get_err_index(&self.line_imports, self.options.ignore_case)
    {
      self.context.add_diagnostic(
        self.line_imports[n].span,
        "sort-imports",
        "Imports should be sorted alphabetically.",
      );
      return;
    }
  }

  fn handle_import_decl(&mut self, import_stmt: &ImportDecl) -> () {
    let specifiers = &import_stmt.specifiers;
    let mut import_ident_vec: Vec<ImportIdent> = vec![];
    let mut import_ident: ImportIdent =
      ImportIdent::new(String::from(""), import_stmt.span);
    for (index, specifier) in specifiers.iter().enumerate() {
      match specifier {
        ImportSpecifier::Named(named_specifier) => {
          import_ident_vec.push(ImportIdent::new(
            named_specifier.local.sym.get(0..).unwrap().to_string(),
            named_specifier.local.span,
          ));
          if index == 0 {
            import_ident = ImportIdent::new(
              named_specifier.local.sym.get(0..).unwrap().to_string(),
              import_stmt.span,
            );
          }
        }
        ImportSpecifier::Default(specifier) => {
          import_ident = ImportIdent::new(
            specifier.local.sym.get(0..).unwrap().to_string(),
            import_stmt.span,
          );
        }
        _ => {}
      }
    }
    self.line_imports.push(import_ident);
    self.sort_import_decl(&import_ident_vec);
  }
}

impl Visit for SortImportsVisitor {
  fn visit_import_decl(
    &mut self,
    import_stmt: &ImportDecl,
    _parent: &dyn Node,
  ) {
    self.handle_import_decl(import_stmt);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn sort_imports_test() {
    assert_lint_ok::<SortImports>(r#"import { b, B } from 'react';"#);
  }
}
