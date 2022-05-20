// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{ControlFlow, End, Metadata};
use crate::test_util;
use deno_ast::swc::common::BytePos;

fn analyze_flow(src: &str) -> ControlFlow {
  let parsed_source = test_util::parse(src);
  ControlFlow::analyze(parsed_source.program_ref().into())
}

macro_rules! assert_flow {
  ($flow:ident, $lo:expr, $unreachable:expr, $end:expr) => {
    assert_eq!(
      $flow.meta(BytePos($lo)).unwrap(),
      &Metadata {
        unreachable: $unreachable,
        end: $end,
      }
    );
  };
}

#[test]
fn while_1() {
  let src = r#"
function foo() {
  while (a) {
    break;
  }
  return 1;
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(flow, 30, false, Some(End::Continue)); // BlockStmt of while
  assert_flow!(flow, 49, false, Some(End::forced_return())); // return stmt
}

#[test]
fn while_2() {
  let src = r#"
function foo() {
  while (a) {
    break;
  }
  bar();
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 30, false, Some(End::Continue)); // BlockStmt of while
  assert_flow!(flow, 49, false, None); // `bar();`
}

#[test]
fn while_3() {
  let src = r#"
function foo() {
  while (a) {
    bar();
  }
  baz();
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 30, false, Some(End::Continue)); // BlockStmt of while
  assert_flow!(flow, 36, false, None); // `bar();`
  assert_flow!(flow, 49, false, None); // `baz();`
}

#[test]
fn while_4() {
  let src = r#"
function foo() {
  while (a) {
    return 1;
  }
  baz();
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`

  // BlockStmt of while
  // This block contains `return 1;` but whether entering the block depends on the specific value
  // of `a`, so we treat it as `End::Continue`.
  assert_flow!(flow, 30, false, Some(End::Continue));

  assert_flow!(flow, 36, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 52, false, None); // `baz();`
}

#[test]
fn while_5() {
  let src = r#"
function foo() {
  while (true) {
    return 1;
  }
  baz();
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`

  // BlockStmt of while
  // This block contains `return 1;` and it returns `1` _unconditionally_.
  assert_flow!(flow, 33, false, Some(End::forced_return()));

  assert_flow!(flow, 39, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 55, true, None); // `baz();`
}

// https://github.com/denoland/deno_lint/issues/674
#[test]
fn while_6() {
  let src = r#"
while (true) {
  if (x === 42) {
    break;
  }
  throw new Error("error");
}
foo();
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, None); // while stmt
  assert_flow!(flow, 14, false, Some(End::Continue)); // BlockStmt of while
  assert_flow!(flow, 18, false, Some(End::Continue)); // if stmt
  assert_flow!(flow, 32, false, Some(End::Break)); // BlockStmt of if
  assert_flow!(flow, 38, false, Some(End::Break)); // break stmt
  assert_flow!(flow, 51, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 79, false, None); // `foo();` (which is _reachable_ if `x` equals `42`)
}

#[test]
fn do_while_1() {
  let src = r#"
function foo() {
  do {
    break;
  } while (a);
  return 1;
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(flow, 23, false, Some(End::Continue)); // BlockStmt of do-while
  assert_flow!(flow, 53, false, Some(End::forced_return())); // return stmt
}

#[test]
fn do_while_2() {
  let src = r#"
function foo() {
  do {
    break;
  } while (a);
  bar();
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 23, false, Some(End::Continue)); // BlockStmt of do-while
  assert_flow!(flow, 53, false, None); // `bar();`
}

#[test]
fn do_while_3() {
  let src = r#"
function foo() {
  do {
    bar();
  } while (a);
  baz();
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 23, false, Some(End::Continue)); // BlockStmt of do-while
  assert_flow!(flow, 53, false, None); // `bar();`
}

#[test]
fn do_while_4() {
  // infinite loop
  let src = r#"
function foo() {
  do {
    bar();
  } while (true);
  return 1;
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_infinite_loop())); // BlockStmt of `foo`
  assert_flow!(flow, 23, false, Some(End::forced_infinite_loop())); // BlockStmt of do-while
  assert_flow!(
    flow,
    56,
    true,
    Some(End::Forced {
      ret: true,
      throw: false,
      infinite_loop: true
    })
  ); // return stmt
}

#[test]
fn do_while_5() {
  let src = r#"
function foo() {
  do {
    return 0;
  } while (a);
  return 1;
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(flow, 23, false, Some(End::forced_return())); // BlockStmt of do-while
  assert_flow!(flow, 56, true, Some(End::forced_return())); // return stmt
}

#[test]
fn do_while_6() {
  let src = r#"
function foo() {
  do {
    throw 0;
  } while (false);
  return 1;
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_throw())); // BlockStmt of `foo`
  assert_flow!(flow, 23, false, Some(End::forced_throw())); // BlockStmt of do-while
  assert_flow!(
    flow,
    59,
    true,
    Some(End::Forced {
      ret: true,
      throw: true,
      infinite_loop: false
    })
  ); // return stmt
}

#[test]
fn do_while_7() {
  let src = r#"
function foo() {
  do {
    throw 0;
  } while (a);
  return 1;
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_throw())); // BlockStmt of `foo`
  assert_flow!(flow, 23, false, Some(End::forced_throw())); // BlockStmt of do-while
  assert_flow!(flow, 29, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(
    flow,
    55,
    true,
    Some(End::Forced {
      ret: true,
      throw: true,
      infinite_loop: false
    })
  ); // return stmt
}

// https://github.com/denoland/deno_lint/issues/674
#[test]
fn do_while_8() {
  let src = r#"
do {
  if (x === 42) {
    break;
  }
  throw new Error("error");
} while (true);
foo();
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, None); // do-while stmt
  assert_flow!(flow, 4, false, Some(End::Continue)); // BlockStmt of do-while
  assert_flow!(flow, 8, false, Some(End::Continue)); // if stmt
  assert_flow!(flow, 22, false, Some(End::Break)); // BlockStmt of if
  assert_flow!(flow, 28, false, Some(End::Break)); // break stmt
  assert_flow!(flow, 41, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 83, false, None); // `foo();` (which is _reachable_ if `x` equals `42`)
}

