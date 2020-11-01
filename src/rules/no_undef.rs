// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use super::Context;
use super::LintRule;
use crate::globals::GLOBALS;
use swc_atoms::js_word;
use swc_ecmascript::{
  ast::*,
  visit::Node,
  visit::{noop_visit_type, Visit, VisitWith},
};

pub struct NoUndef;

const CODE: &str = "no-undef";

impl LintRule for NoUndef {
  fn new() -> Box<Self> {
    Box::new(NoUndef)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let decl_builder = decl_finder::DeclFinderBuilder::new();
    let decl_finder = decl_builder.build(program);

    let mut visitor = NoUndefVisitor::new(context, decl_finder);
    program.visit_with(program, &mut visitor);
  }
}

mod decl_finder {
  use std::cell::RefCell;
  use std::collections::{BTreeMap, BTreeSet};
  use std::rc::Rc;
  use swc_atoms::JsWord;
  use swc_common::{Span, Spanned, DUMMY_SP};
  use swc_ecmascript::ast::{
    ArrowExpr, BlockStmt, BlockStmtOrExpr, CatchClause, Class, ClassDecl,
    ClassExpr, Constructor, DoWhileStmt, FnDecl, FnExpr, ForInStmt, ForOfStmt,
    ForStmt, Function, Ident, IfStmt, ImportDefaultSpecifier,
    ImportNamedSpecifier, ImportStarAsSpecifier, Invalid, ObjectPatProp,
    ParamOrTsParamProp, Pat, Program, Stmt, TsEnumDecl, TsParamPropParam,
    VarDecl, VarDeclKind, VarDeclOrExpr, VarDeclOrPat, WhileStmt, WithStmt,
  };
  use swc_ecmascript::utils::find_ids;
  use swc_ecmascript::visit::{Node, Visit, VisitWith};

  type Scope = Rc<RefCell<RawScope>>;

  #[derive(Debug)]
  struct RawScope {
    parent: Option<Scope>,
    variables: BTreeSet<JsWord>,
  }

  impl RawScope {
    fn new(parent: Option<Scope>) -> Self {
      Self {
        parent,
        variables: BTreeSet::new(),
      }
    }
  }

  #[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug)]
  enum ScopeRange {
    Program,
    Block(Span),
  }

  #[derive(Debug)]
  pub(crate) struct DeclFinder {
    scopes: BTreeMap<ScopeRange, Scope>,
  }

  impl DeclFinder {
    /// Look for a variable declaration that corresponds the given ident by traversing from the scope
    /// where the ident is to the parent. If the declaration is found, it returns true.
    pub(crate) fn decl_exists(&self, ident: &Ident) -> bool {
      let ident_scope = self.find_scope(ident.span);
      let mut cur_scope = self.scopes.get(&ident_scope).map(Rc::clone);

      while let Some(scope) = cur_scope {
        if scope.borrow().variables.contains(&ident.sym) {
          return true;
        }
        cur_scope = scope.borrow().parent.as_ref().map(Rc::clone);
      }

      false
    }

    /// Find a scope to which the span directly belongs.
    fn find_scope(&self, span: Span) -> ScopeRange {
      // To do a search, create a dummy scope range although the span might not represent any
      // block.
      let dummy_scope_range = ScopeRange::Block(span);

      self
        .scopes
        .range(..dummy_scope_range)
        .rev()
        .find_map(|(range, _)| {
          if let ScopeRange::Block(stored_span) = range {
            if stored_span.hi() >= span.hi() {
              return Some(*range);
            }
          }
          None
        })
        .unwrap_or(ScopeRange::Program)
    }
  }

  #[derive(Debug)]
  pub(crate) struct DeclFinderBuilder {
    scopes: BTreeMap<ScopeRange, Scope>,
    cur_scope: ScopeRange,
  }

  impl DeclFinderBuilder {
    pub(crate) fn new() -> Self {
      Self {
        scopes: BTreeMap::new(),
        cur_scope: ScopeRange::Program,
      }
    }

    // TODO(magurotuna): remove this
    #[allow(unused)]
    pub(crate) fn build_from_module(
      mut self,
      module: &swc_ecmascript::ast::Module,
    ) -> DeclFinder {
      self.visit_module(module, &Invalid { span: DUMMY_SP });

      DeclFinder {
        scopes: self.scopes,
      }
    }

