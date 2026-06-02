// Copyright 2020-2024 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::diagnostic::LintFix;
use crate::diagnostic::LintFixChange;
use crate::handler::Handler;
use crate::tags::Tags;
use std::borrow::Cow;

use deno_ast::oxc::ast::ast::{
  IdentifierReference, ImportDeclaration, Program,
};
use deno_ast::oxc::span::GetSpan;
use deno_ast::oxc::span::Span;

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
  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoNodeGlobalsHandler {
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

struct NoNodeGlobalsHandler {
  most_recent_import_span: Option<Span>,
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

fn program_code_start(program: &Program) -> u32 {
  program
    .body
    .first()
    .map(|node| node.span().start)
    .unwrap_or(program.span.start)
}

impl NoNodeGlobalsHandler {
  fn fix_change(
    &self,
    ctx: &mut Context,
    span: Span,
    fix_kind: FixKind,
  ) -> LintFixChange {
    // If the fix is an import, we want to insert it after the last import
    // statement. If there are no import statements, we want to insert it at
    // the beginning of the file (but after any header comments).
    let (fix_range, add_newline) = if matches!(fix_kind, FixKind::Import { .. })
    {
      if let Some(import_span) = self.most_recent_import_span {
        (
          Span::new(import_span.end, import_span.end),
          AddNewline::Leading,
        )
      } else {
        let code_start = program_code_start(ctx.program());
        (Span::new(code_start, code_start), AddNewline::Trailing)
      }
    } else {
      (span, AddNewline::None)
    };
    LintFixChange {
      new_text: fix_kind.to_text(add_newline),
      range: fix_range,
    }
  }
  fn add_diagnostic(
    &mut self,
    ctx: &mut Context,
    span: Span,
    fix_kind: FixKind,
  ) {
    let change = self.fix_change(ctx, span, fix_kind);

    ctx.add_diagnostic_with_fixes(
      span,
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

impl Handler<'_> for NoNodeGlobalsHandler {
  fn identifier_reference(
    &mut self,
    id: &IdentifierReference,
    ctx: &mut Context,
  ) {
    let name = id.name.as_str();
    if !NODE_GLOBALS.contains_key(name) {
      return;
    }
    if ctx.scope().var_by_name(name).is_none() {
      self.add_diagnostic(ctx, id.span, NODE_GLOBALS[name]);
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
