// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use super::{Context, LintRule, ProgramRef};
use crate::handler::{Handler, Traverse, TraverseFlow};
use crate::scopes::Scope;
use dprint_swc_ecma_ast_view as AstView;
use if_chain::if_chain;
use std::convert::TryFrom;
use swc_atoms::JsWord;
use swc_common::Spanned;

pub struct NoDeprecatedDenoApi;

const CODE: &str = "no-deprecated-deno-api";

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

**IO APIs**
- `Deno.Buffer`
- `Deno.readAll`
- `Deno.readAllSync`
- `Deno.writeAll`
- `Deno.writeAllSync`
- `Deno.iter`
- `Deno.iterSync`
- `Deno.copy`

The IO APIs are already available in `std/io`, so replace these deprecated ones
with alternatives from `std/io`.
For more detail, see [the tracking issue](https://github.com/denoland/deno/issues/9795).

**Custom Inspector API**
- `Deno.customInspect`

`Deno.customInspect` was deprecated in favor of
`Symbol.for("Deno.customInspect")`. Replace the usages with this symbol
expression. See [deno#9294](https://github.com/denoland/deno/issues/9294)
for more details.

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

// iter
for await (const x of Deno.iter(xs)) {}
for (const y of Deno.iterSync(ys)) {}

// copy
await Deno.copy(reader, writer);

// custom inspector
class A {
  [Deno.customInspect]() {
    return "This is A";
  }
}
```

### Valid:

```typescript
// buffer
import { Buffer } from "https://deno.land/std/io/buffer.ts";
const a = new Buffer();

// read
import { readAll, readAllSync } from "https://deno.land/std/io/util.ts";
const b = await readAll(reader);
const c = readAllSync(reader);

// write
import { writeAll, writeAllSync } from "https://deno.land/std/io/util.ts";
await writeAll(writer, data);
writeAllSync(writer, data);

// iter
import { iter, iterSync } from "https://deno.land/std/io/util.ts";
for await (const x of iter(xs)) {}
for (const y of iterSync(ys)) {}

// copy
import { copy } from "https://deno.land/std/io/util.ts";
await copy(reader, writer);

// custom inspector
class A {
  [Symbol.for("Deno.customInspect")]() {
    return "This is A";
  }
}
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

/// Extracts a symbol from the given expression if the symbol is statically determined (otherwise,
/// return `None`).
fn extract_symbol<'a>(expr: &'a AstView::Expr) -> Option<&'a JsWord> {
  use AstView::{Expr, Lit, Tpl};
  match expr {
    Expr::Lit(Lit::Str(s)) => Some(s.value()),
    Expr::Ident(ident) => Some(ident.sym()),
    Expr::Tpl(Tpl {
      ref exprs,
      ref quasis,
      ..
    }) if exprs.is_empty() && quasis.len() == 1 => Some(quasis[0].raw.value()),
    _ => None,
  }
}

enum DeprecatedApi {
  Buffer,
  ReadAll,
  ReadAllSync,
  WriteAll,
  WriteAllSync,
  Iter,
  IterSync,
  Copy,
  CustomInspect,
}

impl TryFrom<(&JsWord, &JsWord)> for DeprecatedApi {
  type Error = ();

