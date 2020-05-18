use super::{Context, LintRule};
use swc_ecma_ast::Expr::Unary;
use swc_ecma_ast::UnaryOp::TypeOf;
use swc_ecma_ast::{BinExpr, Module};
use swc_ecma_visit::{Node, Visit};
pub struct ValidTypeof;

impl LintRule for ValidTypeof {
  fn new() -> Box<Self> {
    Box::new(ValidTypeof)
  }

  fn lint_module(&self, context: Context, module: Module) {
    let mut visitor = ValidTypeofVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct ValidTypeofVisitor {
  context: Context,
}

impl ValidTypeofVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for ValidTypeofVisitor {
  fn visit_bin_expr(&mut self, bin_expr: &BinExpr, _parent: &dyn Node) {
    match bin_expr.left {
      Unary(unary) => match unary.op {
        TypeOf(t) => {}
        _ => {}
      },
      _ => {}
    }
  }
}
