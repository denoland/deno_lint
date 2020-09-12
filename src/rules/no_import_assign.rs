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

  fn check(&self, i: Id) {
    if self.imports.contains(&i) {}
  }
}

impl Visit for NoImportAssignVisitor {
  noop_visit_type!();

  fn visit_pat(&mut self, n: &Pat, _: &dyn Node) {
    if let Pat::Ident(i) = n {
      self.check(i.to_id());
    } else {
      n.visit_children_with(self);
    }
  }

  fn visit_assign_pat_prop(&mut self, n: &AssignPatProp, _: &dyn Node) {
    self.check(n.key.to_id());

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
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  // "import * as mod from 'mod'; Object.assign(obj, mod, other);",
  // "import * as mod from 'mod'; Object[assign](mod, obj);",
  // "import * as mod from 'mod'; Object.getPrototypeOf(mod);",
  // "import * as mod from 'mod'; Reflect.set(obj, key, mod);",
  // "import * as mod from 'mod'; { var Object; Object.assign(mod, obj); }",
  // "import * as mod from 'mod'; var Object; Object.assign(mod, obj);",
  // "import * as mod from 'mod'; Object.seal(mod, obj)",
  // "import * as mod from 'mod'; Object.preventExtensions(mod)",
  // "import * as mod from 'mod'; Reflect.preventExtensions(mod)"

  #[test]
  fn ok_18() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }

  #[test]
  fn ok_19() {
    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");

    assert_lint_ok::<NoImportAssign>("");
  }
}
