// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::TsTypeAnn;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoExplicitAny;

impl LintRule for NoExplicitAny {
  fn new() -> Box<Self> {
    Box::new(NoExplicitAny)
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoExplicitAnyVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoExplicitAnyVisitor {
  context: Context,
}

impl NoExplicitAnyVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoExplicitAnyVisitor {
  fn visit_ts_type_ann(&mut self, type_ann: &TsTypeAnn, _parent: &dyn Node) {
    use swc_ecma_ast::TsKeywordTypeKind::*;
    use swc_ecma_ast::TsType::*;

    if let TsKeywordType(keyword_type) = &*type_ann.type_ann {
      if keyword_type.kind == TsAnyKeyword {
        self.context.add_diagnostic(
          type_ann.span,
          "noExplicitAny",
          "`any` type is not allowed",
        );
      }
    }
  }
}
