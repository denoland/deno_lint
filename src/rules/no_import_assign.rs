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

const CODE: &str = "no-import-assign";
const MESSAGE: &str = "Assignment to import is not allowed";

impl LintRule for NoImportAssign {
  fn new() -> Box<Self> {
    Box::new(NoImportAssign)
  }

  fn tags(&self) -> &[&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
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

  fn check(&mut self, span: Span, i: &Ident, is_assign_to_prop: bool) {
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
      self.context.add_diagnostic(span, CODE, MESSAGE);
      return;
    }

    if !is_assign_to_prop && self.imports.contains(&i.to_id()) {
      self.context.add_diagnostic(span, CODE, MESSAGE);
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

  #[test]
  fn no_import_assign_valid() {
    assert_lint_ok! {
      NoImportAssign,
      "import mod from 'mod'; mod.prop = 0",
      "import mod from 'mod'; mod.prop += 0;",
      "import mod from 'mod'; mod.prop++",
      "import mod from 'mod'; delete mod.prop",
      "import mod from 'mod'; for (mod.prop in foo);",
      "import mod from 'mod'; for (mod.prop of foo);",
      "import mod from 'mod'; [mod.prop] = foo;",
      "import mod from 'mod'; [...mod.prop] = foo;",
      "import mod from 'mod'; ({ bar: mod.prop } = foo);",
      "import mod from 'mod'; ({ ...mod.prop } = foo);",
      "import {named} from 'mod'; named.prop = 0",
      "import {named} from 'mod'; named.prop += 0",
      "import {named} from 'mod'; named.prop++",
      "import {named} from 'mod'; delete named.prop",
      "import {named} from 'mod'; for (named.prop in foo);",
      "import {named} from 'mod'; for (named.prop of foo);",
      "import {named} from 'mod'; [named.prop] = foo;",
      "import {named} from 'mod'; [...named.prop] = foo;",
      "import {named} from 'mod'; ({ bar: named.prop } = foo);",
      "import {named} from 'mod'; ({ ...named.prop } = foo);",
      "import * as mod from 'mod'; mod.named.prop = 0",
      "import * as mod from 'mod'; mod.named.prop += 0",
      "import * as mod from 'mod'; mod.named.prop++",
      "import * as mod from 'mod'; delete mod.named.prop",
      "import * as mod from 'mod'; for (mod.named.prop in foo);",
      "import * as mod from 'mod'; for (mod.named.prop of foo);",
      "import * as mod from 'mod'; [mod.named.prop] = foo;",
      "import * as mod from 'mod'; [...mod.named.prop] = foo;",
      "import * as mod from 'mod'; ({ bar: mod.named.prop } = foo);",
      "import * as mod from 'mod'; ({ ...mod.named.prop } = foo);",
      "import * as mod from 'mod'; obj[mod] = 0",
      "import * as mod from 'mod'; obj[mod.named] = 0",
      "import * as mod from 'mod'; for (var foo in mod.named);",
      "import * as mod from 'mod'; for (var foo of mod.named);",
      "import * as mod from 'mod'; [bar = mod.named] = foo;",
      "import * as mod from 'mod'; ({ bar = mod.named } = foo);",
      "import * as mod from 'mod'; ({ bar: baz = mod.named } = foo);",
      "import * as mod from 'mod'; ({ [mod.named]: bar } = foo);",
      "import * as mod from 'mod'; var obj = { ...mod.named };",
      "import * as mod from 'mod'; var obj = { foo: mod.named };",
      "import mod from 'mod'; { let mod = 0; mod = 1 }",
      "import * as mod from 'mod'; { let mod = 0; mod = 1 }",
      "import * as mod from 'mod'; { let mod = 0; mod.named = 1 }",
      "import {} from 'mod'",
      "import 'mod'",
      "import mod from 'mod'; Object.assign(mod, obj);",
      "import {named} from 'mod'; Object.assign(named, obj);",
      "import * as mod from 'mod'; Object.assign(mod.prop, obj);",
      "import * as mod from 'mod'; Object.assign(obj, mod, other);",
      "import * as mod from 'mod'; Object[assign](mod, obj);",
      "import * as mod from 'mod'; Object.getPrototypeOf(mod);",
      "import * as mod from 'mod'; Reflect.set(obj, key, mod);",
      "import * as mod from 'mod'; { var Object; Object.assign(mod, obj); }",
      "import * as mod from 'mod'; var Object; Object.assign(mod, obj);",
      "import * as mod from 'mod'; Object.seal(mod, obj)",
      "import * as mod from 'mod'; Object.preventExtensions(mod)",
      "import * as mod from 'mod'; Reflect.preventExtensions(mod)",
    };
  }

  #[test]
  fn no_import_assign_invalid() {
    assert_lint_err! {
      NoImportAssign,
      "import mod1 from 'mod'; mod1 = 0": [{ col: 24, message: MESSAGE }],
      "import mod2 from 'mod'; mod2 += 0": [{ col: 24, message: MESSAGE }],
      "import mod3 from 'mod'; mod3++": [{ col: 24, message: MESSAGE }],
      "import mod4 from 'mod'; for (mod4 in foo);": [{ col: 29, message: MESSAGE }],
      "import mod5 from 'mod'; for (mod5 of foo);": [{ col: 29, message: MESSAGE }],
      "import mod6 from 'mod'; [mod6] = foo": [{ col: 25, message: MESSAGE }],
      "import mod7 from 'mod'; [mod7 = 0] = foo": [{ col: 25, message: MESSAGE }],
      "import mod8 from 'mod'; [...mod8] = foo": [{ col: 28, message: MESSAGE }],
      "import mod9 from 'mod'; ({ bar: mod9 } = foo)": [{ col: 32, message: MESSAGE }],
      "import mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)": [{ col: 33, message: MESSAGE }],
      "import mod11 from 'mod'; ({ ...mod11 } = foo)": [{ col: 31, message: MESSAGE }],
      "import {named1} from 'mod'; named1 = 0": [{ col: 28, message: MESSAGE }],
      "import {named2} from 'mod'; named2 += 0": [{ col: 28, message: MESSAGE }],
      "import {named3} from 'mod'; named3++": [{ col: 28, message: MESSAGE }],
      "import {named4} from 'mod'; for (named4 in foo);": [{ col: 33, message: MESSAGE }],
      "import {named5} from 'mod'; for (named5 of foo);": [{ col: 33, message: MESSAGE }],
      "import {named6} from 'mod'; [named6] = foo": [{ col: 29, message: MESSAGE }],
      "import {named7} from 'mod'; [named7 = 0] = foo": [{ col: 29, message: MESSAGE }],
      "import {named8} from 'mod'; [...named8] = foo": [{ col: 32, message: MESSAGE }],
      "import {named9} from 'mod'; ({ bar: named9 } = foo)": [{ col: 36, message: MESSAGE }],
      "import {named10} from 'mod'; ({ bar: named10 = 0 } = foo)": [{ col: 37, message: MESSAGE }],
      "import {named11} from 'mod'; ({ ...named11 } = foo)": [{ col: 35, message: MESSAGE }],
      "import {named12 as foo} from 'mod'; foo = 0; named12 = 0": [{ col: 36, message: MESSAGE }],
      "import * as mod1 from 'mod'; mod1 = 0": [{ col: 29, message: MESSAGE }],
      "import * as mod2 from 'mod'; mod2 += 0": [{ col: 29, message: MESSAGE }],
      "import * as mod3 from 'mod'; mod3++": [{ col: 29, message: MESSAGE }],
      "import * as mod4 from 'mod'; for (mod4 in foo);": [{ col: 34, message: MESSAGE }],
      "import * as mod5 from 'mod'; for (mod5 of foo);": [{ col: 34, message: MESSAGE }],
      "import * as mod6 from 'mod'; [mod6] = foo": [{ col: 30, message: MESSAGE }],
      "import * as mod7 from 'mod'; [mod7 = 0] = foo": [{ col: 30, message: MESSAGE }],
      "import * as mod8 from 'mod'; [...mod8] = foo": [{ col: 33, message: MESSAGE }],
      "import * as mod9 from 'mod'; ({ bar: mod9 } = foo)": [{ col: 37, message: MESSAGE }],
      "import * as mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)": [{ col: 38, message: MESSAGE }],
      "import * as mod11 from 'mod'; ({ ...mod11 } = foo)": [{ col: 36, message: MESSAGE }],
      "import * as mod1 from 'mod'; mod1.named = 0": [{ col: 29, message: MESSAGE }],
      "import * as mod2 from 'mod'; mod2.named += 0": [{ col: 29, message: MESSAGE }],
      "import * as mod3 from 'mod'; mod3.named++": [{ col: 29, message: MESSAGE }],
      "import * as mod4 from 'mod'; for (mod4.named in foo);": [{ col: 34, message: MESSAGE }],
      "import * as mod5 from 'mod'; for (mod5.named of foo);": [{ col: 34, message: MESSAGE }],
      "import * as mod6 from 'mod'; [mod6.named] = foo": [{ col: 30, message: MESSAGE }],
      "import * as mod7 from 'mod'; [mod7.named = 0] = foo": [{ col: 30, message: MESSAGE }],
      "import * as mod8 from 'mod'; [...mod8.named] = foo": [{ col: 33, message: MESSAGE }],
      "import * as mod9 from 'mod'; ({ bar: mod9.named } = foo)": [{ col: 37, message: MESSAGE }],
      "import * as mod10 from 'mod'; ({ bar: mod10.named = 0 } = foo)": [{ col: 38, message: MESSAGE }],
      "import * as mod11 from 'mod'; ({ ...mod11.named } = foo)": [{ col: 36, message: MESSAGE }],
      "import * as mod12 from 'mod'; delete mod12.named": [{ col: 30, message: MESSAGE }],
      "import * as mod from 'mod'; Object.assign(mod, obj)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; Object.defineProperty(mod, key, d)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; Object.setPrototypeOf(mod, proto)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; Object.freeze(mod)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; Reflect.defineProperty(mod, key, d)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; Reflect.deleteProperty(mod, key)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; Reflect.set(mod, key, value)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; Reflect.setPrototypeOf(mod, proto)": [{ col: 28, message: MESSAGE }],
      "import mod, * as mod_ns from 'mod'; mod.prop = 0; mod_ns.prop = 0": [{ col: 50, message: MESSAGE }],
      "import * as mod from 'mod'; Object?.defineProperty(mod, key, d)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; (Object?.defineProperty)(mod, key, d)": [{ col: 28, message: MESSAGE }],
      "import * as mod from 'mod'; delete mod?.prop": [{ col: 28, message: MESSAGE }],
    }
  }
}
