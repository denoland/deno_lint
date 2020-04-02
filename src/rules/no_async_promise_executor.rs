// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::TsTypeAnn;

pub struct NoAsyncPromiseExecutor {
  context: Context,
}

impl NoAsyncPromiseExecutor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoAsyncPromiseExecutor {
  fn walk_ts_type_ann(&self, type_ann: TsTypeAnn) {
    use swc_ecma_ast::TsKeywordTypeKind::*;
    use swc_ecma_ast::TsType::*;

    match &*type_ann.type_ann {
      TsKeywordType(keyword_type) => match keyword_type.kind {
        TsAnyKeyword => {
          self.context.add_diagnostic(
            &type_ann.span,
            "noExplicitAny",
            "`any` type is not allowed",
          );
        }
        _ => {}
      },
      _ => {}
    }
  }
}
