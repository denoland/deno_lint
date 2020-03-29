use super::Context;
use crate::traverse::AstTraverser;
use swc_ecma_ast::TsTypeAnn;

pub struct NoExplicitAny {
  context: Context,
}

impl NoExplicitAny {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl AstTraverser for NoExplicitAny {
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
