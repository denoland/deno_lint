use super::LintRule;
use crate::linter::Context;
use std::collections::HashSet;
use swc_atoms::js_word;
use swc_common::Span;
use swc_common::Spanned;
use swc_ecmascript::{
  ast::*,
  utils::find_ids,
  utils::ident::IdentLike,
  utils::Id,
  visit::Node,
  visit::{noop_visit_type, Visit, VisitWith},
};

pub struct NoImportAssign;

impl LintRule for NoImportAssign {
  fn new() -> Box<Self> {
    Box::new(NoImportAssign)
  }

  fn code(&self) -> &'static str {
    "no-import-assign"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut collector = Collector {
      imports: Default::default(),
      ns_imports: Default::default(),
      other_bindings: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoImportAssignVisitor::new(
      context,
      collector.imports,
      collector.ns_imports,
      collector.other_bindings,
    );
    module.visit_with(module, &mut visitor);
  }
}

struct Collector {
  imports: HashSet<Id>,
  ns_imports: HashSet<Id>,
  other_bindings: HashSet<Id>,
}

impl Visit for Collector {
  noop_visit_type!();

  fn visit_import_named_specifier(
    &mut self,
    i: &ImportNamedSpecifier,
    _: &dyn Node,
  ) {
    self.imports.insert(i.local.to_id());
  }

  fn visit_import_default_specifier(
    &mut self,
    i: &ImportDefaultSpecifier,
    _: &dyn Node,
  ) {
    self.imports.insert(i.local.to_id());
  }

  fn visit_import_star_as_specifier(
    &mut self,
    i: &ImportStarAsSpecifier,
    _: &dyn Node,
  ) {
    self.ns_imports.insert(i.local.to_id());
  }

  // Other top level bindings

  fn visit_fn_decl(&mut self, n: &FnDecl, _: &dyn Node) {
    self.other_bindings.insert(n.ident.to_id());
  }

  fn visit_class_decl(&mut self, n: &ClassDecl, _: &dyn Node) {
    self.other_bindings.insert(n.ident.to_id());
  }

  fn visit_var_declarator(&mut self, n: &VarDeclarator, _: &dyn Node) {
    let ids: Vec<Id> = find_ids(&n.name);

    for id in ids {
      self.other_bindings.insert(id);
    }
  }

  fn visit_expr(&mut self, _: &Expr, _: &dyn Node) {}
}

struct NoImportAssignVisitor<'c> {
  context: &'c mut Context,
  /// This hashset only contains top level bindings, so using HashSet<JsWord>
  /// also can be an option.
  imports: HashSet<Id>,
  ns_imports: HashSet<Id>,
  /// Top level bindings other than import.
  other_bindings: HashSet<Id>,
}

impl<'c> NoImportAssignVisitor<'c> {
  fn new(
    context: &'c mut Context,
    imports: HashSet<Id>,
    ns_imports: HashSet<Id>,
    other_bindings: HashSet<Id>,
  ) -> Self {
    Self {
      context,
      imports,
      ns_imports,
      other_bindings,
    }
  }

  fn check(&self, span: Span, i: &Ident, is_assign_to_prop: bool) {
    // All imports are top-level and as a result,
    // if an identifier is not top-level, we are not assigning to import
    if i.span.ctxt != self.context.top_level_ctxt {
      return;
    }

    // We only care about imports
    if self.other_bindings.contains(&i.to_id()) {
      return;
    }

    if self.ns_imports.contains(&i.to_id()) {
      self.context.add_diagnostic(
        span,
        "no-import-assign",
        "Assignment to import is not allowed",
      );
      return;
    }

    if !is_assign_to_prop && self.imports.contains(&i.to_id()) {
      self.context.add_diagnostic(
        span,
        "no-import-assign",
        "Assignment to import is not allowed",
      );
    }
  }

  fn check_assign(&mut self, span: Span, lhs: &Expr, is_assign_to_prop: bool) {
    if let Expr::Ident(lhs) = &lhs {
      self.check(span, lhs, is_assign_to_prop);
    }
  }