    pub(crate) fn build(mut self, program: &Program) -> DeclFinder {
      self.visit_program(program, &Invalid { span: DUMMY_SP });

      DeclFinder {
        scopes: self.scopes,
      }
    }

    fn insert_var(&mut self, ident: &Ident) {
      let mut scope = self.scopes.get(&self.cur_scope).unwrap().borrow_mut();
      scope.variables.insert(ident.sym.clone());
    }

    fn extract_decl_idents(&mut self, pat: &Pat) {
      match pat {
        Pat::Ident(ident) => self.insert_var(ident),
        Pat::Array(array_pat) => {
          for elem in &array_pat.elems {
            if let Some(elem_pat) = elem {
              self.extract_decl_idents(elem_pat);
            }
          }
        }
        Pat::Rest(rest_pat) => self.extract_decl_idents(&*rest_pat.arg),
        Pat::Object(object_pat) => {
          for prop in &object_pat.props {
            match prop {
              ObjectPatProp::KeyValue(key_value) => {
                self.extract_decl_idents(&*key_value.value)
              }
              ObjectPatProp::Assign(assign) => {
                self.insert_var(&assign.key);
              }
              ObjectPatProp::Rest(rest) => self.extract_decl_idents(&*rest.arg),
            }
          }
        }
        Pat::Assign(assign_pat) => self.extract_decl_idents(&*assign_pat.left),
        _ => {}
      }
    }

    fn with_child_scope<F, S>(&mut self, node: S, op: F)
    where
      S: Spanned,
      F: FnOnce(&mut Self),
    {
      let parent_scope_range = self.cur_scope;
      let parent_scope = self.scopes.get(&parent_scope_range).map(Rc::clone);
      let child_scope = RawScope::new(parent_scope);
      self.scopes.insert(
        ScopeRange::Block(node.span()),
        Rc::new(RefCell::new(child_scope)),
      );
      self.cur_scope = ScopeRange::Block(node.span());
      op(self);
      self.cur_scope = parent_scope_range;
    }
  }

  impl Visit for DeclFinderBuilder {
    fn visit_program(&mut self, program: &Program, _: &dyn Node) {
      let scope = RawScope::new(None);
      self
        .scopes
        .insert(ScopeRange::Program, Rc::new(RefCell::new(scope)));
      program.visit_children_with(self);
    }

    // TODO(magurotuna): remove this
    fn visit_module(
      &mut self,
      module: &swc_ecmascript::ast::Module,
      _: &dyn Node,
    ) {
      let scope = RawScope::new(None);
      self
        .scopes
        .insert(ScopeRange::Program, Rc::new(RefCell::new(scope)));
      module.visit_children_with(self);
    }

    fn visit_fn_decl(&mut self, fn_decl: &FnDecl, _: &dyn Node) {
      self.insert_var(&fn_decl.ident);
      fn_decl.visit_children_with(self);
    }

    fn visit_fn_expr(&mut self, fn_expr: &FnExpr, _: &dyn Node) {
      if let Some(ident) = &fn_expr.ident {
        self.insert_var(ident);
      }
      fn_expr.function.visit_with(fn_expr, self);
    }

    fn visit_function(&mut self, function: &Function, _: &dyn Node) {
      self.with_child_scope(function, |a| {
        for param in &function.params {
          param.visit_children_with(a);
          let idents: Vec<Ident> = find_ids(&param.pat);
          for ident in idents {
            a.insert_var(&ident);
          }
        }
        if let Some(body) = &function.body {
          body.visit_children_with(a);
        }
      });
    }

    fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
      self.with_child_scope(arrow_expr, |a| {
        for param in &arrow_expr.params {
          param.visit_children_with(a);
          let idents: Vec<Ident> = find_ids(param);
          for ident in idents {
            a.insert_var(&ident);
          }
        }
        match &arrow_expr.body {
          BlockStmtOrExpr::BlockStmt(block_stmt) => {
            block_stmt.visit_children_with(a);
          }
          BlockStmtOrExpr::Expr(expr) => {
            expr.visit_children_with(a);
          }
        }
      });
    }

    fn visit_block_stmt(&mut self, block_stmt: &BlockStmt, _: &dyn Node) {
      self.with_child_scope(block_stmt, |a| {
        block_stmt.visit_children_with(a);
      });
    }

