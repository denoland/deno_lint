import {
  convert,
  genMarkdownContent,
  genPubMod,
  genRustContent,
} from "../scaffold.ts";
import { assertEquals } from "https://deno.land/std@0.106.0/testing/asserts.ts";

Deno.test(
  "the conversion of input rule name into snake_case, kebab-case, and PascalCase",
  () => {
    const tests = [
      {
        input: "foo",
        expected: {
          snake: "foo",
          kebab: "foo",
          pascal: "Foo",
        },
      },
      {
        input: "foo-bar",
        expected: {
          snake: "foo_bar",
          kebab: "foo-bar",
          pascal: "FooBar",
        },
      },
      {
        input: "foo_bar",
        expected: {
          snake: "foo_bar",
          kebab: "foo-bar",
          pascal: "FooBar",
        },
      },
      {
        input: "foo-bar-baz",
        expected: {
          snake: "foo_bar_baz",
          kebab: "foo-bar-baz",
          pascal: "FooBarBaz",
        },
      },
      {
        input: "foo_bar-baz",
        expected: {
          snake: "foo_bar_baz",
          kebab: "foo-bar-baz",
          pascal: "FooBarBaz",
        },
      },
    ];

    for (const test of tests) {
      const got = convert(test.input);
      assertEquals(got.snake, test.expected.snake);
      assertEquals(got.kebab, test.expected.kebab);
      assertEquals(got.pascal, test.expected.pascal);
    }
  },
);

Deno.test("the content of .md", () => {
  const actual = genMarkdownContent("foo-bar-baz");
  const expected = `[Summary of foo-bar-baz rule]

[Detail description of what this lint rule attempts to detect and/or why it's
considered to be a warning]

### Invalid:

\`\`\`typescript
// provide examples that trigger foo-bar-baz
\`\`\`

### Valid:

\`\`\`typescript
// provide examples that don't trigger foo-bar-baz
\`\`\`
`;
  assertEquals(expected, actual);
});

Deno.test("the content of .rs", () => {
  const now = new Date("2022-08-10T14:48:00");
  const actual = genRustContent(now, "FooBarBaz", "foo-bar-baz", "foo_bar_baz");
  const expected =
    `// Copyright 2020-2022 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::{Program, ProgramRef};
use deno_ast::SourceRanged;
use deno_ast::view as ast_view;
use std::sync::Arc;

#[derive(Debug)]
pub struct FooBarBaz;

const CODE: &str = "foo-bar-baz";
const MESSAGE: &str = "";
const HINT: &str = "";

impl LintRule for FooBarBaz {
  fn new() -> Arc<Self> {
    Arc::new(FooBarBaz)
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
    FooBarBazHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/foo_bar_baz.md")
  }
}

struct FooBarBazHandler;

impl Handler for FooBarBazHandler {
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
  fn foo_bar_baz_valid() {
    assert_lint_ok! {
      FooBarBaz,
      r#"// put a valid case here"#,
    };
  }

  #[test]
  fn foo_bar_baz_invalid() {
    assert_lint_err! {
      FooBarBaz,
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
  assertEquals(expected, actual);
});

Deno.test("the updated content of src/rules.rs", () => {
  const original =
    `// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::context::Context;
use crate::Program;
use crate::ProgramRef;
use std::collections::HashSet;
use std::sync::Arc;

pub mod adjacent_overload_signatures;
pub mod ban_ts_comment;
pub mod ban_types;
pub mod ban_unknown_rule_code;
pub mod ban_untagged_ignore;
pub mod ban_untagged_todo;
pub mod ban_unused_ignore;
pub mod camelcase;
pub mod constructor_super;
pub mod default_param_last;

pub trait LintRule {}

pub fn get_all_rules() -> Vec<Arc<dyn LintRule>> {
  vec![]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn recommended_rules_sorted_alphabetically() {}
}
`;
  const actual = genPubMod(original, "foo_bar_baz");
  const expected =
    `// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use crate::context::Context;
use crate::Program;
use crate::ProgramRef;
use std::collections::HashSet;
use std::sync::Arc;

pub mod foo_bar_baz;
pub mod adjacent_overload_signatures;
pub mod ban_ts_comment;
pub mod ban_types;
pub mod ban_unknown_rule_code;
pub mod ban_untagged_ignore;
pub mod ban_untagged_todo;
pub mod ban_unused_ignore;
pub mod camelcase;
pub mod constructor_super;
pub mod default_param_last;

pub trait LintRule {}

pub fn get_all_rules() -> Vec<Arc<dyn LintRule>> {
  vec![]
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn recommended_rules_sorted_alphabetically() {}
}
`;
  assertEquals(expected, actual);
});