  fn check_expr(&mut self, span: Span, e: &Expr) {
    match e {
      Expr::Ident(i) => {
        self.check(span, i, false);
      }
      Expr::Member(e) => {
        if let ExprOrSuper::Expr(obj) = &e.obj {
          self.check_assign(span, &obj, true)
        }
      }
      Expr::OptChain(e) => self.check_expr(span, &e.expr),
      Expr::Paren(e) => self.check_expr(span, &e.expr),
      _ => e.visit_children_with(self),
    }
  }

  fn is_modifier(&self, obj: &Expr, prop: &Expr) -> bool {
    if let Expr::Ident(obj) = obj {
      if self.context.top_level_ctxt != obj.span.ctxt {
        return false;
      }
      if self.other_bindings.contains(&obj.to_id()) {
        return false;
      }
    }

    match &*obj {
      Expr::Ident(Ident {
        sym: js_word!("Object"),
        ..
      }) => {
        // Check for Object.defineProperty and Object.assign

        match prop {
          Expr::Ident(Ident { sym, .. })
            if *sym == *"defineProperty"
              || *sym == *"assign"
              || *sym == *"setPrototypeOf"
              || *sym == *"freeze" =>
          {
            // It's now property assignment.
            return true;
          }
          _ => {}
        }
      }

      Expr::Ident(Ident {
        sym: js_word!("Reflect"),
        ..
      }) => {
        match prop {
          Expr::Ident(Ident { sym, .. })
            if *sym == *"defineProperty"
              || *sym == *"deleteProperty"
              || *sym == *"set"
              || *sym == *"setPrototypeOf" =>
          {
            // It's now property assignment.
            return true;
          }
          _ => {}
        }
      }
      _ => {}
    }

    false
  }

  /// Returns true for callees like `Object.assign`
  fn modifies_first(&self, callee: &Expr) -> bool {
    match callee {
      Expr::Member(
        callee @ MemberExpr {
          computed: false, ..
        },
      ) => {
        if let ExprOrSuper::Expr(obj) = &callee.obj {
          if self.is_modifier(obj, &callee.prop) {
            return true;
          }
        }
      }

      Expr::Paren(ParenExpr { expr, .. })
      | Expr::OptChain(OptChainExpr { expr, .. }) => {
        return self.modifies_first(&expr)
      }

      _ => {}
    }

    false
  }
}

impl<'c> Visit for NoImportAssignVisitor<'c> {
  noop_visit_type!();

  fn visit_pat(&mut self, n: &Pat, _: &dyn Node) {
    match n {
      Pat::Ident(i) => {
        self.check(i.span, &i, false);
      }
      Pat::Expr(e) => {
        self.check_expr(n.span(), e);
      }
      _ => {
        n.visit_children_with(self);
      }
    }
  }

  fn visit_rest_pat(&mut self, n: &RestPat, _: &dyn Node) {
    if let Pat::Expr(e) = &*n.arg {
      match &**e {
        Expr::Ident(i) => {
          self.check(i.span, i, true);
        }
        _ => {
          self.check_expr(e.span(), e);
        }
      }
    } else {
      n.visit_children_with(self)
    }
  }

  fn visit_assign_expr(&mut self, n: &AssignExpr, _: &dyn Node) {
    match &n.left {
      PatOrExpr::Expr(e) => {
        self.check_expr(n.span, e);
      }
      PatOrExpr::Pat(p) => {
        p.visit_with(n, self);
      }
    };
    n.right.visit_with(n, self);
  }

  fn visit_assign_pat_prop(&mut self, n: &AssignPatProp, _: &dyn Node) {
    self.check(n.key.span, &n.key, false);

    n.value.visit_children_with(self);
  }

  fn visit_update_expr(&mut self, n: &UpdateExpr, _: &dyn Node) {
    self.check_expr(n.span, &n.arg);
  }

  fn visit_unary_expr(&mut self, n: &UnaryExpr, _: &dyn Node) {
    if let UnaryOp::Delete = n.op {
      self.check_expr(n.span, &n.arg);
    } else {
      n.arg.visit_with(n, self);
    }
  }