    fn visit_for_stmt(&mut self, for_stmt: &ForStmt, _: &dyn Node) {
      self.with_child_scope(for_stmt, |a| {
        match &for_stmt.init {
          Some(VarDeclOrExpr::VarDecl(var_decl)) => {
            var_decl.visit_children_with(a);
            if var_decl.kind == VarDeclKind::Let {
              for decl in &var_decl.decls {
                a.extract_decl_idents(&decl.name);
              }
            }
          }
          Some(VarDeclOrExpr::Expr(expr)) => {
            expr.visit_children_with(a);
          }
          None => {}
        }

        if let Some(test_expr) = &for_stmt.test {
          test_expr.visit_children_with(a);
        }
        if let Some(update_expr) = &for_stmt.update {
          update_expr.visit_children_with(a);
        }

        if let Stmt::Block(block_stmt) = &*for_stmt.body {
          block_stmt.visit_children_with(a);
        } else {
          for_stmt.body.visit_children_with(a);
        }
      });
    }

    fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, _: &dyn Node) {
      self.with_child_scope(for_of_stmt, |a| {
        if let VarDeclOrPat::VarDecl(var_decl) = &for_of_stmt.left {
          for decl in &var_decl.decls {
            a.extract_decl_idents(&decl.name);
          }
        }

        for_of_stmt.right.visit_children_with(a);

        if let Stmt::Block(block_stmt) = &*for_of_stmt.body {
          block_stmt.visit_children_with(a);
        } else {
          for_of_stmt.body.visit_children_with(a);
        }
      });
    }

    fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, _: &dyn Node) {
      self.with_child_scope(for_in_stmt, |a| {
        if let VarDeclOrPat::VarDecl(var_decl) = &for_in_stmt.left {
          for decl in &var_decl.decls {
            a.extract_decl_idents(&decl.name);
          }
        }

        for_in_stmt.right.visit_children_with(a);

        if let Stmt::Block(block_stmt) = &*for_in_stmt.body {
          block_stmt.visit_children_with(a);
        } else {
          for_in_stmt.body.visit_children_with(a);
        }
      });
    }

    fn visit_if_stmt(&mut self, if_stmt: &IfStmt, _: &dyn Node) {
      self.with_child_scope(if_stmt, |a| {
        if_stmt.test.visit_children_with(a);
        // BlockStmt needs special handling to avoid creating a duplicate scope
        if let Stmt::Block(body) = &*if_stmt.cons {
          body.visit_children_with(a);
        } else {
          if_stmt.cons.visit_children_with(a);
        }
      });

      if let Some(alt) = &if_stmt.alt {
        self.with_child_scope(alt, |a| {
          alt.visit_children_with(a);
        });
      }
    }

    fn visit_while_stmt(&mut self, while_stmt: &WhileStmt, _: &dyn Node) {
      self.with_child_scope(while_stmt, |a| {
        while_stmt.test.visit_children_with(a);
        // BlockStmt needs special handling to avoid creating a duplicate scope
        if let Stmt::Block(body) = &*while_stmt.body {
          body.visit_children_with(a);
        } else {
          while_stmt.body.visit_children_with(a);
        }
      });
    }

    fn visit_do_while_stmt(
      &mut self,
      do_while_stmt: &DoWhileStmt,
      _: &dyn Node,
    ) {
      self.with_child_scope(do_while_stmt, |a| {
        // BlockStmt needs special handling to avoid creating a duplicate scope
        if let Stmt::Block(body) = &*do_while_stmt.body {
          body.visit_children_with(a);
        } else {
          do_while_stmt.body.visit_children_with(a);
        }
        do_while_stmt.test.visit_children_with(a);
      });
    }

    fn visit_with_stmt(&mut self, with_stmt: &WithStmt, _: &dyn Node) {
      self.with_child_scope(with_stmt, |a| {
        with_stmt.obj.visit_children_with(a);
        // BlockStmt needs special handling to avoid creating a duplicate scope
        if let Stmt::Block(body) = &*with_stmt.body {
          body.visit_children_with(a);
        } else {
          with_stmt.body.visit_children_with(a);
        }
      });
    }

    fn visit_catch_clause(&mut self, catch_clause: &CatchClause, _: &dyn Node) {
      self.with_child_scope(catch_clause, |a| {
        if let Some(param) = &catch_clause.param {
          let idents: Vec<Ident> = find_ids(param);
          for ident in idents {
            a.insert_var(&ident);
          }
        }
        catch_clause.body.visit_children_with(a);
      });
    }

    fn visit_class_decl(&mut self, class_decl: &ClassDecl, _: &dyn Node) {
      self.insert_var(&class_decl.ident);
      class_decl.visit_children_with(self);
    }

    fn visit_class_expr(&mut self, class_expr: &ClassExpr, _: &dyn Node) {
      if let Some(ident) = &class_expr.ident {
        self.insert_var(ident);
      }
      class_expr.visit_children_with(self);
    }

    fn visit_class(&mut self, class: &Class, _: &dyn Node) {
      for decorator in &class.decorators {
        decorator.visit_children_with(self);
      }
      if let Some(super_class) = &class.super_class {
        super_class.visit_children_with(self);
      }
      self.with_child_scope(class, |a| {
        for member in &class.body {
          member.visit_children_with(a);
        }
      });
    }

    fn visit_constructor(&mut self, constructor: &Constructor, _: &dyn Node) {
      self.with_child_scope(constructor, |a| {
        for param in &constructor.params {
          match param {
            ParamOrTsParamProp::TsParamProp(ts_param_prop) => {
              for decorator in &ts_param_prop.decorators {
                decorator.visit_children_with(a);
              }
              match &ts_param_prop.param {
                TsParamPropParam::Ident(ident) => {
                  a.insert_var(ident);
                }
                TsParamPropParam::Assign(assign_pat) => {
                  assign_pat.visit_children_with(a);
                  let idents: Vec<Ident> = find_ids(&assign_pat.left);
                  for ident in idents {
                    a.insert_var(&ident);
                  }
                }
              }
            }
            ParamOrTsParamProp::Param(param) => {
              param.visit_children_with(a);
              let idents: Vec<Ident> = find_ids(&param.pat);
              for ident in idents {
                a.insert_var(&ident);
              }
            }
          }
        }

        if let Some(body) = &constructor.body {
          body.visit_children_with(a);
        }
      });
    }

    fn visit_var_decl(&mut self, var_decl: &VarDecl, _: &dyn Node) {
      var_decl.visit_children_with(self);
      for decl in &var_decl.decls {
        self.extract_decl_idents(&decl.name);
      }
    }

    fn visit_import_named_specifier(
      &mut self,
      import_named_specifier: &ImportNamedSpecifier,
      _: &dyn Node,
    ) {
      self.insert_var(&import_named_specifier.local);
      import_named_specifier.visit_children_with(self);
    }

    fn visit_import_default_specifier(
      &mut self,
      import_default_specifier: &ImportDefaultSpecifier,
      _: &dyn Node,
    ) {
      self.insert_var(&import_default_specifier.local);
      import_default_specifier.visit_children_with(self);
    }

    fn visit_import_star_as_specifier(
      &mut self,
      import_star_as_specifier: &ImportStarAsSpecifier,
      _: &dyn Node,
    ) {
      self.insert_var(&import_star_as_specifier.local);
      import_star_as_specifier.visit_children_with(self);
    }

    fn visit_ts_enum_decl(&mut self, ts_enum_decl: &TsEnumDecl, _: &dyn Node) {
      self.insert_var(&ts_enum_decl.id);
      ts_enum_decl.visit_children_with(self);
    }

    fn visit_ts_param_prop_param(
      &mut self,
      ts_param_prop_param: &TsParamPropParam,
      _: &dyn Node,
    ) {
      match ts_param_prop_param {
        TsParamPropParam::Ident(ident) => {
          self.insert_var(ident);
        }
        TsParamPropParam::Assign(assign) => {
          self.extract_decl_idents(&Pat::Assign(assign.clone()));
        }
      }
      ts_param_prop_param.visit_children_with(self);
    }
  }

  #[cfg(test)]
  mod tests {
    use super::*;
    use crate::test_util::parse;
    use swc_ecmascript::ast::Ident;
    use swc_ecmascript::visit::{Node, Visit, VisitWith};

    fn decl_finder(src: &str) -> DeclFinder {
      let builder = DeclFinderBuilder::new();
      builder.build_from_module(&parse(src))
    }

    fn get_idents(src: &str, query: &'static str) -> Vec<Ident> {
      struct IdentGetter {
        query: &'static str,
        found_ident: Vec<Ident>,
      }

      impl Visit for IdentGetter {
        fn visit_ident(&mut self, ident: &Ident, _: &dyn Node) {
          if ident.sym.as_ref() == self.query {
            self.found_ident.push(ident.clone());
          } else {
            ident.visit_children_with(self);
          }
        }
      }

      let mut getter = IdentGetter {
        query,
        found_ident: Vec::new(),
      };
      let parsed = parse(src);
      getter.visit_module(&parsed, &parsed);
      getter.found_ident
    }

    #[test]
    fn decl_in_outer_scope() {
      let src = r#"
let target = 0;
function foo() {
  target = 1;
}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_assignment = &idents[1];
      assert!(finder.decl_exists(&target_assignment));
    }

    #[test]
    fn class_hoisting() {
      let src = r#"
function foo() {
  let a = new Target(); // hoisting
  class Target {}
}
        "#;
      let idents = get_idents(src, "Target");
      let finder = decl_finder(src);
      let target_new_call = &idents[0];
      assert!(finder.decl_exists(&target_new_call));
    }

    #[test]
    fn class_no_decl() {
      let src = r#"
function foo() {
  let a = new Target(); // no declaration
}
        "#;
      let idents = get_idents(src, "Target");
      let finder = decl_finder(src);
      let target_new_call = &idents[0];
      assert!(!finder.decl_exists(&target_new_call));
    }

    #[test]
    fn class_access_in_export_default_self() {
      let src = r#"
export default class Target {
  foo() {
    Target.doSomething();
  }
}
        "#;
      let idents = get_idents(src, "Target");
      let finder = decl_finder(src);
      let target_member_call = &idents[1];
      assert!(finder.decl_exists(&target_member_call));
    }

    #[test]
    fn arrow_function_hoisting() {
      let src = r#"
function foo() {
  target(); // hoisting
  const target = () => {};
}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_call = &idents[0];
      assert!(finder.decl_exists(&target_call));
    }

    #[test]
    fn decl_as_function_param() {
      let src = r#"
function foo(target) {
  target();
}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_call = &idents[1];
      assert!(finder.decl_exists(&target_call));
    }

    #[test]
    fn decl_in_for_of() {
      let src = r#"
for (const target of [1, 2, 3]) {
  console.log(target);
}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_used = &idents[1];
      assert!(finder.decl_exists(&target_used));
    }

    #[test]
    fn decl_in_for_in() {
      let src = r#"
for (const target in [1, 2, 3]) {
  console.log(target);
}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_used = &idents[1];
      assert!(finder.decl_exists(&target_used));
    }

    #[test]
    fn decl_as_caught_error() {
      let src = r#"
try {}
catch (target) {
  console.log(target);
}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_used = &idents[1];
      assert!(finder.decl_exists(&target_used));
    }

    #[test]
    fn function_decl_hoisting() {
      let src = r#"
target();
function target() {}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_call = &idents[0];
      assert!(finder.decl_exists(&target_call));
    }

    // TODO(magurotuna): Ideally this test should be passed.
    // #[test]
    // fn function_expr_name_discarded() {
    //   let src = r#"
    //   const f = function target() {};
    //   target();
    //     "#;
    //   let idents = get_idents(src, "target");
    //   let finder = decl_finder(src);
    //   let target_call = &idents[1];
    //   assert!(!finder.decl_exists(&target_call));
    // }

    #[test]
    fn function_call_in_export_default_self() {
      let src = r#"
export default function target() {
  target();
}
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_call = &idents[1];
      assert!(finder.decl_exists(&target_call));
    }

    #[test]
    fn decl_in_child_scope() {
      let src = r#"
function foo() {
  let target = 0;
}
target = 1;
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_assignment = &idents[1];
      assert!(!finder.decl_exists(&target_assignment));
    }

    #[test]
    fn decl_as_default_import() {
      let src = r#"
import target from "mod.ts";
target();
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_call = &idents[1];
      assert!(finder.decl_exists(&target_call));
    }

    #[test]
    fn decl_as_named_import() {
      let src = r#"
import { target } from "mod.ts";
target();
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_call = &idents[1];
      assert!(finder.decl_exists(&target_call));
    }

    #[test]
    fn decl_as_star_as_import() {
      let src = r#"
import * as target from "mod.ts";
target();
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_call = &idents[1];
      assert!(finder.decl_exists(&target_call));
    }

    #[test]
    fn decl_enum() {
      let src = r#"
enum Target {
  Foo,
  Bar,
}
const a = Target.Foo;
        "#;
      let idents = get_idents(src, "Target");
      let finder = decl_finder(src);
      let target_used = &idents[1];
      assert!(finder.decl_exists(&target_used));
    }

    #[test]
    fn decl_in_param_of_anonymous_function() {
      let src = r#"
[1,2,3].forEach(target => {
  console.log(target);
});
        "#;
      let idents = get_idents(src, "target");
      let finder = decl_finder(src);
      let target_used = &idents[1];
      assert!(finder.decl_exists(&target_used));
    }
  }
}

