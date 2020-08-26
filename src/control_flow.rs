use swc_ecmascript::ast::{BlockStmt, Stmt};

pub trait EndsWithRet {
  /// Does a node ends with return, throw, break, continue?
  fn ends_with_ret(&self) -> bool;
}

impl EndsWithRet for Stmt {
  fn ends_with_ret(&self) -> bool {
    match self {
      Stmt::Return(_) | Stmt::Break(_) | Stmt::Continue(_) | Stmt::Throw(_) => {
        true
      }

      Stmt::Block(s) => s.ends_with_ret(),
      Stmt::With(s) => s.body.ends_with_ret(),
      Stmt::Labeled(s) => s.body.ends_with_ret(),

      _ => false,
    }
  }
}

impl EndsWithRet for BlockStmt {
  fn ends_with_ret(&self) -> bool {
    self.stmts.ends_with_ret()
  }
}

impl EndsWithRet for Vec<Stmt> {
  fn ends_with_ret(&self) -> bool {
    self.last().map(|s| s.ends_with_ret()).unwrap_or(false)
  }
}
