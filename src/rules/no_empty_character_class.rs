// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use once_cell::sync::Lazy;
use swc_ecmascript::ast::Regex;
use swc_ecmascript::visit::noop_visit_type;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct NoEmptyCharacterClass;

const CODE: &str = "no-empty-character-class";
const MESSAGE: &str = "empty character class in RegExp is not allowed";

impl LintRule for NoEmptyCharacterClass {
  fn new() -> Box<Self> {
    Box::new(NoEmptyCharacterClass)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program(
    &self,
    context: &mut Context,
    program: &swc_ecmascript::ast::Program,
  ) {
    let mut visitor = NoEmptyCharacterClassVisitor::new(context);
    visitor.visit_program(program, program);
  }
}

struct NoEmptyCharacterClassVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> NoEmptyCharacterClassVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for NoEmptyCharacterClassVisitor<'c> {
  noop_visit_type!();

  fn visit_regex(&mut self, regex: &Regex, _parent: &dyn Node) {
    let raw_regex = self
      .context
      .source_map
      .span_to_snippet(regex.span)
      .expect("error in loading snippet");

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
       * 4. `[gimuy]*`: optional regexp flags
       * 5. `$`: fix the match at the end of the string
       */
      regex::Regex::new(r"(?u)^/([^\\\[]|\\.|\[([^\\\]]|\\.)+\])*/[gimuys]*$")
        .unwrap()
    });

    if !RULE_REGEX.is_match(&raw_regex) {
      self.context.add_diagnostic(regex.span, CODE, MESSAGE);
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
    "#,
    };
  }

  #[test]
  fn no_empty_character_invalid() {
    assert_lint_err! {
      NoEmptyCharacterClass,
      r#"const foo = /^abc[]/;"#: [{
        col: 12,
        message: MESSAGE,
      }],
      r#"const foo = /foo[]bar/;"#: [{
        col: 12,
        message: MESSAGE,
      }],
      r#"const foo = /[]]/;"#: [{
        col: 12,
        message: MESSAGE,
      }],
      r#"const foo = /\[[]/;"#: [{
        col: 12,
        message: MESSAGE,
      }],
      r#"const foo = /\\[\\[\\]a-z[]/;"#: [{
        col: 12,
        message: MESSAGE,
      }],
      r#"/^abc[]/.test("abcdefg");"#: [{
        col: 0,
        message: MESSAGE,
      }],
      r#"if (foo.match(/^abc[]/)) {}"#: [{
        col: 14,
        message: MESSAGE,
      }],
      r#""abcdefg".match(/^abc[]/);"#: [{
        col: 16,
        message: MESSAGE,
      }],
      r#"if (/^abc[]/.test(foo)) {}"#: [{
        col: 4,
        message: MESSAGE,
      }],
    }
  }
}
