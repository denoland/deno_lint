#!/usr/bin/env -S deno run --allow-write=. --allow-read=.
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

import { cyan, green, red } from "https://deno.land/std@0.106.0/fmt/colors.ts";
import { fromFileUrl, join } from "https://deno.land/std@0.106.0/path/mod.ts";

if (import.meta.main) {
  if (Deno.args.length !== 1) {
    console.error(red("ERROR"), "Lint name is not specified.");
    console.error("Usage: ./tools/scaffold.ts <new lint name>");
    Deno.exit(1);
  }

  const { snake, kebab, pascal } = convert(Deno.args[0]);

  const thisPath = fromFileUrl(import.meta.url);
  const mdPath = join(thisPath, "../../docs/rules", `${snake}.md`);
  const rsPath = join(thisPath, "../../src/rules", `${snake}.rs`);
  const rulesRsPath = join(thisPath, "../../src/rules.rs");

  const md = genMarkdownContent(kebab);
  const rs = genRustContent(new Date(), pascal, kebab, snake);
  const pubmod = genPubMod(await Deno.readTextFile(rulesRsPath), snake);

  await Promise.all([
    Deno.writeTextFile(rsPath, rs),
    Deno.writeTextFile(mdPath, md),
    Deno.writeTextFile(rulesRsPath, pubmod),
  ]);

  console.log(green("SUCCESS"), `finished to scaffold for ${cyan(kebab)}!`);
  console.log(
    `Next, open ${cyan(`docs/rules/${snake}.md`)} and ${
      cyan(
        `src/rules/${snake}.rs`,
      )
    } in your editor and implement the rule.`,
  );
  console.log(
    `Also, don't forget to manually add a new lint rule to ${
      cyan(
        "get_all_rules",
      )
    } function in ${
      cyan(
        "src/rules.rs",
      )
    } so that the rule will get to be run actually.`,
  );
}

export function convert(input: string): {
  snake: string;
  kebab: string;
  pascal: string;
} {
  const snake = input.replaceAll("-", "_");
  const kebab = snake.replaceAll("_", "-");
  const pascal = snake
    .replace(/^(\w)/, (_match, firstChar) => firstChar.toUpperCase())
    .replace(
      /_(\w)/g,
      (_match, afterUnderscore) => afterUnderscore.toUpperCase(),
    );
  return {
    snake,
    kebab,
    pascal,
  };
}

export function genMarkdownContent(kebabCasedLintName: string): string {
  return `[Summary of ${kebabCasedLintName} rule]

[Detail description of what this lint rule attempts to detect and/or why it's
considered to be a warning]

### Invalid:

\`\`\`typescript
// provide examples that trigger ${kebabCasedLintName}
\`\`\`

### Valid:

\`\`\`typescript
// provide examples that don't trigger ${kebabCasedLintName}
\`\`\`
`;
}

export function genRustContent(
  now: Date,
  pascalCasedLintName: string,
  kebabCasedLintName: string,
  snakeCasedLintName: string,
): string {
  return `// Copyright 2020-${now.getFullYear()} the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::SourceRanged;
use deno_ast::view as ast_view;
use std::sync::Arc;

#[derive(Debug)]
pub struct ${pascalCasedLintName};

const CODE: &str = "${kebabCasedLintName}";
const MESSAGE: &str = "";
const HINT: &str = "";

impl LintRule for ${pascalCasedLintName} {
  fn new() -> Arc<Self> {
    Arc::new(${pascalCasedLintName})
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    ${pascalCasedLintName}Handler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/${snakeCasedLintName}.md")
  }
}

struct ${pascalCasedLintName}Handler;

impl Handler for ${pascalCasedLintName}Handler {
  // implement some methods to achieve the goal of this lint

  // This is an example
  fn with_stmt(&mut self, with_stmt: &ast_view::WithStmt, ctx: &mut Context) {
    ctx.add_diagnostic_with_hint(with_stmt.range(), CODE, MESSAGE, HINT);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn ${snakeCasedLintName}_valid() {
    assert_lint_ok! {
      ${pascalCasedLintName},
      r#"// put a valid case here"#,
    };
  }

  #[test]
  fn ${snakeCasedLintName}_invalid() {
    assert_lint_err! {
      ${pascalCasedLintName},
      MESSAGE,
      HINT,
      r#"
// put a TypeScript/JavaScript snippet that is expected to trigger this lint
      "#: [
        {
          line: 0,
          col: 0,
        },
      ],
    };
  }
}
`;
}

export function genPubMod(orig: string, snakeCasedLintName: string): string {
  const i = orig.indexOf("pub mod");
  return `${orig.slice(0, i)}pub mod ${snakeCasedLintName};\n${orig.slice(i)}`;
}