  fn visit_call_expr(&mut self, n: &CallExpr, _: &dyn Node) {
    n.visit_children_with(self);

    if let ExprOrSuper::Expr(callee) = &n.callee {
      if let Some(arg) = n.args.first() {
        if self.modifies_first(&callee) {
          self.check_assign(n.span, &arg.expr, true);
        }
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn ok_1() {
    assert_lint_ok::<NoImportAssign>("import mod from 'mod'; mod.prop = 0");

    assert_lint_ok::<NoImportAssign>("import mod from 'mod'; mod.prop += 0;");

    assert_lint_ok::<NoImportAssign>("import mod from 'mod'; mod.prop++");
  }

  #[test]
  fn ok_2() {
    assert_lint_ok::<NoImportAssign>("import mod from 'mod'; delete mod.prop");

    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; for (mod.prop in foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; for (mod.prop of foo);",
    );
  }

  #[test]
  fn ok_3() {
    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; [mod.prop] = foo;",
    );

    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; [...mod.prop] = foo;",
    );

    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; ({ bar: mod.prop } = foo);",
    );
  }

  #[test]
  fn ok_4() {
    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; ({ ...mod.prop } = foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; named.prop = 0",
    );

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; named.prop += 0",
    );
  }
  #[test]
  fn ok_5() {
    assert_lint_ok::<NoImportAssign>("import {named} from 'mod'; named.prop++");

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; delete named.prop",
    );

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; for (named.prop in foo);",
    );
  }

  #[test]
  fn ok_6() {
    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; for (named.prop of foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; [named.prop] = foo;",
    );

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; [...named.prop] = foo;",
    );
  }

  #[test]
  fn ok_7() {
    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; ({ bar: named.prop } = foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; ({ ...named.prop } = foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; mod.named.prop = 0",
    );
  }

  #[test]
  fn ok_8() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; mod.named.prop += 0",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; mod.named.prop++",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; delete mod.named.prop",
    );
  }

  #[test]
  fn ok_9() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; for (mod.named.prop in foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; for (mod.named.prop of foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; [mod.named.prop] = foo;",
    );
  }

  #[test]
  fn ok_10() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; [...mod.named.prop] = foo;",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; ({ bar: mod.named.prop } = foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; ({ ...mod.named.prop } = foo);",
    );
  }

  #[test]
  fn ok_11() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; obj[mod] = 0",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; obj[mod.named] = 0",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; for (var foo in mod.named);",
    );
  }

  #[test]
  fn ok_12() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; for (var foo of mod.named);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; [bar = mod.named] = foo;",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; ({ bar = mod.named } = foo);",
    );
  }

  #[test]
  fn ok_13() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; ({ bar: baz = mod.named } = foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; ({ [mod.named]: bar } = foo);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; var obj = { ...mod.named };",
    );
  }

  #[test]
  fn ok_14() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; var obj = { foo: mod.named };",
    );

    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; { let mod = 0; mod = 1 }",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; { let mod = 0; mod = 1 }",
    );
  }

  #[test]
  fn ok_15() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; { let mod = 0; mod.named = 1 }",
    );

    assert_lint_ok::<NoImportAssign>("import {} from 'mod'");

    assert_lint_ok::<NoImportAssign>("import 'mod'");
  }

  #[test]
  fn ok_16() {
    assert_lint_ok::<NoImportAssign>(
      "import mod from 'mod'; Object.assign(mod, obj);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import {named} from 'mod'; Object.assign(named, obj);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Object.assign(mod.prop, obj);",
    );
  }

  #[test]
  fn ok_17() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Object.assign(obj, mod, other);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Object[assign](mod, obj);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Object.getPrototypeOf(mod);",
    );
  }

  #[test]
  fn ok_18() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.set(obj, key, mod);",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; { var Object; Object.assign(mod, obj); }",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; var Object; Object.assign(mod, obj);",
    );
  }

  #[test]
  fn ok_19() {
    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Object.seal(mod, obj)",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Object.preventExtensions(mod)",
    );

    assert_lint_ok::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.preventExtensions(mod)",
    );
  }

  #[test]
  fn err_1() {
    assert_lint_err::<NoImportAssign>("import mod1 from 'mod'; mod1 = 0", 24);

    assert_lint_err::<NoImportAssign>("import mod2 from 'mod'; mod2 += 0", 24);

    assert_lint_err::<NoImportAssign>("import mod3 from 'mod'; mod3++", 24);
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoImportAssign>(
      "import mod4 from 'mod'; for (mod4 in foo);",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod5 from 'mod'; for (mod5 of foo);",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod6 from 'mod'; [mod6] = foo",
      25,
    );
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoImportAssign>(
      "import mod7 from 'mod'; [mod7 = 0] = foo",
      25,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod8 from 'mod'; [...mod8] = foo",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod9 from 'mod'; ({ bar: mod9 } = foo)",
      32,
    );
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoImportAssign>(
      "import mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)",
      33,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod11 from 'mod'; ({ ...mod11 } = foo)",
      31,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named1} from 'mod'; named1 = 0",
      28,
    );
  }

  #[test]
  fn err_5() {
    assert_lint_err::<NoImportAssign>(
      "import {named2} from 'mod'; named2 += 0",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named3} from 'mod'; named3++",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named4} from 'mod'; for (named4 in foo);",
      33,
    );
  }

  #[test]
  fn err_6() {
    assert_lint_err::<NoImportAssign>(
      "import {named5} from 'mod'; for (named5 of foo);",
      33,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named6} from 'mod'; [named6] = foo",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named7} from 'mod'; [named7 = 0] = foo",
      29,
    );
  }

  #[test]
  fn err_7() {
    assert_lint_err::<NoImportAssign>(
      "import {named8} from 'mod'; [...named8] = foo",
      32,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named9} from 'mod'; ({ bar: named9 } = foo)",
      36,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named10} from 'mod'; ({ bar: named10 = 0 } = foo)",
      37,
    );
  }

  #[test]
  fn err_8() {
    assert_lint_err::<NoImportAssign>(
      "import {named11} from 'mod'; ({ ...named11 } = foo)",
      35,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named12 as foo} from 'mod'; foo = 0; named12 = 0",
      36,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod1 from 'mod'; mod1 = 0",
      29,
    );
  }

  #[test]
  fn err_9() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod2 from 'mod'; mod2 += 0",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod3 from 'mod'; mod3++",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod4 from 'mod'; for (mod4 in foo);",
      34,
    );
  }

  #[test]
  fn err_10() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod5 from 'mod'; for (mod5 of foo);",
      34,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod6 from 'mod'; [mod6] = foo",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod7 from 'mod'; [mod7 = 0] = foo",
      30,
    );
  }

  #[test]
  fn err_11() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod8 from 'mod'; [...mod8] = foo",
      33,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod9 from 'mod'; ({ bar: mod9 } = foo)",
      37,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)",
      38,
    );
  }

  #[test]
  fn err_12() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod11 from 'mod'; ({ ...mod11 } = foo)",
      36,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod1 from 'mod'; mod1.named = 0",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod2 from 'mod'; mod2.named += 0",
      29,
    );
  }

  #[test]
  fn err_13() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod3 from 'mod'; mod3.named++",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod4 from 'mod'; for (mod4.named in foo);",
      34,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod5 from 'mod'; for (mod5.named of foo);",
      34,
    );
  }

  #[test]
  fn err_14() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod6 from 'mod'; [mod6.named] = foo",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod7 from 'mod'; [mod7.named = 0] = foo",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod8 from 'mod'; [...mod8.named] = foo",
      33,
    );
  }

  #[test]
  fn err_15() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod9 from 'mod'; ({ bar: mod9.named } = foo)",
      37,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod10 from 'mod'; ({ bar: mod10.named = 0 } = foo)",
      38,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod11 from 'mod'; ({ ...mod11.named } = foo)",
      36,
    );
  }

  #[test]
  fn err_16() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod12 from 'mod'; delete mod12.named",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.assign(mod, obj)",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.defineProperty(mod, key, d)",
      28,
    );
  }

  #[test]
  fn err_17() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.setPrototypeOf(mod, proto)",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.freeze(mod)",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.defineProperty(mod, key, d)",
      28,
    );
  }

  #[test]
  fn err_18() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.deleteProperty(mod, key)",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.set(mod, key, value)",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.setPrototypeOf(mod, proto)",
      28,
    );
  }

  #[test]
  fn err_19() {
    assert_lint_err::<NoImportAssign>(
      "import mod, * as mod_ns from 'mod'; mod.prop = 0; mod_ns.prop = 0",
      50,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object?.defineProperty(mod, key, d)",
      28,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; (Object?.defineProperty)(mod, key, d)",
      28,
    );
  }

  #[test]
  fn err_20() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; delete mod?.prop",
      28,
    );
  }
}