struct NoUndefVisitor<'c> {
  context: &'c mut Context,
  decl_finder: decl_finder::DeclFinder,
}

impl<'c> NoUndefVisitor<'c> {
  fn new(
    context: &'c mut Context,
    decl_finder: decl_finder::DeclFinder,
  ) -> Self {
    Self {
      context,
      decl_finder,
    }
  }

  fn check(&mut self, ident: &Ident) {
    if self.decl_finder.decl_exists(ident) {
      return;
    }

    // Implicitly defined
    // See: https://github.com/denoland/deno_lint/issues/317
    if ident.sym == *"arguments" {
      return;
    }

    // Globals
    if GLOBALS.iter().any(|(name, _)| name == &&*ident.sym) {
      return;
    }

    self.context.add_diagnostic(
      ident.span,
      CODE,
      format!("{} is not defined", ident.sym),
    )
  }

  /// `Pat` appears in two contexts:
  ///
  /// 1. variable declarations e.g.
  ///
  /// ```typescript
  /// const { foo } = obj;
  /// function ({ foo }) {}
  /// for (const { foo } of elements) {}
  /// for (const { foo } in elements) {}
  /// try {} catch (foo) {}
  /// ```
  ///
  /// 2. variable assignments e.g.
  ///
  /// ```typescript
  /// let foo;
  ///
  /// ({ foo } = obj);
  /// for ({ foo } of elements) {}
  /// for ({ foo } in elements) {}
  /// ```
  ///
  /// We have to differentiate these two contexts, which is why `is_decl` parameter exists.
  fn check_pat(&mut self, pat: &Pat, is_decl: bool) {
    match pat {
      Pat::Ident(ident) => {
        if !is_decl {
          self.check(ident);
        }
      }
      Pat::Array(array) => {
        for elem in &array.elems {
          if let Some(elem) = elem {
            self.check_pat(elem, is_decl);
          }
        }
      }
      Pat::Rest(rest) => {
        self.check_pat(&*rest.arg, is_decl);
      }
      Pat::Object(object) => {
        for prop in &object.props {
          match prop {
            ObjectPatProp::KeyValue(kv) => {
              self.check_pat(&*kv.value, is_decl);
            }
            ObjectPatProp::Assign(assign) => {
              if !is_decl {
                self.check(&assign.key);
              }
              if let Some(value) = &assign.value {
                self.visit_expr(&**value, assign);
              }
            }
            ObjectPatProp::Rest(rest) => {
              self.check_pat(&*rest.arg, is_decl);
            }
          }
        }
      }
      Pat::Assign(assign) => {
        self.check_pat(&*assign.left, is_decl);
        self.visit_expr(&*assign.right, assign);
      }
      Pat::Invalid(_) => {}
      Pat::Expr(expr) => {
        self.visit_expr(&**expr, pat);
      }
    }
  }
}

