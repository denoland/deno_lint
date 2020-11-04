use super::LintRule;
use crate::{globals::GLOBALS, linter::Context, swc_util::find_lhs_ids};
use std::collections::HashSet;
use swc_common::Span;
use swc_ecmascript::{
  ast::*,
  utils::find_ids,
  utils::ident::IdentLike,
  utils::Id,
  visit::Node,
  visit::{noop_visit_type, Visit, VisitWith},
};

pub struct NoGlobalAssign;

impl LintRule for NoGlobalAssign {
  fn new() -> Box<Self> {
    Box::new(NoGlobalAssign)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    "no-global-assign"
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut collector = Collector {
      bindings: Default::default(),
    };
    program.visit_with(program, &mut collector);

    let mut visitor = NoGlobalAssignVisitor::new(context, collector.bindings);
    program.visit_with(program, &mut visitor);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows assignment to native Javascript objects

In Javascript, `String` and `Object` for example are native objects.  Like any
object, they can be reassigned, but it is almost never wise to do so as this
can lead to unexpected results and difficult to track down bugs.
    
### Invalid:
```typescript
Object = null;
undefined = true;
window = {};
```
"#
  }
}

struct Collector {
  bindings: HashSet<Id>,
}

impl Visit for Collector {
  noop_visit_type!();

  fn visit_import_named_specifier(
    &mut self,
    i: &ImportNamedSpecifier,
    _: &dyn Node,
  ) {
    self.bindings.insert(i.local.to_id());
  }

  fn visit_import_default_specifier(
    &mut self,
    i: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    self.bindings.insert(i.local.to_id());
  }

  fn visit_import_star_as_specifier(
    &mut self,
    i: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    self.bindings.insert(i.local.to_id());
  }

  // Other top level bindings

  fn visit_fn_decl(&mut self, n: &FnDecl, _: &dyn Node) {
    self.bindings.insert(n.ident.to_id());
  }

  fn visit_class_decl(&mut self, n: &ClassDecl, _: &dyn Node) {
    self.bindings.insert(n.ident.to_id());
  }

  fn visit_var_declarator(&mut self, n: &VarDeclarator, _: &dyn Node) {
    let ids: Vec<Id> = find_ids(&n.name);

    for id in ids {
      self.bindings.insert(id);
    }
  }

  /// No-op, as only top level bindings are relevant to this lint.
  fn visit_expr(&mut self, _: &Expr, _: &dyn Node) {}
}

struct NoGlobalAssignVisitor<'c> {
  context: &'c mut Context,
  /// This hashset only contains top level bindings, so using HashSet<JsWord>
  /// also can be an option.
  bindings: HashSet<Id>,
}

impl<'c> NoGlobalAssignVisitor<'c> {
  fn new(context: &'c mut Context, bindings: HashSet<Id>) -> Self {
    Self { context, bindings }
  }

  fn check(&mut self, span: Span, id: Id) {
    if id.1 != self.context.top_level_ctxt {
      return;
    }

    // Global is shadowed by top level binding
    if self.bindings.contains(&id) {
      return;
    }

    // We only care about globals.
    let can_be_overwritten = GLOBALS.get(&*id.0);

    // If global can be overwritten then don't need to report anything
    if can_be_overwritten == Some(&false) {
      self.context.add_diagnostic_with_hint(
        span,
        "no-global-assign",
        "Assignment to global is not allowed",
        "Remove the assignment to the global variable",
      );
    }
  }
}

impl<'c> Visit for NoGlobalAssignVisitor<'c> {
  noop_visit_type!();

  fn visit_assign_expr(&mut self, e: &AssignExpr, _: &dyn Node) {
    let idents: Vec<Ident> = find_lhs_ids(&e.left);

    for ident in idents {
      self.check(ident.span, ident.to_id());
    }
  }

  fn visit_update_expr(&mut self, e: &UpdateExpr, _: &dyn Node) {
    if let Expr::Ident(i) = &*e.arg {
      self.check(e.span, i.to_id());
    } else {
      e.visit_children_with(self);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_global_assign_valid() {
    assert_lint_ok! {
      NoGlobalAssign,
      "string = 'hello world';",
      "var string;",
      "top = 0;",
      "require = 0;",
      "onmessage = function () {};",
    };
  }

  #[test]
  fn no_global_assign_invalid() {
    assert_lint_err::<NoGlobalAssign>("String = 'hello world';", 0);

    assert_lint_err::<NoGlobalAssign>("String++;", 0);

    assert_lint_err_n::<NoGlobalAssign>(
      "({Object = 0, String = 0} = {});",
      vec![2, 14],
    );
    assert_lint_err::<NoGlobalAssign>("Array = 1;", 0);
  }
}
