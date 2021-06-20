// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef, DUMMY_NODE};
use derive_more::Display;
use once_cell::sync::Lazy;
use regex::Regex;
use swc_common::{BytePos, Span, Spanned, SyntaxContext};
use swc_ecmascript::ast::{Lit, Tpl};
use swc_ecmascript::visit::{Node, VisitAll, VisitAllWith};

static RE: Lazy<Regex> =
  Lazy::new(|| Regex::new("^([\t ]*(\t | \t))").unwrap());

const CODE: &str = "no-mixed-spaces-and-tabs";

#[derive(Display)]
enum NoMixedSpacesAndTabsMessage {
  #[display(fmt = "Mixed spaces and tabs are not allowed.")]
  NotAllowed,
}

pub struct NoMixedSpacesAndTabs;

impl LintRule for NoMixedSpacesAndTabs {
  fn new() -> Box<Self> {
    Box::new(NoMixedSpacesAndTabs)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program<'view>(
    &self,
    context: &mut Context<'view>,
    program: ProgramRef<'view>,
  ) {
    let mut visitor = NoMixedSpacesAndTabsVisitor::default();
    match program {
      ProgramRef::Module(ref m) => m.visit_all_with(&DUMMY_NODE, &mut visitor),
      ProgramRef::Script(ref s) => s.visit_all_with(&DUMMY_NODE, &mut visitor),
    }

    let span = match program {
      ProgramRef::Module(ref m) => m.span,
      ProgramRef::Script(ref s) => s.span,
    };
    let file_and_lines = context.source_map().span_to_lines(span).unwrap();
    let file = file_and_lines.file;

    let mut excluded_ranges = visitor.ranges;

    for comment in context.all_comments() {
      let lines = context
        .source_map()
        .span_to_lines(comment.span)
        .unwrap()
        .lines;
      for line in lines.iter().skip(1) {
        let (lo, hi) = file.line_bounds(line.line_index as usize);
        excluded_ranges.push(Span::new(lo, hi, SyntaxContext::empty()));
      }
    }

    let excluded_ranges = excluded_ranges.iter();
    for line_index in 0..file.count_lines() {
      let line = file.get_line(line_index).unwrap();
      let (byte_pos, _hi) = file.line_bounds(line_index);
      let whitespace_matches = RE.find_iter(&line);
      for whitespace_match in whitespace_matches {
        let range = whitespace_match.range();
        let span = Span::new(
          byte_pos + BytePos(range.start as u32),
          byte_pos + BytePos(range.end as u32),
          SyntaxContext::empty(),
        );
        let is_excluded =
          excluded_ranges.clone().any(|range| range.contains(span));
        if !is_excluded {
          context.add_diagnostic(
            span,
            CODE,
            NoMixedSpacesAndTabsMessage::NotAllowed,
          );
        }
      }
    }
  }

  fn docs(&self) -> &'static str {
    r#"Disallows mixed spaces and tabs for indentation.

Most code conventions require either tabs or spaces be used for indentation. Therefore, if a line of code is indented with both tabs and spaces, it's most likely a mistake of a developer.

### Invalid:

```typescript
function add(x: number, y: number) {
	  return x + y; // indented with a tab + two spaces
}
```

```typescript
	let x = 5, // indented with a tab
	    y = 7; // indented with a tab + four spaces
```

### Valid:

```typescript
function add(x: number, y: number) {
  return x + y;
}
```"#
  }
}

#[derive(Default)]
struct NoMixedSpacesAndTabsVisitor {
  ranges: Vec<Span>,
}

impl VisitAll for NoMixedSpacesAndTabsVisitor {
  fn visit_lit(&mut self, lit: &Lit, _: &dyn Node) {
    self.ranges.push(lit.span());
  }

  fn visit_tpl(&mut self, tpl: &Tpl, _: &dyn Node) {
    self.ranges.push(tpl.span);
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_mixed_spaces_and_tabs_valid() {
    assert_lint_ok! {
      NoMixedSpacesAndTabs,
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
      "' \t\\\n\t multiline string';",
      "'\t \\\n \tmultiline string';",
    };
  }

  #[test]
  fn no_mixed_spaces_and_tabs_invalid() {
    assert_lint_err! {
      NoMixedSpacesAndTabs,
      "function add(x, y) {\n\t return x + y;\n}": [
        {
          line: 2,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      "\t ;\n/*\n\t * Hello\n\t */": [
        {
          line: 1,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      " \t/* comment */": [
        {
          line: 1,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      "\t // comment": [
        {
          line: 1,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      "\t var a /* comment */ = 1;": [
        {
          line: 1,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      " \tvar b = 1; // comment": [
        {
          line: 1,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      "/**/\n \t/*\n \t*/": [
        {
          line: 2,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      "\t var x = 5, y = 2, z = 5;\n\n\t \tvar j =\t x + y;\nz *= j;": [
        {
          line: 1,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        },
        {
          line: 3,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      "  \t'';": [
        {
          line: 1,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ],
      "''\n\t ": [
        {
          line: 2,
          col: 0,
          message: NoMixedSpacesAndTabsMessage::NotAllowed,
        }
      ]
    };
  }
}
