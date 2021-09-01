#!/usr/bin/env -S deno run --allow-write=. --allow-read=.
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

import { cyan, green, red } from "https://deno.land/std@0.106.0/fmt/colors.ts";
import { fromFileUrl, join } from "https://deno.land/std@0.106.0/path/mod.ts";

if (Deno.args.length !== 1) {
  console.error(red("ERROR"), "Lint name is not specified.");
  console.error("Usage: ./tools/scaffold.ts <new lint name>");
  Deno.exit(1);
}

const snakeCasedLintName = Deno.args[0].replaceAll("-", "_");
const kebabCasedLintName = snakeCasedLintName.replaceAll("_", "-");
const pascalCasedLintName = snakeCasedLintName
  .replace(/^(\w)/, (_match, firstChar) => firstChar.toUpperCase())
  .replace(
    /_(\w)/g,
    (_match, afterUnderscore) => afterUnderscore.toUpperCase(),
  );
const thisPath = fromFileUrl(import.meta.url);
const mdPath = join(thisPath, "../../docs/rules", `${snakeCasedLintName}.md`);
const rsPath = join(thisPath, "../../src/rules", `${snakeCasedLintName}.rs`);
const rulesRsPath = join(thisPath, "../../src/rules.rs");

await Promise.all([
  createMarkdown(mdPath),
  createRs(rsPath, pascalCasedLintName, kebabCasedLintName, snakeCasedLintName),
  addPubMod(rulesRsPath, snakeCasedLintName),
]);

console.log(
  green("SUCCESS"),
  `finished to scaffold for ${cyan(kebabCasedLintName)}!`,
);
console.log(
  `Next, open ${cyan(`docs/rules/${snakeCasedLintName}.md`)} and ${
    cyan(
      `src/rules/${snakeCasedLintName}.rs`,
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

async function createMarkdown(mdPath: string) {
  const md = `[Summary of this lint rule]

[Detail description of what this lint rule attempts to detect and/or why it's
considered to be a warning]

### Invalid:

\`\`\`typescript
// provide examples that trigger the lint
\`\`\`

### Valid:

\`\`\`typescript
// provide examples that don't trigger the lint
\`\`\`
`;
  await Deno.writeTextFile(mdPath, md);
}

async function createRs(
  rsPath: string,
  pascalCasedLintName: string,
  kebabCasedLintName: string,
  snakeCasedLintName: string,
) {
  const rs = `// Copyright 2020-${
    new Date().getFullYear()
  } the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};

pub struct ${pascalCasedLintName};

const CODE: &str = "${kebabCasedLintName}";
const MESSAGE: &str = "";
const HINT: &str = "";

impl LintRule for ${pascalCasedLintName} {
  fn new() -> Box<Self> {
    Box::new(${pascalCasedLintName})
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(&self, _context: &mut Context, _program: ProgramRef<'_>) {
    unreachable!();
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

  await Deno.writeTextFile(rsPath, rs);
}

async function addPubMod(rulesRsPath: string, snakeCasedLintName: string) {
  const content = await Deno.readTextFile(rulesRsPath);
  const i = content.indexOf("pub mod");
  const updated = `${
    content.slice(
      0,
      i,
    )
  }pub mod ${snakeCasedLintName};\n${content.slice(i)}`;
  await Deno.writeTextFile(rulesRsPath, updated);
}
