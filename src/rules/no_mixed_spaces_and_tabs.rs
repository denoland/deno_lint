// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use swc_common::BytePos;
use swc_common::Span;
use swc_common::SyntaxContext;
use swc_common::Spanned;
use regex::Regex;
use std::sync::Arc;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::ast::Lit;
use swc_ecmascript::ast::Tpl;

lazy_static! {
  static ref RE: Regex = Regex::new("^([\t ]*(\t | \t))").unwrap();
}

pub struct NoMixedSpacesAndTabs;

impl LintRule for NoMixedSpacesAndTabs {
  fn new() -> Box<Self> {
    Box::new(NoMixedSpacesAndTabs)
  }

  fn code(&self) -> &'static str {
    "no-mixed-spaces-and-tabs"
  }

  fn lint_module(
    &self,
    context: Arc<Context>,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = NoMixedSpacesAndTabsVisitor::default();
    visitor.visit_module(module, module);

    let file_and_lines = context.source_map.span_to_lines(module.span).unwrap();
    let file = file_and_lines.file;

    let mut excluded_ranges = visitor.ranges;

    eprintln!("comments! {:#?}", context.leading_comments);
    eprintln!("tcomments! {:#?}", context.trailing_comments);

    context.leading_comments.values().for_each(|comments| {
      eprintln!("comments! {:#?}", comments);
      for comment in comments {
        let lines = context.source_map.span_to_lines(comment.span).unwrap().lines;
        for line in lines.iter().skip(1) {
          let (lo, hi) = file.line_bounds(line.line_index as usize);
          excluded_ranges.push(Span::new(
            lo,
            hi,
            SyntaxContext::empty(),
          ));
        }
      }
    });
    context.trailing_comments.values().for_each(|comments| {
      eprintln!("comments! {:#?}", comments);
      for comment in comments {
        let lines = context.source_map.span_to_lines(comment.span).unwrap().lines;
        eprintln!("lines {:#?}", lines);
        for line in lines.iter().skip(1) {
          let (lo, hi) = file.line_bounds(line.line_index as usize);
          eprintln!("hi lo {:#?} {:#?}", hi, lo);
          excluded_ranges.push(Span::new(
            lo,
            hi,
            SyntaxContext::empty(),
          ));
        }
      }
    });


    let excluded_ranges = excluded_ranges.iter();
    eprintln!("excluded ranges {:#?}", excluded_ranges);
    // let lines = file_and_lines.lines;
    for line_index in 0..file.count_lines() {
      let line = file.get_line(line_index).unwrap();
      let (byte_pos, _hi) = file.line_bounds(line_index);
      // let byte_pos = BytePos(line_info.line_index as u32);
      // let source_file_and_index = context
      //   .source_map
      //   .lookup_line(byte_pos)
      //   .unwrap();
      // let source_code = source_file_and_index
      //   .sf
      //   .get_line(line_info.line_index)
      //   .unwrap();
      let whitespace_matches = RE.find_iter(&line);
      for whitespace_match in whitespace_matches {
        eprintln!("whitespace match {:#?}", whitespace_match);
        let range = whitespace_match.range();
        let span = Span::new(
          byte_pos + BytePos(range.start as u32),
          byte_pos + BytePos(range.end as u32),
          SyntaxContext::empty(),
        );
        eprintln!("span {:#?}", span);
        let is_excluded = excluded_ranges.clone().any(|range| {
          eprintln!("{:#?} {:#?} {:#?} {:#?}", range.lo, span.lo, span.hi, range.hi);
          range.lo <= span.lo && span.hi <= range.hi
        });
        if !is_excluded {
          context.add_diagnostic(
            span,
            "no-mixed-spaces-and-tabs",
            "Mixed spaces and tabs are not allowed.",
          );
        }
      }
    }
  }
}

struct NoMixedSpacesAndTabsVisitor {
  ranges: Vec<Span>,
}

impl NoMixedSpacesAndTabsVisitor {
  pub fn default() -> Self {
    Self { ranges: vec![] }
  }
}

impl Visit for NoMixedSpacesAndTabsVisitor {
  fn visit_lit(&mut self, lit: &Lit, _parent: &dyn Node) {
    self.ranges.push(lit.span());
  }

  fn visit_tpl(&mut self, tpl: &Tpl, _parent: &dyn Node) {
    self.ranges.push(tpl.span);
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::*;

  #[test]
  fn no_mixed_spaces_and_tabs_valid() {
    assert_lint_ok_n::<NoMixedSpacesAndTabs>(vec![
      "\tvar x = 5;",
      "    var x = 5;",
      "\t/*\n\t * Hello\n\t */",
      "// foo\n\t/**\n\t * Hello\n\t */",
      "/*\n\n \t \n*/",
      "/*\t */ //",
      "/*\n \t*/ //",
      "/*\n\t *//*\n \t*/",
      "// \t",
      "/*\n*/\t ",
      "/* \t\n\t \n \t\n\t */ \t",
      "\tvar x = 5,\n\t    y = 2;",
      "/*\n\t */`\n\t   `;",
      "/*\n\t */var a = `\n\t   `, b = `\n\t   `/*\t \n\t \n*/;",
      "/*\t `template inside comment` */",
      "var foo = `\t /* comment inside template\t */`;",
      "`\n\t   `;",
      "`\n\t   \n`;",
      "`\t   `;",
      "const foo = `${console}\n\t foo`;",
      "`\t   `;`   \t`",
      "`foo${ 5 }\t    `;",
      "\tvar x = 5,\n\t    y = 2;",
      "' \t\\\n\t multiline string';",
      "'\t \\\n \tmultiline string';",
    ]);
  }

  #[test]
  fn no_mixed_spaces_and_tabs_invalid() {
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "function add(x, y) {\n\t return x + y;\n}",
    //   2, 0
    // );
    assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
      "\t ;\n/*\n\t * Hello\n\t */",
      1, 0
    );
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   " \t/* comment */",
    //   1, 0
    // );
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "\t // comment",
    //   1, 0
    // );
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "\t var a /* comment */ = 1;",
    //   1, 0
    // );
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   " \tvar b = 1; // comment",
    //   1, 0
    // );
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "/**/\n \t/*\n \t*/",
    //   2, 0
    // );
    // assert_lint_err_on_line_n::<NoMixedSpacesAndTabs>(
    //   "\t var x = 5, y = 2, z = 5;\n\n\t \tvar j =\t x + y;\nz *= j;",
    //   vec![(2, 0), (3, 0)]
    // );

    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "\tvar x = 5,\n  \t  y = 2;",
    //   2, 0
    // );
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "`foo${\n \t  5 }bar`;",
    //   2, 2
    // );
    
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "  \t'';",
    //   1, 0
    // );
    // assert_lint_err_on_line::<NoMixedSpacesAndTabs>(
    //   "''\n\t ",
    //   2, 0
    // );
  }
}
