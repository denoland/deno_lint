use super::LintRule;
use crate::linter::Context;
use std::{collections::HashSet, sync::Arc};
use swc_ecmascript::{
  ast::*,
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
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut collector = Collector {
      imports: Default::default(),
    };
    module.visit_with(module, &mut collector);

    let mut visitor = NoImportAssignVisitor::new(context, collector.imports);
    module.visit_with(module, &mut visitor);
  }
}

struct Collector {
  imports: HashSet<Id>,
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
    self.imports.insert(i.local.to_id());
  }
}

struct NoImportAssignVisitor {
  context: Arc<Context>,
  /// This hashset only contains top level bindings, so using HashSet<JsWord>
  /// also can be an option.
  imports: HashSet<Id>,
}

impl NoImportAssignVisitor {
  fn new(context: Arc<Context>, imports: HashSet<Id>) -> Self {
    Self { context, imports }
  }

  fn check(&self, i: &Ident) {
    // We only care about imports
    if !self.imports.contains(&i.to_id()) {
      return;
    }

    self.context.add_diagnostic(
      i.span,
      "no-import-assign",
      "Assignment to import is not allowed",
    );
  }
}

impl Visit for NoImportAssignVisitor {
  noop_visit_type!();

  fn visit_pat(&mut self, n: &Pat, _: &dyn Node) {
    if let Pat::Ident(i) = n {
      self.check(&i);
    } else {
      n.visit_children_with(self);
    }
  }

  fn visit_assign_pat_prop(&mut self, n: &AssignPatProp, _: &dyn Node) {
    self.check(&n.key);

    n.value.visit_children_with(self);
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
    assert_lint_err::<NoImportAssign>("import mod1 from 'mod'; mod1 = 0", 0);

    assert_lint_err::<NoImportAssign>("import mod2 from 'mod'; mod2 += 0", 0);

    assert_lint_err::<NoImportAssign>("import mod3 from 'mod'; mod3++", 0);
  }

  #[test]
  fn err_2() {
    assert_lint_err::<NoImportAssign>(
      "import mod4 from 'mod'; for (mod4 in foo);",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod5 from 'mod'; for (mod5 of foo);",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod6 from 'mod'; [mod6] = foo",
      0,
    );
  }

  #[test]
  fn err_3() {
    assert_lint_err::<NoImportAssign>(
      "import mod7 from 'mod'; [mod7 = 0] = foo",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod8 from 'mod'; [...mod8] = foo",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod9 from 'mod'; ({ bar: mod9 } = foo)",
      0,
    );
  }

  #[test]
  fn err_4() {
    assert_lint_err::<NoImportAssign>(
      "import mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import mod11 from 'mod'; ({ ...mod11 } = foo)",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named1} from 'mod'; named1 = 0",
      0,
    );
  }

  #[test]
  fn err_5() {
    assert_lint_err::<NoImportAssign>(
      "import {named2} from 'mod'; named2 += 0",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named3} from 'mod'; named3++",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named4} from 'mod'; for (named4 in foo);",
      0,
    );
  }

  #[test]
  fn err_6() {
    assert_lint_err::<NoImportAssign>(
      "import {named5} from 'mod'; for (named5 of foo);",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named6} from 'mod'; [named6] = foo",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named7} from 'mod'; [named7 = 0] = foo",
      0,
    );
  }

  #[test]
  fn err_7() {
    assert_lint_err::<NoImportAssign>(
      "import {named8} from 'mod'; [...named8] = foo",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named9} from 'mod'; ({ bar: named9 } = foo)",
      0,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named10} from 'mod'; ({ bar: named10 = 0 } = foo)",
      0,
    );
  }

  #[test]
  fn err_8() {
    assert_lint_err::<NoImportAssign>(
      "import {named11} from 'mod'; ({ ...named11 } = foo)",
      31,
    );

    assert_lint_err::<NoImportAssign>(
      "import {named12 as foo} from 'mod'; foo = 0; named12 = 0",
      37,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod1 from 'mod'; mod1 = 0",
      30,
    );
  }

  #[test]
  fn err_9() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod2 from 'mod'; mod2 += ",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod3 from 'mod'; mod3++",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod4 from 'mod'; for (mod4 in foo);",
      30,
    );
  }

  #[test]
  fn err_10() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod5 from 'mod'; for (mod5 of foo);",
      30,
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
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod9 from 'mod'; ({ bar: mod9 } = foo)",
      31,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod10 from 'mod'; ({ bar: mod10 = 0 } = foo)",
      32,
    );
  }

  #[test]
  fn err_12() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod11 from 'mod'; ({ ...mod11 } = foo)",
      32,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod1 from 'mod'; mod1.named = 0",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod2 from 'mod'; mod2.named += 0",
      30,
    );
  }

  #[test]
  fn err_13() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod3 from 'mod'; mod3.named++",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod4 from 'mod'; for (mod4.named in foo);",
      30,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod5 from 'mod'; for (mod5.named of foo);",
      30,
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
      30,
    );
  }

  #[test]
  fn err_15() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod9 from 'mod'; ({ bar: mod9.named } = foo)",
      31,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod10 from 'mod'; ({ bar: mod10.named = 0 } = foo)",
      32,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod11 from 'mod'; ({ ...mod11.named } = foo)",
      32,
    );
  }

  #[test]
  fn err_16() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod12 from 'mod'; delete mod12.named",
      31,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.assign(mod, obj)",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.defineProperty(mod, key, d)",
      29,
    );
  }

  #[test]
  fn err_17() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.setPrototypeOf(mod, proto)",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object.freeze(mod)",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.defineProperty(mod, key, d)",
      29,
    );
  }

  #[test]
  fn err_18() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.deleteProperty(mod, key)",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.set(mod, key, value)",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Reflect.setPrototypeOf(mod, proto)",
      29,
    );
  }

  #[test]
  fn err_19() {
    assert_lint_err::<NoImportAssign>(
      "import mod, * as mod_ns from 'mod'; mod.prop = 0; mod_ns.prop = 0",
      51,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; Object?.defineProperty(mod, key, d)",
      29,
    );

    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; (Object?.defineProperty)(mod, key, d)",
      29,
    );
  }

  #[test]
  fn err_20() {
    assert_lint_err::<NoImportAssign>(
      "import * as mod from 'mod'; delete mod?.prop",
      29,
    );
  }
}