  /// Converts the given member expression (made up of `obj_symbol` and `prop_symbol`) into
  /// `DeprecatedApi` if it's one of deprecated APIs.
  /// Note that this conversion does not take shadowing into account, so use this after calling
  /// `is_shadowed`.
  fn try_from(
    (obj_symbol, prop_symbol): (&JsWord, &JsWord),
  ) -> Result<Self, Self::Error> {
    if obj_symbol != "Deno" {
      return Err(());
    }

    match prop_symbol.as_ref() {
      "Buffer" => Ok(DeprecatedApi::Buffer),
      "readAll" => Ok(DeprecatedApi::ReadAll),
      "readAllSync" => Ok(DeprecatedApi::ReadAllSync),
      "writeAll" => Ok(DeprecatedApi::WriteAll),
      "writeAllSync" => Ok(DeprecatedApi::WriteAllSync),
      "iter" => Ok(DeprecatedApi::Iter),
      "iterSync" => Ok(DeprecatedApi::IterSync),
      "copy" => Ok(DeprecatedApi::Copy),
      "customInspect" => Ok(DeprecatedApi::CustomInspect),
      _ => Err(()),
    }
  }
}
enum Replacement {
  NameAndUrl(&'static str, &'static str),
  Name(&'static str),
}

impl DeprecatedApi {
  fn message(&self) -> String {
    let name = self.get_deprecated_api_name();
    format!(
      "`{}` is deprecated and scheduled for removal in Deno 2.0",
      name,
    )
  }

  fn hint(&self) -> String {
    match self.get_replacement() {
      Replacement::Name(name) => format!("Use `{}` instead", name),
      Replacement::NameAndUrl(name, url) => {
        format!("Use `{}` from {} instead", name, url)
      }
    }
  }

  fn get_deprecated_api_name(&self) -> &'static str {
    use DeprecatedApi::*;
    match *self {
      Buffer => "Deno.Buffer",
      ReadAll => "Deno.readAll",
      ReadAllSync => "Deno.readAllSync",
      WriteAll => "Deno.writeAll",
      WriteAllSync => "Deno.writeAllSync",
      Iter => "Deno.iter",
      IterSync => "Deno.iterSync",
      Copy => "Deno.copy",
      CustomInspect => "Deno.customInspect",
    }
  }

  fn get_replacement(&self) -> Replacement {
    const BUFFER_TS: &str = "https://deno.land/std/io/buffer.ts";
    const UTIL_TS: &str = "https://deno.land/std/io/util.ts";

    use DeprecatedApi::*;
    use Replacement::*;
    match *self {
      Buffer => NameAndUrl("Buffer", BUFFER_TS),
      ReadAll => NameAndUrl("readAll", UTIL_TS),
      ReadAllSync => NameAndUrl("readAllSync", UTIL_TS),
      WriteAll => NameAndUrl("writeAll", UTIL_TS),
      WriteAllSync => NameAndUrl("writeAllSync", UTIL_TS),
      Iter => NameAndUrl("iter", UTIL_TS),
      IterSync => NameAndUrl("iterSync", UTIL_TS),
      Copy => NameAndUrl("copy", UTIL_TS),
      CustomInspect => Name("Symbol.for(\"Deno.customInspect\")"),
    }
  }
}

struct NoDeprecatedDenoApiHandler;