impl<'c> Visit for NoUndefVisitor<'c> {
  noop_visit_type!();

  fn visit_member_expr(&mut self, member_expr: &MemberExpr, _: &dyn Node) {
    member_expr.obj.visit_with(member_expr, self);
    if member_expr.computed {
      member_expr.prop.visit_with(member_expr, self);
    }
  }

  fn visit_unary_expr(&mut self, unary_expr: &UnaryExpr, _: &dyn Node) {
    if unary_expr.op == UnaryOp::TypeOf {
      return;
    }

    unary_expr.visit_children_with(self);
  }

  fn visit_expr(&mut self, expr: &Expr, _: &dyn Node) {
    expr.visit_children_with(self);

    if let Expr::Ident(ident) = expr {
      self.check(ident)
    }
  }

  fn visit_assign_expr(&mut self, assign_expr: &AssignExpr, _: &dyn Node) {
    match &assign_expr.left {
      PatOrExpr::Expr(expr) => {
        expr.visit_with(assign_expr, self);
      }
      PatOrExpr::Pat(pat) => {
        self.check_pat(&*pat, false);
      }
    }
    assign_expr.right.visit_with(assign_expr, self);
  }

  fn visit_var_decl(&mut self, var_decl: &VarDecl, _: &dyn Node) {
    for decl in &var_decl.decls {
      self.check_pat(&decl.name, true);
      if let Some(init) = &decl.init {
        init.visit_with(var_decl, self);
      }
    }
  }

