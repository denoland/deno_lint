// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_ecma_ast::Regex;
use swc_ecma_visit::Node;
use swc_ecma_visit::Visit;

pub struct NoEmptyCharacterClass;

impl LintRule for NoEmptyCharacterClass {
  fn new() -> Box<Self> {
    Box::new(NoEmptyCharacterClass)
  }

  fn code(&self) -> &'static str {
    "no-empty-character-class"
  }

  fn lint_module(&self, context: Context, module: swc_ecma_ast::Module) {
    let mut visitor = NoEmptyCharacterClassVisitor::new(context);
    visitor.visit_module(&module, &module);
  }
}

pub struct NoEmptyCharacterClassVisitor {
  context: Context,
}

impl NoEmptyCharacterClassVisitor {
  pub fn new(context: Context) -> Self {
    Self { context }
  }
}

impl Visit for NoEmptyCharacterClassVisitor {
  fn visit_regex(&mut self, regex: &Regex, _parent: &dyn Node) {
    let raw_regex = self
      .context
      .source_map
      .span_to_snippet(regex.span)
      .expect("error in loading snippet");

    lazy_static! {
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
      static ref RULE_REGEX: regex::Regex = regex::Regex::new(
        r"(?u)^/([^\\\[]|\\.|\[([^\\\]]|\\.)+\])*/[gimuys]*$"
      )
      .unwrap();
    }
    if !RULE_REGEX.is_match(&raw_regex) {
      self.context.add_diagnostic(
        regex.span,
        "no-empty-character-class",
        "empty character class in RegExp is not allowed",
      );
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_empty_character_class() {
    assert_lint_err::<NoEmptyCharacterClass>(r#"const foo = /^abc[]/;"#, 12);
    assert_lint_err::<NoEmptyCharacterClass>(r#"const foo = /foo[]bar/;"#, 12);
    assert_lint_err::<NoEmptyCharacterClass>(r#"const foo = /[]]/;"#, 12);
    assert_lint_err::<NoEmptyCharacterClass>(r#"const foo = /\[[]/;"#, 12);
    assert_lint_err::<NoEmptyCharacterClass>(
      r#"const foo = /\\[\\[\\]a-z[]/;"#,
      12,
    );
  }

  #[test]
  fn no_empty_character_class_match() {
    assert_lint_err::<NoEmptyCharacterClass>(r#"/^abc[]/.test("abcdefg");"#, 0);
    assert_lint_err::<NoEmptyCharacterClass>(
      r#"if (foo.match(/^abc[]/)) {}"#,
      14,
    );
  }

  #[test]
  fn no_empty_character_class_test() {
    assert_lint_err::<NoEmptyCharacterClass>(
      r#""abcdefg".match(/^abc[]/);"#,
      16,
    );
    assert_lint_err::<NoEmptyCharacterClass>(
      r#"if (/^abc[]/.test(foo)) {}"#,
      4,
    );
  }

  #[test]
  fn no_empty_character_class_valid() {
    assert_lint_ok::<NoEmptyCharacterClass>(
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
    );
  }
}
