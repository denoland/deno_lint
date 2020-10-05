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
// Start of structs and enums
struct ImportIdent {
  import_decl: String,
  span: Span,
  import_type: ImportTypes,
}

#[derive(Copy, Clone, Debug, PartialEq)]
enum ImportTypes {
  None,
  All,
  Multiple,
  Single,
}

pub struct SortImportsOptions {
  ignore_case: bool,
  ignore_declaration_sort: bool,
  ignore_member_sort: bool,
  member_syntax_sort_order: Vec<ImportTypes>,
}
// End of structs and enums

// Start of helper functions
fn str_to_import_types(import_type_str: &str) -> ImportTypes {
  match import_type_str {
    "none" => ImportTypes::None,
    "all" => ImportTypes::All,
    "multiple" => ImportTypes::Multiple,
    "single" => ImportTypes::Single,
    &_ => ImportTypes::None,
  }
}

fn import_types_to_string(import_type: &ImportTypes) -> String {
  match import_type {
    ImportTypes::None => String::from("none"),
    ImportTypes::All => String::from("all"),
    ImportTypes::Multiple => String::from("multiple"),
    ImportTypes::Single => String::from("single"),
  }
}

fn config_to_enum(config: [&str; 4]) -> Vec<ImportTypes> {
  config
    .iter()
    .map(|str_slice| str_to_import_types(str_slice))
    .collect::<Vec<ImportTypes>>()
}
// End of helper functions

