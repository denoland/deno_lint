use super::LintRule;
use crate::linter::Context;
use derive_more::Display;
use swc_common::{comments::Comment, Span, Spanned, DUMMY_SP};
use swc_ecmascript::{
  ast::*,
  visit::{noop_visit_type, Node, Visit, VisitWith},
};

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
  fn new() -> Box<Self> {
    Box::new(NoFallthrough)
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
    let mut visitor = NoFallthroughVisitor { context };
    visitor.visit_program(program, program);
  }

  fn docs(&self) -> &'static str {
    r#"Disallows the implicit fallthrough of case statements

Case statements without a `break` will execute their body and then fallthrough
to the next case or default block and execute this block as well.  While this
is sometimes intentional, many times the developer has forgotten to add a break
statement, intending only for a single case statement to be executed.  This
rule enforces that you either end each case statement with a break statement or
an explicit comment that fallthrough was intentional.  The fallthrough comment
must contain one of `fallthrough`, `falls through` or `fall through`.
    
### Invalid:
```typescript
switch(myVar) {
  case 1:
    console.log('1');

  case 2:
    console.log('2');
}
// If myVar = 1, outputs both `1` and `2`.  Was this intentional?
```

### Valid:
```typescript
switch(myVar) {
  case 1:
    console.log('1');
    break;

  case 2:
    console.log('2');
    break;
}
// If myVar = 1, outputs only `1`

switch(myVar) {
  case 1:
    console.log('1');
    /* falls through */

  case 2:
    console.log('2');
}
// If myVar = 1, intentionally outputs both `1` and `2`
```
"#
  }
}

struct NoFallthroughVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> Visit for NoFallthroughVisitor<'c> {
  noop_visit_type!();

  fn visit_switch_cases(&mut self, cases: &[SwitchCase], parent: &dyn Node) {
    let mut should_emit_err = false;
    let mut prev_span = DUMMY_SP;

    'cases: for (case_idx, case) in cases.iter().enumerate() {
      case.visit_with(parent, self);

      if should_emit_err {
        let mut emit = true;
        if let Some(comments) = self.context.leading_comments.get(&case.span.lo)
        {
          if allow_fall_through(&comments) {
            emit = false;
          }
        }
        if emit {
          self.context.add_diagnostic_with_hint(
            prev_span,
            CODE,
            NoFallthroughMessage::Unexpected,
            NoFallthroughHint::BreakOrComment,
          );
        }
      }
      should_emit_err = true;
      let mut stops_exec = false;

      // Handle return / throw / break / continue
      for (idx, stmt) in case.cons.iter().enumerate() {
        let last = idx + 1 == case.cons.len();
        let metadata = self.context.control_flow.meta(stmt.span().lo);
        stops_exec |= metadata.map(|v| v.stops_execution()).unwrap_or(false);
        if stops_exec {
          should_emit_err = false;
        }

        if last {
          if let Some(comments) =
            self.context.trailing_comments.get(&stmt.span().hi)
          {
            if allow_fall_through(&comments) {
              should_emit_err = false;
              // User comment beats everything
              prev_span = case.span;
              continue 'cases;
            }
          }
        }
      }

      let empty = case.cons.is_empty()
        || match &case.cons[0] {
          Stmt::Block(b) => b.stmts.is_empty(),
          _ => false,
        };

      if case_idx + 1 < cases.len() && empty {
        let span = Span {
          lo: case.span.lo(),
          hi: cases[case_idx + 1].span.lo(),
          ctxt: case.span.ctxt,
        };
        let span_lines = self.context.source_map.span_to_lines(span).unwrap();
        // When the case body contains only new lines `case.cons` will be empty.
        // This means there are no statements detected so we must detect case
        // bodies made up of only new lines by counting the total amount of new lines.
        // If there's more than 2 new lines and `case.cons` is empty this indicates the case body only contains new lines.
        should_emit_err = span_lines.lines.len() > 2;
      }

      prev_span = case.span;
    }
  }
}

fn allow_fall_through(comments: &[Comment]) -> bool {
  for comment in comments {
    let l = comment.text.to_ascii_lowercase();
    if l.contains("fallthrough")
      || l.contains("falls through")
      || l.contains("fall through")
    {
      return true;
    }
  }
  false
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
      "switch(foo) { case 0:\n\n default: b() }": [
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
      "switch(foo) { case 0:\n // comment\n default: b() }": [
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