impl Handler for NoDeprecatedDenoApiHandler {
  fn member_expr(
    &mut self,
    member_expr: &AstView::MemberExpr,
    ctx: &mut Context,
  ) -> TraverseFlow {
    // Not check chained member expressions (e.g. `foo.bar.baz`)
    if member_expr.parent().is::<AstView::MemberExpr>() {
      return TraverseFlow::Continue;
    }

    use AstView::{Expr, ExprOrSuper};
    if_chain! {
      if let ExprOrSuper::Expr(Expr::Ident(obj)) = &member_expr.obj;
      let obj_symbol = obj.sym();
      if !is_shadowed(obj_symbol, ctx.scope());
      if let Some(prop_symbol) = extract_symbol(&member_expr.prop);
      if let Ok(deprecated_api) = DeprecatedApi::try_from((obj_symbol, prop_symbol));
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.span(),
          CODE,
          deprecated_api.message(),
          deprecated_api.hint(),
        );
      }
    }

    TraverseFlow::Continue
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
      "Deno.foo.iter();",
      "Deno.foo.iterSync();",
      "Deno.foo.copy();",
      "Deno.foo.customInspect;",
      "foo.Deno.Buffer();",
      "foo.Deno.readAll();",
      "foo.Deno.readAllSync();",
      "foo.Deno.writeAll();",
      "foo.Deno.writeAllSync();",
      "foo.Deno.iter();",
      "foo.Deno.iterSync();",
      "foo.Deno.copy();",
      "foo.Deno.customInspect;",

      // `Deno` is shadowed
      "const Deno = 42; const a = new Deno.Buffer();",
      "const Deno = 42; const a = await Deno.readAll(reader);",
      "const Deno = 42; const a = Deno.readAllSync(reader);",
      "const Deno = 42; await Deno.writeAll(writer, data);",
      "const Deno = 42; Deno.writeAllSync(writer, data);",
      "const Deno = 42; for await (const x of Deno.iter(xs)) {}",
      "const Deno = 42; for (const x of Deno.iterSync(xs)) {}",
      "const Deno = 42; await Deno.copy(reader, writer);",
      r#"const Deno = 42; Deno.customInspect"#,
      r#"import { Deno } from "./foo.ts"; Deno.writeAllSync(writer, data);"#,

      // access property with string literal (shadowed)
      r#"const Deno = 42; new Deno["Buffer"]();"#,
      r#"const Deno = 42; Deno["readAll"](reader);"#,
      r#"const Deno = 42; Deno["readAllSync"](reader);"#,
      r#"const Deno = 42; Deno["writeAll"](writer, data);"#,
      r#"const Deno = 42; Deno["writeAllSync"](writer, data);"#,
      r#"const Deno = 42; for await (const x of Deno["iter"](xs)) {}"#,
      r#"const Deno = 42; for (const x of Deno["iterSync"](xs)) {}"#,
      r#"const Deno = 42; Deno["copy"](reader, writer);"#,
      r#"const Deno = 42; Deno["customInspect"]"#,

      // access property with template literal (shadowed)
      r#"const Deno = 42; new Deno[`Buffer`]();"#,
      r#"const Deno = 42; Deno[`readAll`](reader);"#,
      r#"const Deno = 42; Deno[`readAllSync`](reader);"#,
      r#"const Deno = 42; Deno[`writeAll`](writer, data);"#,
      r#"const Deno = 42; Deno[`writeAllSync`](writer, data);"#,
      r#"const Deno = 42; for await (const x of Deno[`iter`](xs)) {}"#,
      r#"const Deno = 42; for (const x of Deno[`iterSync`](xs)) {}"#,
      r#"const Deno = 42; Deno[`copy`](reader, writer);"#,
      r#"const Deno = 42; Deno[`customInspect`]"#,

      // Ignore template literals that include expressions
      r#"const read = "read"; Deno[`${read}All`](reader);"#,
    };
  }

  #[test]
  fn no_deprecated_deno_api_invalid() {
    use DeprecatedApi::*;

    assert_lint_err! {
      NoDeprecatedDenoApi,
      "new Deno.Buffer();": [
        {
          col: 4,
          message: Buffer.message(),
          hint: Buffer.hint()
        }
      ],
      "Deno.readAll(reader);": [
        {
          col: 0,
          message: ReadAll.message(),
          hint: ReadAll.hint()
        }
      ],
      "Deno.readAllSync(reader);": [
        {
          col: 0,
          message: ReadAllSync.message(),
          hint: ReadAllSync.hint()
        }
      ],
      "Deno.writeAll(writer, data);": [
        {
          col: 0,
          message: WriteAll.message(),
          hint: WriteAll.hint()
        }
      ],
      "Deno.writeAllSync(writer, data);": [
        {
          col: 0,
          message: WriteAllSync.message(),
          hint: WriteAllSync.hint()
        }
      ],
      "Deno.iter(reader);": [
        {
          col: 0,
          message: Iter.message(),
          hint: Iter.hint()
        }
      ],
      "Deno.iterSync(reader);": [
        {
          col: 0,
          message: IterSync.message(),
          hint: IterSync.hint()
        }
      ],
      "Deno.copy(reader, writer);": [
        {
          col: 0,
          message: Copy.message(),
          hint: Copy.hint()
        }
      ],
      "Deno.customInspect;": [
        {
          col: 0,
          message: CustomInspect.message(),
          hint: CustomInspect.hint()
        }
      ],

      // access property with string literal
      r#"new Deno["Buffer"]();"#: [
        {
          col: 4,
          message: Buffer.message(),
          hint: Buffer.hint()
        }
      ],
      r#"Deno["readAll"](reader);"#: [
        {
          col: 0,
          message: ReadAll.message(),
          hint: ReadAll.hint()
        }
      ],
      r#"Deno["readAllSync"](reader);"#: [
        {
          col: 0,
          message: ReadAllSync.message(),
          hint: ReadAllSync.hint()
        }
      ],
      r#"Deno["writeAll"](writer, data);"#: [
        {
          col: 0,
          message: WriteAll.message(),
          hint: WriteAll.hint()
        }
      ],
      r#"Deno["writeAllSync"](writer, data);"#: [
        {
          col: 0,
          message: WriteAllSync.message(),
          hint: WriteAllSync.hint()
        }
      ],
      r#"Deno["iter"](reader);"#: [
        {
          col: 0,
          message: Iter.message(),
          hint: Iter.hint()
        }
      ],
      r#"Deno["iterSync"](reader);"#: [
        {
          col: 0,
          message: IterSync.message(),
          hint: IterSync.hint()
        }
      ],
      r#"Deno["copy"](reader, writer);"#: [
        {
          col: 0,
          message: Copy.message(),
          hint: Copy.hint()
        }
      ],
      r#"Deno["customInspect"];"#: [
        {
          col: 0,
          message: CustomInspect.message(),
          hint: CustomInspect.hint()
        }
      ],

      // access property with template literal
      r#"new Deno[`Buffer`]();"#: [
        {
          col: 4,
          message: Buffer.message(),
          hint: Buffer.hint()
        }
      ],
      r#"Deno[`readAll`](reader);"#: [
        {
          col: 0,
          message: ReadAll.message(),
          hint: ReadAll.hint()
        }
      ],
      r#"Deno[`readAllSync`](reader);"#: [
        {
          col: 0,
          message: ReadAllSync.message(),
          hint: ReadAllSync.hint()
        }
      ],
      r#"Deno[`writeAll`](writer, data);"#: [
        {
          col: 0,
          message: WriteAll.message(),
          hint: WriteAll.hint()
        }
      ],
      r#"Deno[`writeAllSync`](writer, data);"#: [
        {
          col: 0,
          message: WriteAllSync.message(),
          hint: WriteAllSync.hint()
        }
      ],
      r#"Deno[`iter`](reader);"#: [
        {
          col: 0,
          message: Iter.message(),
          hint: Iter.hint()
        }
      ],
      r#"Deno[`iterSync`](reader);"#: [
        {
          col: 0,
          message: IterSync.message(),
          hint: IterSync.hint()
        }
      ],
      r#"Deno[`copy`](reader);"#: [
        {
          col: 0,
          message: Copy.message(),
          hint: Copy.hint()
        }
      ],
      r#"Deno[`customInspect`];"#: [
        {
          col: 0,
          message: CustomInspect.message(),
          hint: CustomInspect.hint()
        }
      ],
    }
  }

  #[test]
  #[ignore = "Scope analyzer enhancement is required to deal with this"]
  fn shadowed_in_unrelated_scope() {
    use DeprecatedApi::*;
    assert_lint_err! {
      NoDeprecatedDenoApi,
      r#"
function foo () {
  const Deno = 42;
}
Deno.readAll(reader);
      "#: [
        {
          line: 5,
          col: 0,
          message: ReadAll.message(),
          hint: ReadAll.hint()
        }
      ],
    }
  }
}
