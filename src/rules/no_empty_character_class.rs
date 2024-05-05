// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};
use crate::Program;
use deno_ast::view::Regex;
use deno_ast::SourceRanged;
use once_cell::sync::Lazy;

#[derive(Debug)]
pub struct NoEmptyCharacterClass;

const CODE: &str = "no-empty-character-class";
const MESSAGE: &str = "empty character class in RegExp is not allowed";
const HINT: &str =
  "Remove or rework the empty character class (`[]`) in the RegExp";

impl LintRule for NoEmptyCharacterClass {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    NoEmptyCharacterClassVisitor.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_empty_character_class.md")
  }
}

struct NoEmptyCharacterClassVisitor;

impl Handler for NoEmptyCharacterClassVisitor {
  fn regex(&mut self, regex: &Regex, ctx: &mut Context) {
    let raw_regex = regex.text_fast(ctx.text_info());

    static RULE_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
      /* reference : [eslint no-empty-character-class](https://github.com/eslint/eslint/blob/master/lib/rules/no-empty-character-class.js#L13)
       * plain-English description of the following regexp:
       * 0. `^` fix the match at the beginning of the string
       * 1. `\/`: the `/` that begins the regexp
       * 2. `([^\\[]|\\.|\[([^\\\]]|\\.)+\])*`: regexp contents; 0 or more of the following
       * 2.0. `[^\\[]`: any character that's not a `\` or a `[` (anything but escape sequences and character classes)
       * 2.1. `\\.`: an escape sequence
       * 2.2. `\[([^\\\]]|\\.)+\]`: a character class that isn't empty
       * 3. `\/` the `/` that ends the regexp
       * 4. `[dgimsuvy]*`: optional regexp flags
       * 5. `$`: fix the match at the end of the string
       */
      regex::Regex::new(r"(?u)^/([^\\\[]|\\.|\[([^\\\]]|\\.)+\])*/[dgimsuvy]*$")
        .unwrap()
    });

    if !RULE_REGEX.is_match(raw_regex) {
      ctx.add_diagnostic_with_hint(regex.range(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_empty_character_class_valid() {
    assert_lint_ok! {
      NoEmptyCharacterClass,
      r#"
    const foo = /^abc[a-zA-Z]/;
    const regExp = new RegExp("^abc[]");
    const foo = /^abc/;
    const foo = /[\\[]/;
    const foo = /[\\]]/;
    const foo = /[a-zA-Z\\[]/;
    const foo = /[[]/;
    const foo = /[\\[a-z[]]/;
    const foo = /[\-\[\]\/\{\}\(\)\*\+\?\.\\^\$\|]/g;
    const foo = /\[/g;
    const foo = /\]/i;
    const foo = /\]/dgimsuvy;
    "#,
    };
  }

  #[test]
  fn no_empty_character_invalid() {
    assert_lint_err! {
      NoEmptyCharacterClass,
      r"const foo = /^abc[]/;": [{
        col: 12,
        message: MESSAGE,
        hint: HINT,
      }],
      r"const foo = /foo[]bar/;": [{
        col: 12,
        message: MESSAGE,
        hint: HINT,
      }],
      r"const foo = /[]]/;": [{
        col: 12,
        message: MESSAGE,
        hint: HINT,
      }],
      r"const foo = /\[[]/;": [{
        col: 12,
        message: MESSAGE,
        hint: HINT,
      }],
      r"const foo = /\\[\\[\\]a-z[]/;": [{
        col: 12,
        message: MESSAGE,
        hint: HINT,
      }],
      r#"/^abc[]/.test("abcdefg");"#: [{
        col: 0,
        message: MESSAGE,
        hint: HINT,
      }],
      r#"if (foo.match(/^abc[]/)) {}"#: [{
        col: 14,
        message: MESSAGE,
        hint: HINT,
      }],
      r#""abcdefg".match(/^abc[]/);"#: [{
        col: 16,
        message: MESSAGE,
        hint: HINT,
      }],
      r#"if (/^abc[]/.test(foo)) {}"#: [{
        col: 4,
        message: MESSAGE,
        hint: HINT,
      }],
    }
  }
}
