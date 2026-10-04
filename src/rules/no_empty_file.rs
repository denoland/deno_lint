// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::Program;
use deno_ast::swc::common::comments::Comment;
use deno_ast::swc::common::comments::CommentKind;
use deno_ast::view::{Expr, Lit, ModuleItem, Stmt};
use deno_ast::{SourceRange, SourceRanged};
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::Regex;

#[derive(Debug)]
pub struct NoEmptyFile;

const CODE: &str = "no-empty-file";

#[derive(Display)]
enum NoEmptyFileMessage {
  #[display(fmt = "Empty files are not allowed")]
  Empty,
}

#[derive(Display)]
enum NoEmptyFileHint {
  #[display(fmt = "Delete this file or add some code to it")]
  DeleteOrAddCode,
}

impl LintRule for NoEmptyFile {
  fn tags(&self) -> crate::tags::Tags {
    &[crate::tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    // A triple-slash reference directive counts as meaningful content even
    // though it is technically a comment, so such files are exempt.
    if context.all_comments().any(is_triple_slash_reference) {
      return;
    }

    let is_empty = match program {
      Program::Module(module) => module.body.iter().all(is_empty_module_item),
      Program::Script(script) => {
        script.body.iter().all(is_empty_top_level_stmt)
      }
    };

    if !is_empty {
      return;
    }

    // Cap the reported span at 100 characters to avoid emitting an enormous
    // diagnostic for comment-heavy (but otherwise empty) files.
    let start = program.start();
    let text = program.text_fast(context.text_info());
    let range = match text.char_indices().nth(100) {
      Some((byte_offset, _)) => SourceRange::new(start, start + byte_offset),
      None => program.range(),
    };

    context.add_diagnostic_with_hint(
      range,
      CODE,
      NoEmptyFileMessage::Empty,
      NoEmptyFileHint::DeleteOrAddCode,
    );
  }
}

fn is_empty_module_item(item: &ModuleItem) -> bool {
  match item {
    // Imports and exports are meaningful code.
    ModuleItem::ModuleDecl(_) => false,
    ModuleItem::Stmt(stmt) => is_empty_top_level_stmt(stmt),
  }
}

fn is_empty_top_level_stmt(stmt: &Stmt) -> bool {
  match stmt {
    Stmt::Empty(_) => true,
    // A bare string literal statement at the top level is a directive
    // (e.g. `"use strict";`) and carries no real code.
    Stmt::Expr(expr_stmt) => matches!(expr_stmt.expr, Expr::Lit(Lit::Str(_))),
    Stmt::Block(block) => block.stmts.iter().all(is_empty_nested_stmt),
    _ => false,
  }
}

fn is_empty_nested_stmt(stmt: &Stmt) -> bool {
  match stmt {
    Stmt::Empty(_) => true,
    Stmt::Block(block) => block.stmts.iter().all(is_empty_nested_stmt),
    // Unlike at the top level, a string literal nested in a block is a real
    // expression statement rather than a directive.
    _ => false,
  }
}

fn is_triple_slash_reference(comment: &Comment) -> bool {
  if comment.kind != CommentKind::Line {
    return false;
  }

  static TSR_REGEX: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^/\s*<reference\s*(types|path|lib)\s*=\s*["|'](.*)["|']"#)
      .unwrap()
  });

  TSR_REGEX.is_match(&comment.text)
}

#[cfg(test)]
mod tests {
  use super::*;

  // Some tests are derived from
  // https://github.com/oxc-project/oxc/blob/main/crates/oxc_linter/src/rules/unicorn/no_empty_file.rs
  // MIT Licensed.

  #[test]
  fn no_empty_file_valid() {
    assert_lint_ok! {
      NoEmptyFile,
      "const x = 0;",
      ";; const x = 0;",
      "{{{;;const x = 0;}}}",
      "'use strict';\nconst x = 0;",
      ";;'use strict';\nconst x = 0;",
      "{'use strict';}",
      r#"("use strict")"#,
      "`use strict`",
      "({})",
      "#!/usr/bin/env node\nconsole.log('done');",
      "false",
      r#"("")"#,
      "NaN",
      "undefined",
      "null",
      "[]",
      "(() => {})()",
      "(() => {})();",
      r#"/// <reference types="vite/client" />"#,
    };
  }

  #[test]
  fn no_empty_file_invalid() {
    assert_lint_err! {
      NoEmptyFile,
      "": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      " ": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "\t": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "\n": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "\r": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "\r\n": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "// comment": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "/* comment */": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "#!/usr/bin/env node": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "'use asm';": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "'use strict';": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      r#""use strict""#: [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      r#""""#: [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      ";": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      ";;": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "{}": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "{;;}": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      "{{}}": [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      r#""";"#: [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
      r#""use strict";"#: [{ col: 0, message: NoEmptyFileMessage::Empty, hint: NoEmptyFileHint::DeleteOrAddCode }],
    };
  }
}
