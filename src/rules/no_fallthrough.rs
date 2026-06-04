// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{Comment, Program, Statement, SwitchStatement};
use deno_ast::oxc::span::GetSpan;
use derive_more::Display;

#[derive(Debug)]
pub struct NoFallthrough;

const CODE: &str = "no-fallthrough";

#[derive(Display)]
enum NoFallthroughMessage {
  #[display(fmt = "Fallthrough is not allowed")]
  Unexpected,
}

#[derive(Display)]
enum NoFallthroughHint {
  #[display(
    fmt = "Add `break` or comment `/* falls through */` to your case statement"
  )]
  BreakOrComment,
}

impl LintRule for NoFallthrough {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = NoFallthroughHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct NoFallthroughHandler;

impl Handler<'_> for NoFallthroughHandler {
  fn switch_statement(
    &mut self,
    switch_stmt: &SwitchStatement,
    context: &mut Context,
  ) {
    let cases = &switch_stmt.cases;
    let mut should_emit_err = false;
    let mut prev_span = None;

    for case in cases.iter() {
      if should_emit_err {
        // Check for leading comments on this case that allow fallthrough
        let comments: Vec<&Comment> = context
          .all_comments()
          .filter(|c| c.span.end <= case.span.start)
          .collect();
        let leading_allows = comments.iter().rev().any(|comment| {
          let text = context.comment_text(comment);
          allow_fall_through_text(text)
        });
        if !leading_allows {
          if let Some(prev) = prev_span.take() {
            context.add_diagnostic_with_hint(
              prev,
              CODE,
              NoFallthroughMessage::Unexpected,
              NoFallthroughHint::BreakOrComment,
            );
          }
        }
      }
      should_emit_err = true;
      let mut stops_exec = false;

      // Handle return / throw / break / continue
      for (idx, stmt) in case.consequent.iter().enumerate() {
        let last = idx + 1 == case.consequent.len();
        let metadata = context.control_flow().meta(stmt.span().start);
        stops_exec |= metadata.map(|v| v.stops_execution()).unwrap_or(false);
        // break/continue statements stop execution for fallthrough purposes
        // even if the control flow analysis doesn't mark them (e.g. `continue`
        // inside a switch that targets the outer loop).
        if matches!(
          stmt,
          Statement::BreakStatement(_) | Statement::ContinueStatement(_)
        ) {
          stops_exec = true;
        }
        if stops_exec {
          should_emit_err = false;
        }

        if last {
          // Check for trailing comments after this statement that allow fallthrough
          let trailing_allows = context.all_comments().any(|comment| {
            comment.span.start >= stmt.span().end
              && comment.span.end <= case.span.end
              && allow_fall_through_text(context.comment_text(comment))
          });
          if trailing_allows {
            should_emit_err = false;
            continue;
          }
        }
      }

      let empty = case.consequent.is_empty()
        || matches!(
          case.consequent.as_slice(),
          [Statement::BlockStatement(b)] if b.body.is_empty()
        );

      if empty {
        should_emit_err = false;
      }

      prev_span = Some(case.span);
    }
  }
}

fn allow_fall_through_text(text: &str) -> bool {
  let l = text.to_ascii_lowercase();
  l.contains("fallthrough")
    || l.contains("falls through")
    || l.contains("fall through")
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_fallthrough_valid() {
    assert_lint_ok! {
      NoFallthrough,
      "switch(foo) { case 0: a(); /* falls through */ case 1: b(); }",
      "switch(foo) { case 0: a()\n /* falls through */ case 1: b(); }",
      "switch(foo) { case 0: a(); /* fall through */ case 1: b(); }",
      "switch(foo) { case 0: a(); /* fallthrough */ case 1: b(); }",
      "switch(foo) { case 0: a(); /* FALLS THROUGH */ case 1: b(); }",
      "function foo() { switch(foo) { case 0: a(); return; case 1: b(); }; }",
      "switch(foo) { case 0: a(); throw 'foo'; case 1: b(); }",
      "while (a) { switch(foo) { case 0: a(); continue; case 1: b(); } }",
      "switch(foo) { case 0: a(); break; case 1: b(); }",
      "switch(foo) { case 0: case 1: a(); break; case 2: b(); }",
      "switch(foo) { case 0: case 1: break; case 2: b(); }",
      "switch(foo) { case 0: case 1: break; default: b(); }",
      "switch(foo) { case 0: case 1: a(); }",
      "switch(foo) { case 0: case 1: a(); break; }",
      "switch(foo) { case 0: case 1: break; }",
      "switch(foo) { case 0:\n case 1: break; }",
      "switch(foo) { case 0: // comment\n case 1: break; }",
      "function foo() { switch(foo) { case 0: case 1: return; } }",
      "function foo() { switch(foo) { case 0: {return;}\n case 1: {return;} } }",
      "switch(foo) { case 0: case 1: {break;} }",
      "switch(foo) { }",
      "switch(foo) { case 0: switch(bar) { case 2: break; } /* falls through */ case 1: break; }",
      "function foo() { switch(foo) { case 1: return a; a++; }}",
      "switch (foo) { case 0: a(); /* falls through */ default:  b(); /* comment */ }",
      "switch (foo) { case 0: a(); /* falls through */ default: /* comment */ b(); }",
      "switch (foo) { case 0: if (a) { break; } else { throw 0; } default: b(); }",
      "switch (foo) { case 0: try { break; } finally {} default: b(); }",
      "switch (foo) { case 0: try {} finally { break; } default: b(); }",
      "switch (foo) { case 0: try { throw 0; } catch (err) { break; } default: b(); }",
      "switch (foo) { case 0: do { throw 0; } while(a); default: b(); }",
      "switch('test') { case 'symbol':\n case 'function': default: b(); }",
      "switch('test') { case 'symbol':\n case 'function':\n default: b(); }",
      "switch('test') { case 'symbol': case 'function': default: b(); }",
      "switch(foo) { case 1:\n\n default: a(); }",
      "switch(foo) { case 1:\n// comment\n default: a(); }",

      // https://github.com/denoland/deno_lint/issues/746
      r#"
switch(someValue) {
  case 0: {
    // nothing to do for this value
  } break;
  case 1:
    break;
}
      "#,
      r#"
switch(someValue) {
  case 0: {
    // nothing to do for this value
  } break;
  default:
    console.log(42);
}
      "#,
    };
  }

  #[test]
  fn no_fallthrough_invalid() {
    assert_lint_err! {
      NoFallthrough,
      "switch(foo) { case 0: a();\ncase 1: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ],
      "switch(foo) { case 0: a();\ndefault: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ],
      "switch(foo) { case 0: a(); default: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ],
      "switch(foo) { case 0: if (a) { break; } default: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ],
      "switch(foo) { case 0: try { throw 0; } catch (err) {} default: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ],
      "switch(foo) { case 0: while (a) { break; } default: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ],
      "switch(foo) { case 0:\n\n b()\n default: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ],
      "switch(foo) { case 0: a(); /* falling through */ default: b() }": [
        {
          col: 14,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ]
    };
  }

  #[test]
  #[ignore = "It ends with break statement"]
  fn no_fallthrough_invalid_2() {
    assert_lint_err! {
      NoFallthrough,
      "switch(foo) { case 0: do { break; } while (a); default: b() }": [
        {
          col: 47,
          message: NoFallthroughMessage::Unexpected,
          hint: NoFallthroughHint::BreakOrComment,
        }
      ]
    };
  }
}
