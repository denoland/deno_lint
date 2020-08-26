use std::collections::HashMap;
use swc_common::BytePos;
use swc_ecmascript::ast::*;
use swc_ecmascript::visit::{Node, Visit};

pub struct Info {
  stmts: HashMap<BytePos, StmtInfo>,
}

impl Info {
  pub fn stmt(&self, lo: BytePos) -> Option<&StmtInfo> {
    self.stmts.get(&lo)
  }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlockKind {}

#[derive(Debug, Clone)]
pub struct StmtInfo {
  path: Vec<BlockKind>,
}

impl StmtInfo {
  pub fn path(&self) -> &[BlockKind] {
    &self.path
  }
}

struct Scope<'a> {
  parent: Option<&'a Scope<'a>>,
}

impl Visit for Info {
  fn visit_fn_decl(&mut self, n: &FnDecl, _: &dyn Node) {}
}

pub(crate) trait EndsWithRet {
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
      Stmt::If(s) => s.cons.ends_with_ret() && s.alt.ends_with_ret(),
      Stmt::Switch(s) => s.cases.iter().all(|case| case.cons.ends_with_ret()),
      Stmt::Try(s) => match s {
        TryStmt {
          finalizer: None, ..
        } => s.block.ends_with_ret(),
        // TODO: Improve to eslint grade
        _ => s.finalizer.ends_with_ret(),
      },
      // Stmt::While(_) => {}
      // Stmt::DoWhile(_) => {}
      // Stmt::For(_) => {}
      // Stmt::ForIn(_) => {}
      // Stmt::ForOf(_) => {}
      Stmt::Decl(_) | Stmt::Expr(_) => false,
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

impl<T> EndsWithRet for Option<T>
where
  T: EndsWithRet,
{
  fn ends_with_ret(&self) -> bool {
    self.as_ref().map(|s| s.ends_with_ret()).unwrap_or(false)
  }
}

impl<T> EndsWithRet for Box<T>
where
  T: EndsWithRet,
{
  fn ends_with_ret(&self) -> bool {
    (**self).ends_with_ret()
  }
}