  fn visit_param(&mut self, param: &Param, _: &dyn Node) {
    param.decorators.visit_with(param, self);
    self.check_pat(&param.pat, true);
  }

  fn visit_arrow_expr(&mut self, arrow_expr: &ArrowExpr, _: &dyn Node) {
    for param in &arrow_expr.params {
      self.check_pat(param, true);
    }
    arrow_expr.body.visit_with(arrow_expr, self);
  }

  fn visit_catch_clause(&mut self, catch_clause: &CatchClause, _: &dyn Node) {
    if let Some(param) = &catch_clause.param {
      self.check_pat(param, true);
    }
    catch_clause.body.visit_with(catch_clause, self);
  }

  fn visit_for_of_stmt(&mut self, for_of_stmt: &ForOfStmt, _: &dyn Node) {
    match &for_of_stmt.left {
      VarDeclOrPat::VarDecl(var_decl) => {
        var_decl.visit_with(for_of_stmt, self);
      }
      VarDeclOrPat::Pat(pat) => {
        self.check_pat(pat, false);
      }
    }
    for_of_stmt.right.visit_with(for_of_stmt, self);
    for_of_stmt.body.visit_with(for_of_stmt, self);
  }

  fn visit_for_in_stmt(&mut self, for_in_stmt: &ForInStmt, _: &dyn Node) {
    match &for_in_stmt.left {
      VarDeclOrPat::VarDecl(var_decl) => {
        var_decl.visit_with(for_in_stmt, self);
      }
      VarDeclOrPat::Pat(pat) => {
        self.check_pat(pat, false);
      }
    }
    for_in_stmt.right.visit_with(for_in_stmt, self);
    for_in_stmt.body.visit_with(for_in_stmt, self);
  }

