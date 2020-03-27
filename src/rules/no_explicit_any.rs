use super::Context;
use swc_common::Visit;
use swc_common::VisitWith;

pub struct NoExplicitAny {
  context: Context,
}

impl NoExplicitAny {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl<T> Visit<T> for NoExplicitAny
where
  T: VisitWith<Self>,
{
  default fn visit(&mut self, n: &T) {
    n.visit_children(self)
  }
}

impl Visit<swc_ecma_ast::TsTypeAnn> for NoExplicitAny {
  fn visit(&mut self, node: &swc_ecma_ast::TsTypeAnn) {
    use swc_ecma_ast::TsKeywordTypeKind::*;
    use swc_ecma_ast::TsType::*;

    match &*node.type_ann {
      TsKeywordType(keyword_type) => match keyword_type.kind {
        TsAnyKeyword => {
          self.context.add_diagnostic(
            &node.span,
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
