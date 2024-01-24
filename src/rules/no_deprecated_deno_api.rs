// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.
use super::Context;
use super::LintRule;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::Program;

use deno_ast::view as ast_view;
use deno_ast::SourceRanged;
use if_chain::if_chain;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct NoDeprecatedDenoApi;

const CODE: &str = "no-deprecated-deno-api";

impl LintRule for NoDeprecatedDenoApi {
  fn tags(&self) -> &'static [&'static str] {
    &["recommended"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program<'_>,
  ) {
    NoDeprecatedDenoApiHandler.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/no_deprecated_deno_api.md")
  }
}

/// Extracts a symbol from the given member prop if the symbol is statically determined (otherwise,
/// return `None`).
fn extract_symbol<'a>(
  member_prop: &'a ast_view::MemberProp,
) -> Option<&'a str> {
  use deno_ast::view::{Expr, Lit, MemberProp, Tpl};
  match member_prop {
    MemberProp::Ident(ident) => Some(ident.sym()),
    MemberProp::PrivateName(ident) => Some(ident.id.sym()),
    MemberProp::Computed(prop) => match &prop.expr {
      Expr::Lit(Lit::Str(s)) => Some(s.value()),
      Expr::Ident(ident) => Some(ident.sym()),
      Expr::Tpl(Tpl {
        ref exprs,
        ref quasis,
        ..
      }) if exprs.is_empty() && quasis.len() == 1 => Some(quasis[0].raw()),
      _ => None,
    },
  }
}

enum DeprecatedApi {
  Buffer,
  Copy,
  CustomInspect,
  File,
  Iter,
  IterSync,
  Read,
  ReadSync,
  ReadAll,
  ReadAllSync,
  Run,
  ServeHttp,
  WriteAll,
  WriteAllSync,
}

impl TryFrom<(&str, &str)> for DeprecatedApi {
  type Error = ();

  /// Converts the given member expression (made up of `obj_symbol` and `prop_symbol`) into
  /// `DeprecatedApi` if it's one of deprecated APIs.
  /// Note that this conversion does not take shadowing into account, so use this after calling
  /// `is_global` method on the scope analyzer.
  fn try_from(
    (obj_symbol, prop_symbol): (&str, &str),
  ) -> Result<Self, Self::Error> {
    if obj_symbol != "Deno" {
      return Err(());
    }

    match prop_symbol {
      "Buffer" => Ok(DeprecatedApi::Buffer),
      "copy" => Ok(DeprecatedApi::Copy),
      "customInspect" => Ok(DeprecatedApi::CustomInspect),
      "iter" => Ok(DeprecatedApi::Iter),
      "iterSync" => Ok(DeprecatedApi::IterSync),
      "File" => Ok(DeprecatedApi::File),
      "read" => Ok(DeprecatedApi::Read),
      "readSync" => Ok(DeprecatedApi::ReadSync),
      "readAll" => Ok(DeprecatedApi::ReadAll),
      "readAllSync" => Ok(DeprecatedApi::ReadAllSync),
      "run" => Ok(DeprecatedApi::Run),
      "serveHttp" => Ok(DeprecatedApi::ServeHttp),
      "writeAll" => Ok(DeprecatedApi::WriteAll),
      "writeAllSync" => Ok(DeprecatedApi::WriteAllSync),
      _ => Err(()),
    }
  }
}
enum Replacement {
  NameAndUrl(&'static str, &'static str),
  Name(&'static str),
  #[allow(dead_code)]
  NameAndUrls(Vec<(&'static str, &'static str)>),
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
      Replacement::NameAndUrls(name_and_urls) => {
        let mut hint = String::from("Use ");
        for (i, (name, url)) in name_and_urls.into_iter().enumerate() {
          if i != 0 {
            hint.push_str(" and ");
          }
          hint.push_str(&format!("`{}` from {}", name, url));
        }
        hint.push_str(" instead");
        hint
      }
    }
  }

  fn get_deprecated_api_name(&self) -> &'static str {
    use DeprecatedApi::*;
    match *self {
      Buffer => "Deno.Buffer",
      Copy => "Deno.copy",
      CustomInspect => "Deno.customInspect",
      Iter => "Deno.iter",
      IterSync => "Deno.iterSync",
      File => "Deno.File",
      Read => "Deno.read",
      ReadSync => "Deno.readSync",
      ReadAll => "Deno.readAll",
      ReadAllSync => "Deno.readAllSync",
      Run => "Deno.run",
      ServeHttp => "Deno.serveHttp",
      WriteAll => "Deno.writeAll",
      WriteAllSync => "Deno.writeAllSync",
    }
  }

  fn get_replacement(&self) -> Replacement {
    const DENO_COMMAND_API: &str = "https://deno.land/api?s=Deno.Command";
    const DENO_SERVE_API: &str = "https://deno.land/api?s=Deno.serve";
    const STD_BUFFER: &str = "https://deno.land/std/io/buffer.ts?s=Buffer";
    const STD_COPY: &str = "https://deno.land/std/io/copy.ts?s=copy";
    const STD_READ_ALL: &str = "https://deno.land/std/io/read_all.ts?s=readAll";
    const STD_READ_ALL_SYNC: &str =
      "https://deno.land/std/io/read_all.ts?s=readAllSync";
    const STD_WRITE_ALL: &str =
      "https://deno.land/std/io/write_all.ts?s=writeAll";
    const STD_WRITE_ALL_SYNC: &str =
      "https://deno.land/std/io/write_all.ts?s=writeAllSync";
    const STREAMS_READABLE_TS: &str = "https://deno.land/api?s=ReadableStream";

    use DeprecatedApi::*;
    use Replacement::*;
    match *self {
      Buffer => Name(STD_BUFFER),
      Copy => Name(STD_COPY),
      CustomInspect => Name("Symbol.for(\"Deno.customInspect\")"),
      Iter => Name(STREAMS_READABLE_TS),
      IterSync => Name(STREAMS_READABLE_TS),
      File => Name("Deno.FsFile"),
      Read => Name("resource.read"),
      ReadSync => Name("resource.readSync"),
      ReadAll => NameAndUrl("readAll", STD_READ_ALL),
      ReadAllSync => NameAndUrl("readAllSync", STD_READ_ALL_SYNC),
      Run => NameAndUrl("Deno.Command", DENO_COMMAND_API),
      ServeHttp => NameAndUrl("Deno.serve", DENO_SERVE_API),
      WriteAll => NameAndUrl("writeAll", STD_WRITE_ALL),
      WriteAllSync => NameAndUrl("writeAllSync", STD_WRITE_ALL_SYNC),
    }
  }
}

