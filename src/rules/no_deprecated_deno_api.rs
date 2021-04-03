// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse};
use crate::scopes::Scope;
use dprint_swc_ecma_ast_view as AstView;
use if_chain::if_chain;
use swc_atoms::JsWord;
use swc_common::Spanned;

pub struct NoDeprecatedDenoApi;

const CODE: &str = "no-deprecated-deno-api";
const MESSAGE: &str = "This API is deprecated";
const HINT: &str = "Consider using alternative APIs in `std`";

impl LintRule for NoDeprecatedDenoApi {
  fn new() -> Box<Self> {
    Box::new(NoDeprecatedDenoApi)
  }

  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
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
    program: AstView::Program<'_>,
  ) {
    NoDeprecatedDenoApiHandler.traverse(program, context);
  }

  fn docs(&self) -> &'static str {
    r#"Warns the usage of the deprecated Deno APIs

The following APIs in `Deno` namespace are now marked as deprecated and will get
removed from the namespace in the future.

- `Deno.Buffer`
- `Deno.readAll`
- `Deno.readAllSync`
- `Deno.writeAll`
- `Deno.writeAllSync`

They are already available in `std`, so replace these deprecated ones with
alternatives from `std`.
For more detail, see [the tracking issue](https://github.com/denoland/deno/issues/9795).

### Invalid:
```typescript
// buffer
const a = Deno.Buffer();

// read
const b = await Deno.readAll(reader);
const c = Deno.readAllSync(reader);

// write
await Deno.writeAll(writer, data);
Deno.writeAllSync(writer, data);
```

### Valid:
```typescript
// buffer
import { Buffer } from "https://deno.land/std@0.92.0/io/buffer.ts";
const a = new Buffer();

// read
import { readAll, readAllSync } from "https://deno.land/std@0.92.0/io/util.ts";
const b = await readAll(reader);
const c = readAllSync(reader);

// write
import { writeAll, writeAllSync } from "https://deno.land/std@0.92.0/io/util.ts";
await writeAll(writer, data);
writeAllSync(writer, data);
```
"#
  }
}

/// Checks if the symbol is declared in user-land.
/// This is meant to be used for determining whether the global `Deno` object is valid at the
/// point.
// TODO(@magurotuna): scope analyzer enhancement is required to handle shadowing correctly.
fn is_shadowed(symbol: &JsWord, scope: &Scope) -> bool {
  scope.ids_with_symbol(symbol).is_some()
}

/// Checks if the given member expression (made up of `obj_symbol` and `prop_symbol`) is deprecated
/// API or not. Note that this function does not take shadowing into account, so use it after
/// calling `is_shadowed`.
fn is_deprecated(obj_symbol: &JsWord, prop_symbol: &JsWord) -> bool {
  const DEPRECATED_APIS: &[&str] = &[
    "Buffer",
    "readAll",
    "readAllSync",
    "writeAll",
    "writeAllSync",
  ];

  obj_symbol == "Deno" && DEPRECATED_APIS.iter().any(|d| *d == prop_symbol)
}

/// Extracts a symbol from the given expression if the symbol is statically determined (otherwise,
/// return `None`).
fn extract_symbol<'a>(expr: &'a AstView::Expr) -> Option<&'a JsWord> {
  use AstView::{Expr, Lit, Tpl};
  match expr {
    Expr::Lit(Lit::Str(ref s)) => Some(s.value()),
    Expr::Ident(ref ident) => Some(ident.sym()),
    Expr::Tpl(Tpl {
      ref exprs,
      ref quasis,
      ..
    }) if exprs.is_empty() && quasis.len() == 1 => Some(quasis[0].raw.value()),
    _ => None,
  }
}

struct NoDeprecatedDenoApiHandler;

