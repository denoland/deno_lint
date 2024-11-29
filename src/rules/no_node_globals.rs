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
use std::borrow::Cow;

use deno_ast::view as ast_view;
use deno_ast::SourcePos;
use deno_ast::SourceRange;
use deno_ast::SourceRanged;
use deno_ast::SourceRangedForSpanned;

#[derive(Debug)]
pub struct NoNodeGlobals;

const CODE: &str = "no-node-globals";
const MESSAGE: &str = "NodeJS globals are not available in Deno";

static NODE_GLOBALS: phf::Map<&'static str, FixKind> = phf::phf_map! {
  "Buffer" => FixKind::Import { module: "node:buffer", import: "{ Buffer }" },
  "global" => FixKind::Replace("globalThis"),
  "setImmediate" => FixKind::Import { module: "node:timers", import: "{ setImmediate }" },
  "clearImmediate" => FixKind::Import { module: "node:timers", import: "{ clearImmediate }" },
};

impl LintRule for NoNodeGlobals {
  fn lint_program_with_ast_view<'view>(
    &self,
    context: &mut Context<'view>,
    program: Program<'view>,
  ) {
    NoNodeGlobalsHandler {
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

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_node_globals.md")
  }
}

struct NoNodeGlobalsHandler {
  most_recent_import_range: Option<SourceRange>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FixKind {
  Import {
    module: &'static str,
    import: &'static str,
  },
  Replace(&'static str),
}

#[derive(Default)]
enum AddNewline {
  Leading,
  Trailing,
  #[default]
  None,
}

impl FixKind {
  fn hint(&self) -> String {
    match self {
      FixKind::Import { import, module } => {
        format!("Add `import {} from \"{}\";`", import, module)
      }
      FixKind::Replace(new) => format!("Use {new} instead"),
    }
  }

  fn description(&self) -> String {
    match self {
      FixKind::Import { module, .. } => {
        format!("Import from \"{module}\"")
      }
      FixKind::Replace(new) => format!("Replace with {new}"),
    }
  }

  fn to_text(self, newline: AddNewline) -> Cow<'static, str> {
    match self {
      FixKind::Import { module, import } => {
        let (leading, trailing) = match newline {
          AddNewline::Leading => ("\n", ""),
          AddNewline::Trailing => ("", "\n"),
          AddNewline::None => ("", ""),
        };
        format!("{leading}import {import} from \"{module}\";{trailing}").into()
      }
      FixKind::Replace(new_text) => new_text.into(),
    }
  }
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

impl NoNodeGlobalsHandler {
  fn fix_change(
    &self,
    ctx: &mut Context,
    range: SourceRange,
    fix_kind: FixKind,
  ) -> LintFixChange {
    // If the fix is an import, we want to insert it after the last import
    // statement. If there are no import statements, we want to insert it at
    // the beginning of the file (but after any header comments).
    let (fix_range, add_newline) = if matches!(fix_kind, FixKind::Import { .. })
    {
      if let Some(range) = self.most_recent_import_range {
        (
          SourceRange::new(range.end(), range.end()),
          AddNewline::Leading,
        )
      } else {
        let code_start = program_code_start(ctx.program());
        (
          SourceRange::new(code_start, code_start),
          AddNewline::Trailing,
        )
      }
    } else {
      (range, AddNewline::None)
    };
    LintFixChange {
      new_text: fix_kind.to_text(add_newline),
      range: fix_range,
    }
  }
  fn add_diagnostic(
    &mut self,
    ctx: &mut Context,
    range: SourceRange,
    fix_kind: FixKind,
  ) {
    let change = self.fix_change(ctx, range, fix_kind);

    ctx.add_diagnostic_with_fixes(
      range,
      CODE,
      MESSAGE,
      Some(fix_kind.hint().to_string()),
      vec![LintFix {
        description: fix_kind.description().into(),
        changes: vec![change],
      }],
    );
  }
}

impl Handler for NoNodeGlobalsHandler {
  fn ident(&mut self, id: &ast_view::Ident, ctx: &mut Context) {
    if !NODE_GLOBALS.contains_key(id.sym()) {
      return;
    }
    if id.ctxt() == ctx.unresolved_ctxt() {
      self.add_diagnostic(ctx, id.range(), NODE_GLOBALS[id.sym()]);
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
      NoNodeGlobals,
      "import { Buffer } from 'node:buffer';\nconst b = Buffer;",
      "const Buffer = {};\nconst b = Buffer;",
      "const global = globalThis;\nconst c = global;",
      "const setImmediate = () => {};\nconst d = setImmediate;",
      "const clearImmediate = () => {};\nconst e = clearImmediate;",
    }
  }

  #[test]
  fn invalid() {
    assert_lint_err! {
      NoNodeGlobals,
      "const b = Buffer;": [
        {
          col: 10,
          line: 1,
          message: MESSAGE,
          hint: "Add `import { Buffer } from \"node:buffer\";`",
          fix: (
            "Import from \"node:buffer\"",
            "import { Buffer } from \"node:buffer\";\nconst b = Buffer;"
          ),
        }
      ],
      "const c = global;": [
        {
          col: 10,
          line: 1,
          message: MESSAGE,
          hint: "Use globalThis instead",
          fix: (
            "Replace with globalThis",
            "const c = globalThis;"
          ),
        }
      ],
      "const d = setImmediate;": [
        {
          col: 10,
          line: 1,
          message: MESSAGE,
          hint: "Add `import { setImmediate } from \"node:timers\";`",
          fix: (
            "Import from \"node:timers\"",
            "import { setImmediate } from \"node:timers\";\nconst d = setImmediate;"
          ),
        }
      ],
      "const e = clearImmediate;": [
        {
          col: 10,
          line: 1,
          message: MESSAGE,
          hint: "Add `import { clearImmediate } from \"node:timers\";`",
          fix: (
            "Import from \"node:timers\"",
            "import { clearImmediate } from \"node:timers\";\nconst e = clearImmediate;"
          ),
        }
      ],
      "const a = setImmediate;\nconst b = Buffer;": [
        {
          col: 10,
          line: 1,
          message: MESSAGE,
          hint: "Add `import { setImmediate } from \"node:timers\";`",
          fix: (
            "Import from \"node:timers\"",
            "import { setImmediate } from \"node:timers\";\nconst a = setImmediate;\nconst b = Buffer;"
          ),
        },
        {
          col: 10,
          line: 2,
          message: MESSAGE,
          hint: "Add `import { Buffer } from \"node:buffer\";`",
          fix: (
            "Import from \"node:buffer\"",
            "import { Buffer } from \"node:buffer\";\nconst a = setImmediate;\nconst b = Buffer;"
          ),
        }
      ],
      "// A copyright notice\n\nconst a = setImmediate;": [
        {
          col: 10,
          line: 3,
          message: MESSAGE,
          hint: "Add `import { setImmediate } from \"node:timers\";`",
          fix: (
            "Import from \"node:timers\"",
            "// A copyright notice\n\nimport { setImmediate } from \"node:timers\";\nconst a = setImmediate;"
          ),
        }
      ]
    };
  }
}