struct NoDeprecatedDenoApiHandler;

impl Handler for NoDeprecatedDenoApiHandler {
  fn member_expr(
    &mut self,
    member_expr: &ast_view::MemberExpr,
    ctx: &mut Context,
  ) {
    // Not check chained member expressions (e.g. `foo.bar.baz`)
    if member_expr.parent().is::<ast_view::MemberExpr>() {
      return;
    }

    use deno_ast::view::Expr;
    if_chain! {
      if let Expr::Ident(obj) = &member_expr.obj;
      if ctx.scope().is_global(&obj.inner.to_id());
      let obj_symbol: &str = obj.sym();
      if let Some(prop_symbol) = extract_symbol(&member_expr.prop);
      if let Ok(deprecated_api) = DeprecatedApi::try_from((obj_symbol, prop_symbol));
      then {
        ctx.add_diagnostic_with_hint(
          member_expr.range(),
          CODE,
          deprecated_api.message(),
          deprecated_api.hint(),
        );
      }
    }
  }

  fn ts_qualified_name(
    &mut self,
    qualified_name: &ast_view::TsQualifiedName,
    ctx: &mut Context,
  ) {
    if_chain! {
      if let ast_view::TsEntityName::Ident(ident) = qualified_name.left;
      if ident.sym() == "Deno";
      if qualified_name.right.sym() == "File";
      if ctx.scope().is_global(&ident.inner.to_id());
      then {
        let deprecated_api = DeprecatedApi::File;
        ctx.add_diagnostic_with_hint(
          qualified_name.range(),
          CODE,
          deprecated_api.message(),
          deprecated_api.hint(),
        );
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
      "Deno.foo.iter();",
      "Deno.foo.iterSync();",
      "Deno.foo.copy();",
      "Deno.foo.customInspect;",
      "Deno.foo.read();",
      "Deno.foo.readSync();",
      "foo.Deno.Buffer();",
      "foo.Deno.readAll();",
      "foo.Deno.readAllSync();",
      "foo.Deno.writeAll();",
      "foo.Deno.writeAllSync();",
      "foo.Deno.iter();",
      "foo.Deno.iterSync();",
      "foo.Deno.copy();",
      "foo.Deno.customInspect;",
      "foo.Deno.read();",
      "foo.Deno.readSync();",

      // `Deno` is shadowed
      "const Deno = 42; const a = new Deno.Buffer();",
      "const Deno = 42; const a = await Deno.readAll(reader);",
      "const Deno = 42; const a = Deno.readAllSync(reader);",
      "const Deno = 42; await Deno.writeAll(writer, data);",
      "const Deno = 42; Deno.writeAllSync(writer, data);",
      "const Deno = 42; for await (const x of Deno.iter(xs)) {}",
      "const Deno = 42; for (const x of Deno.iterSync(xs)) {}",
      "const Deno = 42; await Deno.copy(reader, writer);",
      "const Deno = 42; const a = await Deno.read(reader);",
      "const Deno = 42; const a = Deno.readSync(reader);",
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
      r#"const Deno = 42; Deno["read"](reader);"#,
      r#"const Deno = 42; Deno["readSync"](reader);"#,

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
      r#"const Deno = 42; Deno[`read`](reader);"#,
      r#"const Deno = 42; Deno[`readSync`](reader);"#,

      // Ignore template literals that include expressions
      r#"const read = "read"; Deno[`${read}All`](reader);"#,
      r#"const sync = "Sync"; Deno[`read${sync}`](reader);"#,

      // types
      r#"interface Deno {} let file: Deno.File;"#,
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
      "Deno.read(reader);": [
        {
          col: 0,
          message: Read.message(),
          hint: Read.hint()
        }
      ],
      "Deno.readSync(reader);": [
        {
          col: 0,
          message: ReadSync.message(),
          hint: ReadSync.hint()
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
      "Deno.File;": [
        {
          col: 0,
          message: File.message(),
          hint: File.hint()
        }
      ],
      "let file: Deno.File;": [
        {
          col: 10,
          message: File.message(),
          hint: File.hint()
        }
      ],
      "Deno.run(options);": [
        {
          col: 0,
          message: Run.message(),
          hint: Run.hint()
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
      r#"Deno[`File`];"#: [
        {
          col: 0,
          message: File.message(),
          hint: File.hint()
        }
      ],

      // `Deno` is shadowed in the other scope
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
      r#"Deno.serveHttp();"#: [
        {
          col: 0,
          message: ServeHttp.message(),
          hint: ServeHttp.hint()
        }
      ],
      r#"Deno[`serveHttp`];"#: [
        {
          col: 0,
          message: ServeHttp.message(),
          hint: ServeHttp.hint()
        }
      ],
    }
  }

  #[test]
  fn expect_deprecated_api_hint() {
    let tests = vec![
      (
        "Buffer",
        "Use `https://deno.land/std/io/buffer.ts?s=Buffer` instead",
      ),
      ("copy", "Use `https://deno.land/std/io/copy.ts?s=copy` instead"),
      ("customInspect", "Use `Symbol.for(\"Deno.customInspect\")` instead"),
      ("File", "Use `Deno.FsFile` instead"),
      ("iter", "Use `https://deno.land/api?s=ReadableStream` instead"),
      ("iterSync", "Use `https://deno.land/api?s=ReadableStream` instead"),
      ("read", "Use `resource.read` instead"),
      ("readSync", "Use `resource.readSync` instead"),
      ("readAll", "Use `readAll` from https://deno.land/std/io/read_all.ts?s=readAll instead"),
      ("readAllSync", "Use `readAllSync` from https://deno.land/std/io/read_all.ts?s=readAllSync instead"),
      ("run", "Use `Deno.Command` from https://deno.land/api?s=Deno.Command instead"),
      ("serveHttp", "Use `Deno.serve` from https://deno.land/api?s=Deno.serve instead"),
      ("writeAll", "Use `writeAll` from https://deno.land/std/io/write_all.ts?s=writeAll instead"),
      ("writeAllSync", "Use `writeAllSync` from https://deno.land/std/io/write_all.ts?s=writeAllSync instead"),
    ];

    for test in tests {
      let hint = DeprecatedApi::try_from(("Deno", test.0)).unwrap().hint();
      assert_eq!(hint, test.1);
    }
  }
}
