// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use std::sync::Arc;
use std::sync::Mutex;
use swc_common::comments::CommentMap;
use swc_common::SourceMap;
use swc_common::Span;

mod explicit_function_return_type;
pub use explicit_function_return_type::ExplicitFunctionReturnType;
mod no_debugger;
pub use no_debugger::NoDebugger;
mod no_eval;
pub use no_eval::NoEval;
mod no_explicit_any;
pub use no_explicit_any::NoExplicitAny;
mod no_var;
pub use no_var::NoVar;
mod single_var_declarator;
pub use single_var_declarator::SingleVarDeclarator;
mod ban_ts_ignore;
pub use ban_ts_ignore::BanTsIgnore;
mod ban_untagged_todo;
pub use ban_untagged_todo::BanUntaggedTodo;
mod no_empty_interface;
pub use no_empty_interface::NoEmptyInterface;
mod no_delete_var;
pub use no_delete_var::NoDeleteVar;
mod use_isnan;
pub use use_isnan::UseIsNaN;
mod no_empty_function;
pub use no_empty_function::NoEmptyFunction;
mod no_async_promise_executor;
pub use no_async_promise_executor::NoAsyncPromiseExecutor;
mod no_sparse_array;
pub use no_sparse_array::NoSparseArray;
mod no_duplicate_case;
pub use no_duplicate_case::NoDuplicateCase;

#[derive(Debug, Clone)]
pub struct Location {
  pub filename: String,
  pub line: usize,
  pub col: usize,
}

impl Into<Location> for swc_common::Loc {
  fn into(self) -> Location {
    use swc_common::FileName::*;

    let filename = match &self.file.name {
      Real(path_buf) => path_buf.to_string_lossy().to_string(),
      Custom(str_) => str_.to_string(),
      _ => panic!("invalid filename"),
    };

    Location {
      filename,
      line: self.line,
      col: self.col_display,
    }
  }
}

#[derive(Debug)]
pub struct LintDiagnotic {
  pub location: Location,
  pub message: String,
  pub code: String,
}

#[derive(Clone)]
pub struct Context {
  pub file_name: String,
  pub diagnostics: Arc<Mutex<Vec<LintDiagnotic>>>,
  pub source_map: Arc<SourceMap>,
  pub leading_comments: CommentMap,
  pub trailing_comments: CommentMap,
}

impl Context {
  pub fn add_diagnostic(&self, span: &Span, code: &str, message: &str) {
    let location = self.source_map.lookup_char_pos(span.lo());
    let mut diags = self.diagnostics.lock().unwrap();
    diags.push(LintDiagnotic {
      location: location.into(),
      message: message.to_string(),
      code: code.to_string(),
    });
  }
}
