// Copyright 2020-2024 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::tags::Tags;

use deno_ast::oxc::ast::ast::{
  IdentifierReference, ImportDeclaration, Program,
};
use deno_ast::oxc::span::GetSpan;
use deno_ast::oxc::span::Span;

#[derive(Debug)]
pub struct NoProcessGlobal;

const CODE: &str = "no-process-global";
const MESSAGE: &str = "NodeJS process global is discouraged in Deno";

impl LintRule for NoProcessGlobal {
  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoProcessGlobalHandler {
      most_recent_import_span: None,
    };
    crate::handler::traverse_program(&mut handler, program, context);
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn tags(&self) -> Tags {
    &[]
  }
}

struct NoProcessGlobalHandler {
  most_recent_import_span: Option<Span>,
}

fn program_code_start(program: &Program) -> u32 {
  program
    .body
    .first()
    .map(|node| node.span().start)
    .unwrap_or(program.span.start)
}

impl NoProcessGlobalHandler {
  fn fix_change(&self, ctx: &mut Context) -> LintFixChange {
    // If the fix is an import, we want to insert it after the last import
    // statement. If there are no import statements, we want to insert it at
    // the beginning of the file (but after any header comments).
    let (fix_range, leading, trailing) =
      if let Some(span) = self.most_recent_import_span {
        (Span::new(span.end, span.end), "\n", "")
      } else {
        let code_start = program_code_start(ctx.program());
        (Span::new(code_start, code_start), "", "\n")
      };

    LintFixChange {
      new_text: format!(
        "{leading}import process from \"node:process\";{trailing}"
      )
      .into(),
      range: fix_range,
    }
  }

  fn add_diagnostic(&mut self, ctx: &mut Context, span: Span) {
    let change = self.fix_change(ctx);

    ctx.add_diagnostic_with_fixes(
      span,
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

impl Handler<'_> for NoProcessGlobalHandler {
  fn identifier_reference(
    &mut self,
    id: &IdentifierReference,
    ctx: &mut Context,
  ) {
    if id.name.as_str() != "process" {
      return;
    }
    if ctx.scope().var_by_name(id.name.as_str()).is_none() {
      self.add_diagnostic(ctx, id.span);
    }
  }

  fn import_declaration(
    &mut self,
    imp: &ImportDeclaration,
    _ctx: &mut Context,
  ) {
    self.most_recent_import_span = Some(imp.span);
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
