// Copyright 2020-2024 the Deno authors. All rights reserved. MIT license.
use super::program_ref;
use super::Context;
use super::LintRule;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::tags;
use crate::tags::Tags;
use crate::Program;

use deno_ast::view as ast_view;
use deno_ast::SourcePos;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;

#[derive(Debug)]
pub struct NoProcessGlobal;

const CODE: &str = "no-process-global";
const MESSAGE: &str = "NodeJS process global is discouraged in Deno";

impl LintRule for NoProcessGlobal {
  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    NoProcessGlobalHandler {
      most_recent_import_range: None,
    }
    .traverse(program, context);
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }
}

struct NoProcessGlobalHandler {
  most_recent_import_range: Option<SourceRange>,
}

fn program_code_start(program: Program) -> SourcePos {
  match program_ref(program) {
    ast_view::ProgramRef::Module(m) => m
      .body
      .first()
      .map(|node| node.start())
      .unwrap_or(program.start()),
    ast_view::ProgramRef::Script(s) => s
      .body
      .first()
      .map(|node| node.start())
      .unwrap_or(program.start()),
  }
}

impl NoProcessGlobalHandler {
  fn fix_change(&self, ctx: &mut Context) -> LintFixChange {
    // If the fix is an import, we want to insert it after the last import
    // statement. If there are no import statements, we want to insert it at
    // the beginning of the file (but after any header comments).
    let (fix_range, leading, trailing) =
      if let Some(range) = self.most_recent_import_range {
        (SourceRange::new(range.end(), range.end()), "\n", "")
      } else {
        let code_start = program_code_start(ctx.program());
        (SourceRange::new(code_start, code_start), "", "\n")
      };

    LintFixChange {
      new_text: format!(
        "{leading}import process from \"node:process\";{trailing}"
      )
      .into(),
      range: fix_range,
    }
  }

  fn add_diagnostic(&mut self, ctx: &mut Context, range: SourceRange) {
    let change = self.fix_change(ctx);

    ctx.add_diagnostic_with_fixes(
      range,
      CODE,
      MESSAGE,
      Some(String::from("Add `import process from \"node:process\";`")),
      vec![LintFix {
        description: "Import from \"node:process\"".into(),
        changes: vec![change],
      }],
    );
  }
}

impl Handler for NoProcessGlobalHandler {
  fn ident(&mut self, id: &ast_view::Ident, ctx: &mut Context) {
    if id.sym() != "process" {
      return;
    }
    if id.ctxt() == ctx.unresolved_ctxt() {
      self.add_diagnostic(ctx, id.range());
    }
  }

  fn import_decl(&mut self, imp: &ast_view::ImportDecl, _ctx: &mut Context) {
    self.most_recent_import_range = Some(imp.range());
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn valid() {
    assert_lint_ok! {
      NoProcessGlobal,
      "import process from 'node:process';\nconst a = process.env;",
      "const process = { env: {} };\nconst a = process.env;",
    }
  }

  #[test]
  fn invalid() {
    assert_lint_err! {
      NoProcessGlobal,
      "import a from 'b';\nconst e = process.env;": [
        {
          col: 10,
          line: 2,
          message: MESSAGE,
          hint: "Add `import process from \"node:process\";`",
          fix: (
            "Import from \"node:process\"",
            "import a from 'b';\nimport process from \"node:process\";\nconst e = process.env;"
          ),
        }
      ],
      "const a = process;": [
        {
          col: 10,
          line: 1,
          message: MESSAGE,
          hint: "Add `import process from \"node:process\";`",
          fix: (
            "Import from \"node:process\"",
            "import process from \"node:process\";\nconst a = process;"
          ),
        }
      ],
      "// A copyright notice\n\nconst a = process.env;": [
        {
          col: 10,
          line: 3,
          message: MESSAGE,
          hint: "Add `import process from \"node:process\";`",
          fix: (
            "Import from \"node:process\"",
            "// A copyright notice\n\nimport process from \"node:process\";\nconst a = process.env;"
          ),
        }
      ]
    };
  }
}
