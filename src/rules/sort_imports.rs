// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use std::sync::Arc;
use swc_common::Span;
use swc_ecmascript::ast::ImportDecl;
use swc_ecmascript::ast::ImportDefaultSpecifier;
use swc_ecmascript::ast::ImportNamedSpecifier;
use swc_ecmascript::ast::ImportSpecifier;
use swc_ecmascript::ast::ImportStarAsSpecifier;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

enum ErrMsgTypes {
  SortImportsAlphabetically,
  SortMembersAlphabetically,
}

struct ImportIdent {
  import_decl: String,
  span: Span,
}

impl ImportIdent {
  fn new(import_decl: String, span: Span) -> ImportIdent {
    ImportIdent { import_decl, span }
  }
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
    visitor.sort_import_ident(None);
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

  fn sort_import_ident(&mut self, import_ident_vec: Option<&Vec<ImportIdent>>) {
    let mut err_type: ErrMsgTypes = ErrMsgTypes::SortMembersAlphabetically;
    let ident_vec = match import_ident_vec {
      Some(vec) => vec,
      None => {
        err_type = ErrMsgTypes::SortImportsAlphabetically;
        &self.line_imports
      }
    };

    let mut vec_srings: Vec<String> = ident_vec
      .iter()
      .map(|s| {
        if self.options.ignore_case {
          s.import_decl.to_string().to_ascii_lowercase()
        } else {
          s.import_decl.to_string()
        }
      })
      .collect();
    vec_srings.sort();
    for (index, s) in vec_srings.iter().enumerate() {
      let mut import_decl = ident_vec[index].import_decl.to_string();
      if self.options.ignore_case {
        import_decl = import_decl.to_lowercase();
      }
      if s != &import_decl {
        let mut err_string = String::from("");
        match err_type {
          ErrMsgTypes::SortImportsAlphabetically => {
            err_string.push_str("Imports should be sorted alphabetically.");
          }
          ErrMsgTypes::SortMembersAlphabetically => {
            err_string.push_str("Member ");
            err_string.push_str(&import_decl);
            err_string.push_str(
              " of the import declaration should be sorted alphabetically.",
            );
          }
        }
        self.context.add_diagnostic(
          ident_vec[index].span,
          "sort-imports",
          &err_string,
        );
      }
    }
  }

  fn sort_import_decl(&mut self, import_specifiers: &Vec<ImportIdent>) {
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
    for (index, sth) in first.enumerate() {
      if index != &import_specifiers.len() - 1 {
        let bs = &sorted[index + 1];
        let mut som: Vec<String> = vec![bs.to_string(), sth.to_string()];
        som.sort();
        if &som[0] != sth {
          first_unsorted_index = Some(index + 1);
        }
      };
    }
    if let Some(n) = first_unsorted_index {
      let mut err_string = String::from("Member ");
      err_string.push_str(&sorted[n]);
      err_string.push_str(
        " of the import declaration should be sorted alphabetically.",
      );
      self.context.add_diagnostic(
        import_specifiers[n].span,
        "sort-imports",
        &err_string,
      );
      return;
    }
  }

  fn handle_import_decl(&mut self, import_stmt: &ImportDecl) -> () {
    let specifiers = &import_stmt.specifiers;
    let mut import_ident_vec: Vec<ImportIdent> = vec![];
    let mut import_ident: ImportIdent =
      ImportIdent::new(String::from("hi"), import_stmt.span);
    for specifier in specifiers.iter() {
      if let ImportSpecifier::Named(named_specifier) = &specifier {
        match &named_specifier.imported {
          Some(renamed_import) => {
            import_ident_vec.push(ImportIdent::new(
              renamed_import.sym.get(0..).unwrap().to_string(),
              renamed_import.span,
            ));
            import_ident = ImportIdent::new(
              renamed_import.sym.get(0..).unwrap().to_string(),
              import_stmt.span,
            );
          }
          None => {
            import_ident_vec.push(ImportIdent::new(
              named_specifier.local.sym.get(0..).unwrap().to_string(),
              named_specifier.local.span,
            ));
            import_ident = ImportIdent::new(
              named_specifier.local.sym.get(0..).unwrap().to_string(),
              import_stmt.span,
            );
          }
        }
      }
    }

    self.line_imports.push(import_ident);

    // self.sort_import_ident(Some(&import_ident_vec));
    self.sort_import_decl(&import_ident_vec);
  }
  fn handle_import_default(
    &mut self,
    default_specifier: &ImportDefaultSpecifier,
  ) {
    self.line_imports.push(ImportIdent::new(
      default_specifier.local.sym.get(0..).unwrap().to_string(),
      default_specifier.span,
    ));
    println!(
      "{} a {}",
      default_specifier.local.sym.get(0..).unwrap().to_string(),
      self.line_imports.len()
    );
  }
}

impl Visit for SortImportsVisitor {
  fn visit_import_decl(
    &mut self,
    import_stmt: &ImportDecl,
    _parent: &dyn Node,
  ) {
    if import_stmt.specifiers.len() > 1 {
      if !self.options.ignore_member_sort {
        self.handle_import_decl(import_stmt);
      }
    };
  }
  fn visit_import_default_specifier(
    &mut self,
    import_stmt: &ImportDefaultSpecifier,
    _parent: &dyn Node,
  ) {
    println!("default_specifier {:?}", import_stmt.span);
    self.handle_import_default(import_stmt);

    if false {
      self.context.add_diagnostic(
        import_stmt.span,
        "sort-imports",
        "Sort imports correctly",
      );
    }
  }
  fn visit_import_named_specifier(
    &mut self,
    import_stmt: &ImportNamedSpecifier,
    _parent: &dyn Node,
  ) {
    println!("import_named_specifier: {:?}", import_stmt.local.span);
  }
  fn visit_import_star_as_specifier(
    &mut self,
    import_stmt: &ImportStarAsSpecifier,
    _parent: &dyn Node,
  ) {
    println!("import_star_as_specifier: {:?}", import_stmt.span);
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