  fn visit_class_prop(&mut self, class_prop: &ClassProp, _: &dyn Node) {
    // don't check the key of class_prop
    class_prop.value.visit_with(class_prop, self)
  }

  fn visit_prop(&mut self, prop: &Prop, _: &dyn Node) {
    prop.visit_children_with(self);

    if let Prop::Shorthand(i) = &prop {
      self.check(i);
    }
  }

  fn visit_call_expr(&mut self, call_expr: &CallExpr, _: &dyn Node) {
    if let ExprOrSuper::Expr(callee) = &call_expr.callee {
      if let Expr::Ident(i) = &**callee {
        if i.sym == js_word!("import") {
          return;
        }
      }
    }

    call_expr.visit_children_with(self)
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn magurotuna() {
    assert_lint_ok! {
      NoUndef,
      "[1,2,3].forEach(obj => {\n  obj++; \n});",
      r#"
        for (const [key, value] of [1,2,3]) {
          console.log(`${key}: ${value}\r\n`);
        }
        "#,
    };
  }

  #[test]
  fn no_undef_valid() {
    assert_lint_ok! {
      NoUndef,
      "var a = 1, b = 2; a;",
      "function a(){}  a();",
      "function f(b) { b; }",
      "var a; a = 1; a++;",
      "var a; function f() { a = 1; }",
      "Object; isNaN();",
      "toString()",
      "hasOwnProperty()",
      "function evilEval(stuffToEval) { var ultimateAnswer; ultimateAnswer = 42; eval(stuffToEval); }",
      "typeof a",
      "typeof (a)",
      "var b = typeof a",
      "typeof a === 'undefined'",
      "if (typeof a === 'undefined') {}",
      "function foo() { var [a, b=4] = [1, 2]; return {a, b}; }",
      "var toString = 1;",
      "function myFunc(...foo) {  return foo; }",

      // https://github.com/denoland/deno_lint/issues/317
      "function myFunc() { console.log(arguments); }",

      // TODO(kdy1): Parse as jsx
      // "var React, App, a=1; React.render(<App attr={a} />);",

      "var console; [1,2,3].forEach(obj => {\n  console.log(obj);\n});",
      "var Foo; class Bar extends Foo { constructor() { super();  }}",
      "import Warning from '../lib/warning'; var warn = new Warning('text');",
      "import * as Warning from '../lib/warning'; var warn = new Warning('text');",
      "var a; [a] = [0];",
      "var a; ({a} = {});",
      "var a; ({b: a} = {});",
      "var obj; [obj.a, obj.b] = [0, 1];",
      "(foo, bar) => { foo ||= WeakRef; bar ??= FinalizationRegistry; }",
      "Array = 1;",
      "class A { constructor() { new.target; } }",
      r#"export * as ns from "source""#,
      "import.meta",
      "
      await new Promise((resolve: () => void, _) => {
        setTimeout(resolve, 100);
      });
      ",
      "
      const importPath = \"./foo.ts\";
      const dataProcessor = await import(importPath);
      ",
      r#"
    class PartWriter implements Deno.Writer {
      closed = false;
      private readonly partHeader: string;
      private headersWritten = false;

      constructor(
        private writer: Deno.Writer,
        readonly boundary: string,
        public headers: Headers,
        isFirstBoundary: boolean,
      ) {
        let buf = "";
        if (isFirstBoundary) {
          buf += `--${boundary}\r\n`;
        } else {
          buf += `\r\n--${boundary}\r\n`;
        }
        for (const [key, value] of headers.entries()) {
          buf += `${key}: ${value}\r\n`;
        }
        buf += `\r\n`;
        this.partHeader = buf;
      }

      close(): void {
        this.closed = true;
      }

      async write(p: Uint8Array): Promise<number> {
        if (this.closed) {
          throw new Error("part is closed");
        }
        if (!this.headersWritten) {
          this.headersWritten = true;
        }
        return this.writer.write(p);
      }
    }
    "#,
      r#"
    const listeners = [];
    for (const listener of listeners) {
      try {
      } catch (err) {
        this.emit("error", err);
      }
    }
    "#,

      // https://github.com/denoland/deno_lint/issues/463
      r#"
(() => {
  function foo() {
    return new Bar();
  }
  class Bar {}
})();
      "#,
      r#"
const f = () => {
  function foo() {
    return new Bar();
  }
  class Bar {}
};
      "#,
    };
  }

  #[test]
  fn no_undef_invalid() {
    assert_lint_err! {
      NoUndef,
      "a = 1;": [
        {
          col: 0,
          message: "a is not defined",
        },
      ],
      "var a = b;": [
        {
          col: 8,
          message: "b is not defined",
        },
      ],
      "function f() { b; }": [
        {
          col: 15,
          message: "b is not defined",
        },
      ],
      // "var React; React.render(<img attr={a} />);": [
      //   {
      //     col: 0,
      //     message: "a is not defined",
      //    },
      // ],
      // "var React, App; React.render(<App attr={a} />);": [
      //   {
      //     col: 0,
      //     message: "a is not defined",
      //   },
      // ],
      "[a] = [0];": [
        {
          col: 1,
          message: "a is not defined",
        },
      ],
      "({a} = {});": [
        {
          col: 2,
          message: "a is not defined",
        },
      ],
      "({b: a} = {});": [
        {
          col: 5,
          message: "a is not defined",
        },
      ],
      "[obj.a, obj.b] = [0, 1];": [
        {
          col: 1,
          message: "obj is not defined",
        },
        {
          col: 8,
          message: "obj is not defined",
        },
      ],
      "const c = 0; const a = {...b, c};": [
        {
          col: 27,
          message: "b is not defined",
        },
      ],
    };
  }
}
