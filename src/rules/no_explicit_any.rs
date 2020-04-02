// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use swc_ecma_ast::TsTypeAnn;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;
pub struct NoExplicitAny {
  context: Context,
}

impl NoExplicitAny {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoExplicitAny {
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