#[test]
fn for_1() {
  let src = r#"
function foo() {
  for (let i = 0; f(); i++) {
    return 1;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`

  // BlockStmt of for statement
  // This is marked as `End::Continue` because it's quite difficult to decide statically whether
  // the program enters the block or not.
  assert_flow!(flow, 46, false, Some(End::Continue));

  assert_flow!(flow, 52, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 68, false, None); // `bar();`
}

#[test]
fn for_2() {
  // infinite loop
  let src = r#"
function foo() {
  for (let i = 0; true; i++) {
    return 1;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(flow, 47, false, Some(End::forced_return())); // BlockStmt of for statement
  assert_flow!(flow, 53, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 69, true, None); // `bar();`
}

#[test]
fn for_3() {
  // infinite loop
  let src = r#"
function foo() {
  for (let i = 0;; i++) {
    return 1;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(flow, 42, false, Some(End::forced_return())); // BlockStmt of for statement
  assert_flow!(flow, 48, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 64, true, None); // `bar();`
}

#[test]
fn for_4() {
  // never enter the block of for
  let src = r#"
function foo() {
  for (let i = 0; false; i++) {
    return 1;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 48, false, Some(End::Continue)); // BlockStmt of for statement
  assert_flow!(flow, 54, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 70, false, None); // `bar();`
}

// https://github.com/denoland/deno_lint/issues/674
#[test]
fn for_5() {
  let src = r#"
for (let i = 0; true; i++) {
  if (f(i)) {
    break;
  }
  throw new Error("error");
}
foo();
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, None); // for stmt
  assert_flow!(flow, 28, false, Some(End::Continue)); // BlockStmt of for
  assert_flow!(flow, 32, false, Some(End::Continue)); // if stmt
  assert_flow!(flow, 42, false, Some(End::Break)); // BlockStmt of if
  assert_flow!(flow, 48, false, Some(End::Break)); // break stmt
  assert_flow!(flow, 61, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 89, false, None); // `foo();` (which is _reachable_ if `f(i)` is truthy)
}

#[test]
fn for_in_1() {
  let src = r#"
function foo() {
  for (let i in {}) {
    return 1;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 38, false, Some(End::Continue)); // BlockStmt of for-in
  assert_flow!(flow, 44, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 60, false, None); // `bar();`
}

#[test]
fn for_in_2() {
  let src = r#"
function foo() {
  for (let i in {}) {
    break;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 38, false, Some(End::Continue)); // BlockStmt of for-in
  assert_flow!(flow, 44, false, Some(End::Break)); // return stmt
  assert_flow!(flow, 57, false, None); // `bar();`
}

#[test]
fn for_of_1() {
  let src = r#"
function foo() {
  for (let i of []) {
    return 1;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 38, false, Some(End::Continue)); // BlockStmt of for-of
  assert_flow!(flow, 44, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 60, false, None); // `bar();`
}

#[test]
fn for_of_2() {
  let src = r#"
function foo() {
  for (let i of []) {
    break;
  }
  bar();
}
    "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 38, false, Some(End::Continue)); // BlockStmt of for-of
  assert_flow!(flow, 44, false, Some(End::Break)); // return stmt
  assert_flow!(flow, 57, false, None); // `bar();`
}

#[test]
fn try_1() {
  let src = r#"
function foo() {
  try {
    return 1;
  } finally {
    bar();
  }
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(flow, 20, false, Some(End::forced_return())); // TryStmt
  assert_flow!(flow, 24, false, Some(End::forced_return())); // BlockStmt of try
  assert_flow!(flow, 30, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 52, false, Some(End::Continue)); // BlockStmt of finally
  assert_flow!(flow, 58, false, None); // `bar();`
}

#[test]
fn try_2() {
  let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    return 2;
  }
  bar();
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(
    flow,
    20,
    false,
    Some(End::Forced {
      ret: true,
      throw: false,
      infinite_loop: false
    })
  ); // TryStmt
  assert_flow!(flow, 24, false, Some(End::forced_throw())); // BlockStmt of try
  assert_flow!(flow, 30, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 43, false, Some(End::forced_return())); // catch
  assert_flow!(flow, 53, false, Some(End::forced_return())); // BlockStmt of catch
  assert_flow!(flow, 59, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 75, true, None); // `bar();`
}

#[test]
fn try_3() {
  let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    bar();
  }
  baz();
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 20, false, Some(End::Continue)); // TryStmt
  assert_flow!(flow, 24, false, Some(End::forced_throw())); // BlockStmt of try
  assert_flow!(flow, 30, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 43, false, Some(End::Continue)); // catch
  assert_flow!(flow, 53, false, Some(End::Continue)); // BlockStmt of catch
  assert_flow!(flow, 59, false, None); // `bar();`
  assert_flow!(flow, 72, false, None); // `baz();`
}

#[test]
fn try_4() {
  let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    bar();
  } finally {
    baz();
  }
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 20, false, Some(End::Continue)); // TryStmt
  assert_flow!(flow, 24, false, Some(End::forced_throw())); // BlockStmt of try
  assert_flow!(flow, 30, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 43, false, Some(End::Continue)); // catch
  assert_flow!(flow, 53, false, Some(End::Continue)); // BlockStmt of catch
  assert_flow!(flow, 59, false, None); // `bar();`
  assert_flow!(flow, 78, false, Some(End::Continue)); // BlockStmt of finally
  assert_flow!(flow, 84, false, None); // `baz();`
}

#[test]
fn try_5() {
  let src = r#"
function foo() {
  try {
    throw 1;
  } catch (e) {
    return 2;
  } finally {
    bar();
  }
  baz();
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(
    flow,
    20,
    false,
    Some(End::Forced {
      ret: true,
      throw: false,
      infinite_loop: false
    })
  ); // TryStmt
  assert_flow!(flow, 24, false, Some(End::forced_throw())); // BlockStmt of try
  assert_flow!(flow, 30, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 43, false, Some(End::forced_return())); // catch
  assert_flow!(flow, 53, false, Some(End::forced_return())); // BlockStmt of catch
  assert_flow!(flow, 59, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 81, false, Some(End::Continue)); // BlockStmt of finally
  assert_flow!(flow, 87, false, None); // `bar();`
  assert_flow!(flow, 100, true, None); // `baz();`
}

#[test]
fn try_6() {
  let src = r#"
try {}
finally {
  break;
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Break)); // try stmt
  assert_flow!(flow, 5, false, Some(End::Continue)); // BlockStmt of try
  assert_flow!(flow, 16, false, Some(End::Break)); // BlockStmt of finally
  assert_flow!(flow, 20, false, Some(End::Break)); // break stmt
}

#[test]
fn try_7() {
  let src = r#"
try {
  throw 0;
} catch (e) {
  break;
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Break)); // try stmt
  assert_flow!(flow, 5, false, Some(End::forced_throw())); // BlockStmt of try
  assert_flow!(flow, 9, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 20, false, Some(End::Break)); // catch
  assert_flow!(flow, 30, false, Some(End::Break)); // BloskStmt of catch
  assert_flow!(flow, 34, false, Some(End::Break)); // break stmt
}

#[test]
fn try_8() {
  let src = r#"
try {
  break;
} finally {}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Break)); // try stmt
  assert_flow!(flow, 5, false, Some(End::Break)); // BlockStmt of try
  assert_flow!(flow, 9, false, Some(End::Break)); // break stmt
  assert_flow!(flow, 26, false, Some(End::Continue)); // finally
}

#[test]
fn try_9() {
  let src = r#"
try {
  try {
    throw 1;
  } catch {
    throw 2;
  }
} catch {
  foo();
}
bar();
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Continue)); // 1st try stmt
  assert_flow!(flow, 5, false, Some(End::forced_throw())); // BlockStmt of 1st try
  assert_flow!(flow, 9, false, Some(End::forced_throw())); // 2nd try stmt
  assert_flow!(flow, 13, false, Some(End::forced_throw())); // BlockStmt of 2nd try
  assert_flow!(flow, 19, false, Some(End::forced_throw())); // throw 1;
  assert_flow!(flow, 32, false, Some(End::forced_throw())); // 1st catch
  assert_flow!(flow, 44, false, Some(End::forced_throw())); // throw 2;
  assert_flow!(flow, 59, false, Some(End::Continue)); // 2nd catch
  assert_flow!(flow, 69, false, None); // `foo();`
  assert_flow!(flow, 78, false, None); // `bar();`
}

#[test]
fn try_10() {
  let src = r#"
try {
  try {
    throw 1;
  } catch {
    f();
  }
} catch {
  foo();
}
bar();
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Continue)); // 1st try stmt
  assert_flow!(flow, 5, false, Some(End::Continue)); // BlockStmt of 1st try
  assert_flow!(flow, 9, false, Some(End::Continue)); // 2nd try stmt
  assert_flow!(flow, 13, false, Some(End::forced_throw())); // BlockStmt of 2nd try
  assert_flow!(flow, 19, false, Some(End::forced_throw())); // throw 1;
  assert_flow!(flow, 32, false, Some(End::Continue)); // 1st catch
  assert_flow!(flow, 44, false, None); // `someF();`;
  assert_flow!(flow, 55, false, Some(End::Continue)); // 2nd catch
  assert_flow!(flow, 65, false, None); // `foo();`
  assert_flow!(flow, 74, false, None); // `bar();`
}

#[test]
fn try_11() {
  let src = r#"
try {
  throw 0;
} catch (e) {
  break;
} finally {
  return;
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::forced_return())); // try stmt
  assert_flow!(flow, 5, false, Some(End::forced_throw())); // BlockStmt of try
  assert_flow!(flow, 9, false, Some(End::forced_throw())); // throw stmt
  assert_flow!(flow, 20, false, Some(End::Break)); // catch
  assert_flow!(flow, 30, false, Some(End::Break)); // BloskStmt of catch
  assert_flow!(flow, 34, false, Some(End::Break)); // break stmt
  assert_flow!(flow, 51, false, Some(End::forced_return())); // finally
  assert_flow!(flow, 55, false, Some(End::forced_return())); // return stmt
}

#[test]
fn if_1() {
  let src = r#"
function foo() {
  if (a) {
    return 1;
  }
  bar();
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 20, false, Some(End::Continue)); // if
  assert_flow!(flow, 27, false, Some(End::forced_return())); // BloskStmt of if
  assert_flow!(flow, 33, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 49, false, None); // `bar();`
}

#[test]
fn if_2() {
  let src = r#"
function foo() {
  if (a) {
    bar();
  } else {
    return 1;
  }
  baz();
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 20, false, Some(End::Continue)); // if
  assert_flow!(flow, 27, false, Some(End::Continue)); // BloskStmt of if
  assert_flow!(flow, 33, false, None); // `bar();`
  assert_flow!(flow, 49, false, Some(End::forced_return())); // else
  assert_flow!(flow, 55, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 71, false, None); // `baz();`
}

#[test]
fn if_3() {
  let src = r#"
function foo() {
  if (a) {
    return 1;
  } else {
    bar();
  }
  return 0;
}
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 16, false, Some(End::forced_return())); // BlockStmt of `foo`
  assert_flow!(flow, 20, false, Some(End::Continue)); // if
  assert_flow!(flow, 27, false, Some(End::forced_return())); // BloskStmt of if
  assert_flow!(flow, 33, false, Some(End::forced_return())); // `return 1;`
  assert_flow!(flow, 52, false, Some(End::Continue)); // else
  assert_flow!(flow, 58, false, None); // `bar();`
  assert_flow!(flow, 71, false, Some(End::forced_return())); // `return 0;`
}

#[test]
fn switch_1() {
  let src = r#"
switch (foo) {
  case 1:
    return 0;
  default: {
    if (bar) {
      break;
    }
    return 0;
  }
}
throw err;
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Continue)); // switch stmt
  assert_flow!(flow, 18, false, Some(End::forced_return())); // `case 1`
  assert_flow!(flow, 30, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 42, false, Some(End::Break)); // `default`
  assert_flow!(flow, 51, false, Some(End::forced_return())); // BlockStmt of `default`
  assert_flow!(flow, 57, false, Some(End::Continue)); // if
  assert_flow!(flow, 66, false, Some(End::Break)); // BlockStmt of if
  assert_flow!(flow, 74, false, Some(End::Break)); // break stmt
  assert_flow!(flow, 91, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 107, false, Some(End::forced_throw())); // throw stmt
}

#[test]
fn switch_2() {
  let src = r#"
switch (foo) {
  case 1:
    return 0;
  default: {
    return 0;
  }
}
throw err;
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::forced_return())); // switch stmt
  assert_flow!(flow, 18, false, Some(End::forced_return())); // `case 1`
  assert_flow!(flow, 30, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 42, false, Some(End::forced_return())); // `default`
  assert_flow!(flow, 51, false, Some(End::forced_return())); // BlockStmt of `default`
  assert_flow!(flow, 57, false, Some(End::forced_return())); // return stmt
  assert_flow!(
    flow,
    73,
    true,
    Some(End::Forced {
      ret: true,
      throw: true,
      infinite_loop: false
    })
  ); // throw stmt
}

#[test]
fn switch_3() {
  let src = r#"
switch (foo) {
  case 1:
    break;
  default: {
    return 0;
  }
}
throw err;
"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Continue)); // switch stmt
  assert_flow!(flow, 18, false, Some(End::Break)); // `case 1`
  assert_flow!(flow, 30, false, Some(End::Break)); // break stmt
  assert_flow!(flow, 39, false, Some(End::forced_return())); // `default`
  assert_flow!(flow, 48, false, Some(End::forced_return())); // BlockStmt of `default`
  assert_flow!(flow, 54, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 70, false, Some(End::forced_throw())); // throw stmt
}

// https://github.com/denoland/deno_lint/issues/823
#[test]
fn issue_823() {
  let src = r#"
switch (foo) {
  case 1:
    switch (bar) {
      case 1:
        break;
    }
    return 0;
  default: {
    return 0;
  }
}"#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::forced_return())); // switch foo stmt
  assert_flow!(flow, 18, false, Some(End::forced_return())); // `switch foo case 1`
  assert_flow!(flow, 30, false, Some(End::Continue)); // `switch bar stmt`
  assert_flow!(flow, 51, false, Some(End::Break)); // `switch bar case1`
  assert_flow!(flow, 67, false, Some(End::Break)); // `break stmt`
  assert_flow!(flow, 84, false, Some(End::forced_return())); // `return stmt`
  assert_flow!(flow, 96, false, Some(End::forced_return())); // `default`
  assert_flow!(flow, 111, false, Some(End::forced_return())); // `return stmt`
}

// https://github.com/denoland/deno_lint/issues/644
#[test]
fn issue_644() {
  let src = r#"
function foo() {
  break;
}

function bar() {
  continue;
}
"#;

  // Confirms that no panic happens even if there's invalid `break` or `continue` statement
  let _ = analyze_flow(src);
}

#[test]
fn issue_716() {
  let src = r#"
function foo() {
  if (bool) {} else {}
  try {
    bar();
    return 42;
  } catch (err) {
    console.error(err);
  }
}
      "#;
  let flow = analyze_flow(src);
  assert_flow!(flow, 1, false, Some(End::Continue)); // function
  assert_flow!(flow, 16, false, Some(End::Continue)); // BlockStmt of `foo`
  assert_flow!(flow, 20, false, Some(End::Continue)); // if stmt
  assert_flow!(flow, 30, false, Some(End::Continue)); // BlockStmt of if
  assert_flow!(flow, 38, false, Some(End::Continue)); // BlockStmt of else
  assert_flow!(flow, 43, false, Some(End::Continue)); // try stmt
  assert_flow!(flow, 47, false, Some(End::forced_return())); // BlockStmt of try
  assert_flow!(flow, 53, false, None); // `bar();`
  assert_flow!(flow, 64, false, Some(End::forced_return())); // return stmt
  assert_flow!(flow, 79, false, Some(End::Continue)); // catch clause
  assert_flow!(flow, 91, false, Some(End::Continue)); // BlockStmt of catch
  assert_flow!(flow, 97, false, None); // `console.error(err);`
}