impl ImportIdent {
  fn new(
    import_decl: String,
    span: Span,
    import_type: ImportTypes,
  ) -> ImportIdent {
    ImportIdent {
      import_decl,
      span,
      import_type,
    }
  }
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
        member_syntax_sort_order: config_to_enum([
          "none", "all", "multiple", "single",
        ]),
      },
      line_imports: vec![],
    }
  }

  fn get_err_index(
    &self,
    import_specifiers: &Vec<ImportIdent>,
    report_multiple: Option<bool>,
  ) -> (Option<usize>, Option<Vec<usize>>, Option<Vec<usize>>) {
    let get_sortable_name = if self.options.ignore_case {
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
    let mut error_indices: Vec<usize> = vec![];
    let mut unexpected_order_indicies: Vec<usize> = vec![];
    for (index, sth) in first.enumerate() {
      if report_multiple.is_some() && index != &import_specifiers.len() - 1 {
        let f = self
          .get_member_param_grp_index(import_specifiers[index].import_type)
          .unwrap();

        let g = self
          .get_member_param_grp_index(import_specifiers[index + 1].import_type)
          .unwrap();

        if f != g {
          if g < f {
            unexpected_order_indicies.push(index + 1);
          }
          continue;
        }
      }

      if index != &import_specifiers.len() - 1 {
        let bs = &sorted[index + 1];
        let mut som: Vec<String> = vec![bs.to_string(), sth.to_string()];
        som.sort();
        if &som[0] != sth {
          first_unsorted_index = Some(index + 1);
          if report_multiple.is_some() {
            error_indices.push(index + 1)
          }
          if report_multiple.is_some() {
            continue;
          } else {
            break;
          }
        }
      };
    }

    (
      first_unsorted_index,
      if error_indices.len() > 0 {
        Some(error_indices)
      } else {
        None
      },
      if unexpected_order_indicies.len() > 0 {
        Some(unexpected_order_indicies)
      } else {
        None
      },
    )
  }

  fn get_member_param_grp_index(&self, variant: ImportTypes) -> Option<usize> {
    self
      .options
      .member_syntax_sort_order
      .iter()
      .position(|import_type| variant == import_type.clone())
  }

  fn sort_import_decl(&mut self, import_specifiers: &Vec<ImportIdent>) {
    if !self.options.ignore_member_sort {
      let (a, _, _) = self.get_err_index(&import_specifiers, None);
      if let Some(n) = a {
        let mut err_string = String::from("Member '");
        err_string.push_str(&import_specifiers[n].import_decl);
        err_string.push_str(
          "' of the import declaration should be sorted alphabetically",
        );
        self.context.add_diagnostic(
          import_specifiers[n].span,
          "sort-imports",
          &err_string,
        );
        return;
      }
    }
  }

  fn sort_line_imports(&mut self) {
    let (_, b, c) = self.get_err_index(&self.line_imports, Some(true));
    if let Some(vec_n) = b {
      for n in vec_n.into_iter() {
        self.context.add_diagnostic(
          self.line_imports[n].span,
          "sort-imports",
          "Imports should be sorted alphabetically",
        );
      }
    }
    if let Some(vec_r) = c {
      for n in vec_r.into_iter() {
        let mut err_string = String::from("Expected '");
        err_string
          .push_str(&import_types_to_string(&self.line_imports[n].import_type));
        err_string.push_str("' syntax before '");
        err_string.push_str(&import_types_to_string(
          &self.line_imports[n - 1].import_type,
        ));
        err_string.push_str("' syntax");
        self.context.add_diagnostic(
          self.line_imports[n].span,
          "sort-imports",
          &err_string,
        );
      }
    }
  }

  fn handle_import_decl(&mut self, import_stmt: &ImportDecl) {
    let specifiers = &import_stmt.specifiers;
    let mut import_ident_vec: Vec<ImportIdent> = vec![];
    let mut import_ident: ImportIdent =
      ImportIdent::new(String::from(""), import_stmt.span, ImportTypes::None);
    for (index, specifier) in specifiers.iter().enumerate() {
      match specifier {
        ImportSpecifier::Named(named_specifier) => {
          import_ident_vec.push(ImportIdent::new(
            named_specifier.local.sym.get(0..).unwrap().to_string(),
            named_specifier.local.span,
            if specifiers.len() > 1 {
              ImportTypes::Multiple
            } else {
              ImportTypes::Single
            },
          ));
          if index == 0 {
            import_ident = ImportIdent::new(
              named_specifier.local.sym.get(0..).unwrap().to_string(),
              import_stmt.span,
              if specifiers.len() > 1 {
                ImportTypes::Multiple
              } else {
                ImportTypes::Single
              },
            );
          }
        }
        ImportSpecifier::Default(specifier) => {
          import_ident = ImportIdent::new(
            specifier.local.sym.get(0..).unwrap().to_string(),
            import_stmt.span,
            ImportTypes::Single,
          );
        }
        ImportSpecifier::Namespace(specifier) => {
          import_ident = ImportIdent::new(
            specifier.local.sym.get(0..).unwrap().to_string(),
            import_stmt.span,
            ImportTypes::All,
          );
        }
      }
    }
    self.line_imports.push(import_ident);
    if !self.options.ignore_declaration_sort {
      self.sort_import_decl(&import_ident_vec);
    }
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
    // Sort imports alphabetically
    assert_lint_err_on_line::<SortImports>(
      "import a from 'foo.js';\nimport A from 'bar.js';",
      2,
      0,
    );
    assert_lint_err_on_line::<SortImports>(
      "import b from 'foo.js';\nimport a from 'bar.js';",
      2,
      0,
    );
    assert_lint_err_on_line::<SortImports>(
      "import {b, c} from 'foo.js';\nimport {a, d} from 'bar.js';",
      2,
      0,
    );
    assert_lint_err_on_line::<SortImports>(
      "import * as foo from 'foo.js';\nimport * as bar from 'bar.js';",
      2,
      0,
    );

    // Unexpected syntax order
    assert_lint_err_on_line::<SortImports>(
      "import a from 'foo.js';\nimport {b, c} from 'bar.js';",
      2,
      0,
    );
    assert_lint_err_on_line::<SortImports>(
      "import a from 'foo.js';\nimport * as b from 'bar.js';",
      2,
      0,
    );
    assert_lint_err_on_line::<SortImports>(
      "import a from 'foo.js';\nimport 'bar.js';",
      2,
      0,
    );

    // Sort members alphabetically
    assert_lint_err_on_line::<SortImports>(
      "import {b, a, d, c} from 'foo.js';\nimport {e, f, g, h} from 'bar.js';",
      1,
      11,
    );
    assert_lint_err_on_line::<SortImports>(
      "import {a, B, c, D} from 'foo.js';",
      1,
      11,
    );
    assert_lint_err_on_line::<SortImports>(
      "import {zzzzz, /* comment */ aaaaa} from 'foo.js';",
      1,
      29,
    );
    assert_lint_err_on_line::<SortImports>(
      "import {zzzzz /* comment */, aaaaa} from 'foo.js';",
      1,
      29,
    );
    assert_lint_err_on_line::<SortImports>(
      "import {/* comment */ zzzzz, aaaaa} from 'foo.js';",
      1,
      29,
    );
    assert_lint_err_on_line::<SortImports>(
      "import {zzzzz, aaaaa /* comment */} from 'foo.js';",
      1,
      15,
    );
    assert_lint_err_on_line::<SortImports>(
      r#"import {
      boop,
      foo,
      zoo,
      baz as qux,
      bar,
      beep
    } from 'foo.js';"#,
      5,
      13,
    );
  }
}