impl Handler for NoDeprecatedDenoApiHandler {
  fn member_expr(
    &mut self,
    member_expr: &AstView::MemberExpr,
    ctx: &mut Context,
  ) {
    // Not check chained member expressions (e.g. `foo.bar.baz`)
    if member_expr.parent.is::<AstView::MemberExpr>() {
      return;
    }

    use AstView::{Expr, ExprOrSuper};
    if_chain! {
      if let ExprOrSuper::Expr(Expr::Ident(ref obj)) = &member_expr.obj;
      let obj_symbol = obj.sym();
      if !is_shadowed(obj_symbol, &ctx.scope);
      if let Some(prop_symbol) = extract_symbol(&member_expr.prop);
      if is_deprecated(obj_symbol, prop_symbol);
      then {
        ctx.add_diagnostic_with_hint(member_expr.span(), CODE, MESSAGE, HINT);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_deprecated_deno_api_valid() {
    assert_lint_ok! {
      NoDeprecatedDenoApi,
      "Deno.foo();",
      "Deno.foo.Buffer();",
      "Deno.foo.readAll();",
      "Deno.foo.readAllSync();",
      "Deno.foo.writeAll();",
      "Deno.foo.writeAllSync();",
      "foo.Deno.Buffer();",
      "foo.Deno.readAll();",
      "foo.Deno.readAllSync();",
      "foo.Deno.writeAll();",
      "foo.Deno.writeAllSync();",

      // `Deno` is shadowed
      "const Deno = 42; const a = new Deno.Buffer();",
      "const Deno = 42; const a = await Deno.readAll(reader);",
      "const Deno = 42; const a = Deno.readAllSync(reader);",
      "const Deno = 42; await Deno.writeAll(writer, data);",
      "const Deno = 42; Deno.writeAllSync(writer, data);",
      r#"import { Deno } from "./foo.ts"; Deno.writeAllSync(writer, data);"#,

      // access property with string literal (shadowed)
      r#"const Deno = 42; new Deno["Buffer"]();"#,
      r#"const Deno = 42; Deno["readAll"](reader);"#,
      r#"const Deno = 42; Deno["readAllSync"](reader);"#,
      r#"const Deno = 42; Deno["writeAll"](writer, data);"#,
      r#"const Deno = 42; Deno["writeAllSync"](writer, data);"#,

      // access property with template literal (shadowed)
      r#"const Deno = 42; new Deno[`Buffer`]();"#,
      r#"const Deno = 42; Deno[`readAll`](reader);"#,
      r#"const Deno = 42; Deno[`readAllSync`](reader);"#,
      r#"const Deno = 42; Deno[`writeAll`](writer, data);"#,
      r#"const Deno = 42; Deno[`writeAllSync`](writer, data);"#,

      // Ignore template literals that include expressions
      r#"const read = "read"; Deno[`${read}All`](reader);"#,
    };
  }

  #[test]
  fn no_deprecated_deno_api_invalid() {
    assert_lint_err! {
      NoDeprecatedDenoApi,
      "new Deno.Buffer();": [{ col: 4, message: MESSAGE, hint: HINT }],
      "Deno.readAll(reader);": [{ col: 0, message: MESSAGE, hint: HINT }],
      "Deno.readAllSync(reader);": [{ col: 0, message: MESSAGE, hint: HINT }],
      "Deno.writeAll(writer, data);": [{ col: 0, message: MESSAGE, hint: HINT }],
      "Deno.writeAllSync(writer, data);": [{ col: 0, message: MESSAGE, hint: HINT }],

      // access property with string literal
      r#"new Deno["Buffer"]();"#: [{ col: 4, message: MESSAGE, hint: HINT }],
      r#"Deno["readAll"](reader);"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"Deno["readAllSync"](reader);"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"Deno["writeAll"](writer, data);"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"Deno["writeAllSync"](writer, data);"#: [{ col: 0, message: MESSAGE, hint: HINT }],

      // access property with template literal
      r#"new Deno[`Buffer`]();"#: [{ col: 4, message: MESSAGE, hint: HINT }],
      r#"Deno[`readAll`](reader);"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"Deno[`readAllSync`](reader);"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"Deno[`writeAll`](writer, data);"#: [{ col: 0, message: MESSAGE, hint: HINT }],
      r#"Deno[`writeAllSync`](writer, data);"#: [{ col: 0, message: MESSAGE, hint: HINT }],
    }
  }

  #[test]
  #[ignore = "Scope analyzer enhancement is required to deal with this"]
  fn shadowed_in_unrelated_scope() {
    assert_lint_err! {
      NoDeprecatedDenoApi,
      r#"
function foo () {
  const Deno = 42;
}
Deno.readAll(reader);
      "#: [{ line: 5, col: 0, message: MESSAGE, hint: HINT }],
    }
  }
}
