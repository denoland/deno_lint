// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::Context;
use super::LintRule;
use crate::handler::Handler;
use crate::handler::Traverse;
use crate::tags;
use crate::tags::Tags;
use crate::Program;

use deno_ast::view as ast_view;
use deno_ast::SourceRanged;
use if_chain::if_chain;
use std::convert::TryFrom;

#[derive(Debug)]
pub struct NoDeprecatedDenoApi;

const CODE: &str = "no-deprecated-deno-api";

impl LintRule for NoDeprecatedDenoApi {
  fn tags(&self) -> Tags {
    &[tags::RECOMMENDED]
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
    MemberProp::PrivateName(ident) => Some(ident.name()),
    MemberProp::Computed(prop) => match &prop.expr {
      Expr::Lit(Lit::Str(s)) => Some(s.value()),
      Expr::Ident(ident) => Some(ident.sym()),
      Expr::Tpl(Tpl { exprs, quasis, .. })
        if exprs.is_empty() && quasis.len() == 1 =>
      {
        Some(quasis[0].raw())
      }
      _ => None,
    },
  }
}

enum DeprecatedApi {
  Buffer,
  Close,
  Copy,
  CustomInspect,
  Fdatasync,
  FdatasyncSync,
  File,
  Flock,
  FlockSync,
  Fstat,
  FstatSync,
  Fsync,
  FsyncSync,
  Ftruncate,
  FtruncateSync,
  Funlock,
  FunlockSync,
  Futime,
  FutimeSync,
  Isatty,
  Iter,
  IterSync,
  Metrics,
  Read,
  ReadSync,
  ReadAll,
  ReadAllSync,
  Resources,
  Run,
  Seek,
  SeekSync,
  ServeHttp,
  Shutdown,
  Write,
  WriteSync,
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
      "close" => Ok(DeprecatedApi::Close),
      "copy" => Ok(DeprecatedApi::Copy),
      "customInspect" => Ok(DeprecatedApi::CustomInspect),
      "fdatasync" => Ok(DeprecatedApi::Fdatasync),
      "fdatasyncSync" => Ok(DeprecatedApi::FdatasyncSync),
      "File" => Ok(DeprecatedApi::File),
      "flock" => Ok(DeprecatedApi::Flock),
      "flockSync" => Ok(DeprecatedApi::FlockSync),
      "fstat" => Ok(DeprecatedApi::Fstat),
      "fstatSync" => Ok(DeprecatedApi::FstatSync),
      "fsync" => Ok(DeprecatedApi::Fsync),
      "fsyncSync" => Ok(DeprecatedApi::FsyncSync),
      "ftruncate" => Ok(DeprecatedApi::Ftruncate),
      "ftruncateSync" => Ok(DeprecatedApi::FtruncateSync),
      "funlock" => Ok(DeprecatedApi::Funlock),
      "funlockSync" => Ok(DeprecatedApi::FunlockSync),
      "futime" => Ok(DeprecatedApi::Futime),
      "futimeSync" => Ok(DeprecatedApi::FutimeSync),
      "isatty" => Ok(DeprecatedApi::Isatty),
      "iter" => Ok(DeprecatedApi::Iter),
      "iterSync" => Ok(DeprecatedApi::IterSync),
      "metrics" => Ok(DeprecatedApi::Metrics),
      "read" => Ok(DeprecatedApi::Read),
      "readSync" => Ok(DeprecatedApi::ReadSync),
      "readAll" => Ok(DeprecatedApi::ReadAll),
      "readAllSync" => Ok(DeprecatedApi::ReadAllSync),
      "resources" => Ok(DeprecatedApi::Resources),
      "run" => Ok(DeprecatedApi::Run),
      "seek" => Ok(DeprecatedApi::Seek),
      "seekSync" => Ok(DeprecatedApi::SeekSync),
      "serveHttp" => Ok(DeprecatedApi::ServeHttp),
      "shutdown" => Ok(DeprecatedApi::Shutdown),
      "write" => Ok(DeprecatedApi::Write),
      "writeSync" => Ok(DeprecatedApi::WriteSync),
      "writeAll" => Ok(DeprecatedApi::WriteAll),
      "writeAllSync" => Ok(DeprecatedApi::WriteAllSync),
      _ => Err(()),
    }
  }
}
enum Replacement {
  NameAndUrl(&'static str, &'static str),
  Name(&'static str),
  Method(&'static str),
  None,
}

impl DeprecatedApi {
  fn message(&self) -> String {
    let name = self.get_deprecated_api_name();
    format!(
      "`{}` was removed in Deno 2. See the Deno 1.x to 2.x Migration Guide for further details: https://docs.deno.com/runtime/reference/migrate_deprecations/",
      name,
    )
  }

  fn hint(&self) -> Option<String> {
    match self.get_replacement() {
      Replacement::Name(name) => Some(format!("Use `{}` instead", name)),
      Replacement::NameAndUrl(name, url) => {
        Some(format!("Use `{}` from {} instead", name, url))
      }
      Replacement::Method(method) => Some(format!(
        "Use `{}` from the given class instance instead",
        method
      )),
      Replacement::None => None,
    }
  }

  fn get_deprecated_api_name(&self) -> &'static str {
    use DeprecatedApi::*;
    match *self {
      Buffer => "Deno.Buffer",
      Copy => "Deno.copy",
      Close => "Deno.close",
      CustomInspect => "Deno.customInspect",
      Fdatasync => "Deno.fdatasync",
      FdatasyncSync => "Deno.fdatasyncSync",
      File => "Deno.File",
      Flock => "Deno.flock",
      FlockSync => "Deno.flockSync",
      Fstat => "Deno.fstat",
      FstatSync => "Deno.fstatSync",
      Fsync => "Deno.fsync",
      FsyncSync => "Deno.fsyncSync",
      Ftruncate => "Deno.ftruncate",
      FtruncateSync => "Deno.ftruncateSync",
      Funlock => "Deno.funlock",
      FunlockSync => "Deno.funlockSync",
      Futime => "Deno.futime",
      FutimeSync => "Deno.futimeSync",
      Isatty => "Deno.isatty",
      Iter => "Deno.iter",
      IterSync => "Deno.iterSync",
      Metrics => "Deno.metrics",
      Read => "Deno.read",
      ReadSync => "Deno.readSync",
      ReadAll => "Deno.readAll",
      ReadAllSync => "Deno.readAllSync",
      Resources => "Deno.resources",
      Run => "Deno.run",
      Seek => "Deno.seek",
      SeekSync => "Deno.seekSync",
      ServeHttp => "Deno.serveHttp",
      Shutdown => "Deno.shutdown",
      Write => "Deno.write",
      WriteSync => "Deno.writeSync",
      WriteAll => "Deno.writeAll",
      WriteAllSync => "Deno.writeAllSync",
    }
  }

  fn get_replacement(&self) -> Replacement {
    use DeprecatedApi::*;
    use Replacement::*;
    match *self {
      Buffer => {
        NameAndUrl("Buffer", "https://jsr.io/@std/io/doc/buffer/~/Buffer")
      }
      Close => Method(".close()"),
      Copy => NameAndUrl("copy()", "https://jsr.io/@std/io/doc/copy/~/copy"),
      CustomInspect => Name("Symbol.for(\"Deno.customInspect\")"),
      Fdatasync => Method(".syncData()"),
      FdatasyncSync => Method(".syncDataSync()"),
      File => Name("Deno.FsFile"),
      Flock => Method(".lock()"),
      FlockSync => Method(".lockSync()"),
      Fstat => Method(".stat()"),
      FstatSync => Method(".statSync()"),
      Fsync => Method(".sync()"),
      FsyncSync => Method(".syncSync()"),
      Ftruncate => Method(".truncate()"),
      FtruncateSync => Method(".truncateSync()"),
      Funlock => Method(".unlock()"),
      FunlockSync => Method(".unlockSync()"),
      Futime => Method(".utime()"),
      FutimeSync => Method(".utimeSync()"),
      Isatty => Method(".isTerminal()"),
      Iter => NameAndUrl(
        "iterateReader()",
        "https://jsr.io/@std/io/doc/iterate-reader/~/iterateReader",
      ),
      IterSync => NameAndUrl(
        "iterateReaderSync()",
        "https://jsr.io/@std/io/doc/iterate-reader/~/iterateReaderSync",
      ),
      Metrics => None,
      Read => Method(".read()"),
      ReadSync => Method(".readSync()"),
      ReadAll => {
        NameAndUrl("readAll()", "https://jsr.io/@std/io/doc/read-all/~/readAll")
      }
      ReadAllSync => NameAndUrl(
        "readAllSync()",
        "https://jsr.io/@std/io/doc/read-all/~/readAllSync",
      ),
      Resources => None,
      Run => NameAndUrl("Deno.Command", "https://deno.land/api?s=Deno.Command"),
      Seek => Method(".seek()"),
      SeekSync => Method(".seekSync()"),
      ServeHttp => Name("Deno.serve()"),
      Shutdown => Method(".closeWrite()"),
      Write => Method(".write()"),
      WriteSync => Method(".writeSync()"),
      WriteAll => NameAndUrl(
        "writeAll",
        "https://jsr.io/@std/io/doc/write-all/~/writeAllSync",
      ),
      WriteAllSync => NameAndUrl(
        "writeAllSync",
        "https://jsr.io/@std/io/doc/write-all/~/writeAllSync",
      ),
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
        match deprecated_api.hint() {
          Some(hint) => {
            ctx.add_diagnostic_with_hint(
              member_expr.range(),
              CODE,
              deprecated_api.message(),
              hint,
            );
          }
          None => {
            ctx.add_diagnostic(member_expr.range(), CODE, deprecated_api.message());
          }
        }
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
          deprecated_api.hint().unwrap(),
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
      "Deno.foo.close();",
      "Deno.foo.copy();",
      "Deno.foo.customInspect;",
      "Deno.foo.fdatasync();",
      "Deno.foo.fdatasyncSync();",
      "new Deno.foo.File();",
      "Deno.foo.flock();",
      "Deno.foo.flockSync();",
      "Deno.foo.fstat();",
      "Deno.foo.fstatSync();",
      "Deno.foo.fsync();",
      "Deno.foo.fsyncSync();",
      "Deno.foo.ftruncate();",
      "Deno.foo.ftruncateSync();",
      "Deno.foo.funlock();",
      "Deno.foo.funlockSync();",
      "Deno.foo.futime();",
      "Deno.foo.futimeSync();",
      "Deno.foo.isatty();",
      "Deno.foo.iter();",
      "Deno.foo.iterSync();",
      "Deno.foo.metrics();",
      "Deno.foo.read();",
      "Deno.foo.readSync();",
      "Deno.foo.readAll();",
      "Deno.foo.readAllSync();",
      "Deno.foo.resources();",
      "Deno.foo.run();",
      "Deno.foo.seek();",
      "Deno.foo.seekSync();",
      "Deno.foo.serveHttp();",
      "Deno.foo.shutdown();",
      "Deno.foo.write();",
      "Deno.foo.writeSync();",
      "Deno.foo.writeAll();",
      "Deno.foo.writeAllSync();",
      "foo.Deno.Buffer();",
      "foo.Deno.close();",
      "foo.Deno.copy();",
      "foo.Deno.customInspect;",
      "foo.Deno.fdatasync();",
      "foo.Deno.fdatasyncSync();",
      "new foo.Deno.File();",
      "foo.Deno.flock();",
      "foo.Deno.flockSync();",
      "foo.Deno.fstat();",
      "foo.Deno.fstatSync();",
      "foo.Deno.fsync();",
      "foo.Deno.fsyncSync();",
      "foo.Deno.ftruncate();",
      "foo.Deno.ftruncateSync();",
      "foo.Deno.funlock();",
      "foo.Deno.funlockSync();",
      "foo.Deno.futime();",
      "foo.Deno.futimeSync();",
      "foo.Deno.isatty();",
      "foo.Deno.iter();",
      "foo.Deno.iterSync();",
      "foo.Deno.metrics();",
      "foo.Deno.read();",
      "foo.Deno.readSync();",
      "foo.Deno.readAll();",
      "foo.Deno.readAllSync();",
      "foo.Deno.resources();",
      "foo.Deno.run();",
      "foo.Deno.seek();",
      "foo.Deno.seekSync();",
      "foo.Deno.serveHttp();",
      "foo.Deno.shutdown();",
      "foo.Deno.write();",
      "foo.Deno.writeSync();",
      "foo.Deno.writeAll();",
      "foo.Deno.writeAllSync();",

      // `Deno` is shadowed
      "const Deno = 42; const a = new Deno.Buffer();",
      "const Deno = 42; Deno.close(closer);",
      "const Deno = 42; const a = Deno.copy(src, dst);",
      "const Deno = 42; const a = Deno.customInspect;",
      "const Deno = 42; await Deno.fdatasync(rid);",
      "const Deno = 42; Deno.fdatasyncSync(rid);",
      "const Deno = 42; const a = new Deno.File();",
      "const Deno = 42; await Deno.flock(rid, exclusive);",
      "const Deno = 42; Deno.flockSync(rid, exclusive);",
      "const Deno = 42; const a = await Deno.fstat(rid);",
      "const Deno = 42; const a = Deno.fstatSync(rid);",
      "const Deno = 42; await Deno.fsync(rid);",
      "const Deno = 42; Deno.fsyncSync(rid);",
      "const Deno = 42; await Deno.ftruncate(rid, len);",
      "const Deno = 42; Deno.ftruncateSync(rid, len);",
      "const Deno = 42; await Deno.funlock(rid);",
      "const Deno = 42; Deno.funlockSync(rid);",
      "const Deno = 42; await Deno.futime(rid, atime, mtime);",
      "const Deno = 42; Deno.futimeSync(rid, atime, mtime);",
      "const Deno = 42; const a = Deno.isatty(rid);",
      "const Deno = 42; for await (const x of Deno.iter(reader)) {}",
      "const Deno = 42; for (const x of Deno.iterSync(reader)) {}",
      "const Deno = 42; Deno.metrics();",
      "const Deno = 42; const a = await Deno.read(reader, buffer);",
      "const Deno = 42; const a = Deno.readSync(rid, buffer);",
      "const Deno = 42; const a = await Deno.readAll(reader);",
      "const Deno = 42; const a = Deno.readAllSync(reader);",
      "const Deno = 42; Deno.resources();",
      "const Deno = 42; const a = Deno.run(options);",
      "const Deno = 42; const a = await Deno.seek(rid, offset, whence);",
      "const Deno = 42; const a = Deno.seekSync(rid, offset, whence);",
      "const Deno = 42; const a = Deno.serveHttp();",
      "const Deno = 42; await Deno.shutdown(rid);",
      "const Deno = 42; const a = await Deno.write(rid, data);",
      "const Deno = 42; const a = Deno.writeSync(rid, data);",
      "const Deno = 42; await Deno.writeAll(writer, data);",
      "const Deno = 42; Deno.writeAllSync(writer, data);",
      r#"const Deno = 42; Deno.customInspect"#,
      r#"import { Deno } from "./foo.ts"; Deno.writeAllSync(writer, data);"#,

      // access property with string literal (shadowed)
      r#"const Deno = 42; const a = new Deno["Buffer"]();"#,
      r#"const Deno = 42; Deno["close"](closer);"#,
      r#"const Deno = 42; const a = Deno["copy"](src, dst);"#,
      r#"const Deno = 42; const a = Deno["customInspect"];"#,
      r#"const Deno = 42; await Deno["fdatasync"](rid);"#,
      r#"const Deno = 42; Deno["fdatasyncSync"](rid);"#,
      r#"const Deno = 42; const a = new Deno["File"]();"#,
      r#"const Deno = 42; await Deno["flock"](rid, exclusive);"#,
      r#"const Deno = 42; Deno["flockSync"](rid, exclusive);"#,
      r#"const Deno = 42; const a = await Deno["fstat"](rid);"#,
      r#"const Deno = 42; const a = Deno["fstatSync"](rid);"#,
      r#"const Deno = 42; await Deno["fsync"](rid);"#,
      r#"const Deno = 42; Deno["fsyncSync"](rid);"#,
      r#"const Deno = 42; await Deno["ftruncate"](rid, len);"#,
      r#"const Deno = 42; Deno["ftruncateSync"](rid, len);"#,
      r#"const Deno = 42; await Deno["funlock"](rid);"#,
      r#"const Deno = 42; Deno["funlockSync"](rid);"#,
      r#"const Deno = 42; await Deno["futime"](rid, atime, mtime);"#,
      r#"const Deno = 42; Deno["futimeSync"](rid, atime, mtime);"#,
      r#"const Deno = 42; const a = Deno["isatty"](rid);"#,
      r#"const Deno = 42; for await (const x of Deno["iter"](reader)) {}"#,
      r#"const Deno = 42; for (const x of Deno["iterSync"](reader)) {}"#,
      r#"const Deno = 42; Deno["metrics"]();"#,
      r#"const Deno = 42; const a = await Deno["read"](reader, buffer);"#,
      r#"const Deno = 42; const a = Deno["readSync"](rid, buffer);"#,
      r#"const Deno = 42; const a = await Deno["readAll"](reader);"#,
      r#"const Deno = 42; const a = Deno["readAllSync"](reader);"#,
      r#"const Deno = 42; Deno["resources"]();"#,
      r#"const Deno = 42; const a = Deno["run"](options);"#,
      r#"const Deno = 42; const a = await Deno["seek"](rid, offset, whence);"#,
      r#"const Deno = 42; const a = Deno["seekSync"](rid, offset, whence);"#,
      r#"const Deno = 42; const a = Deno["serveHttp"]();"#,
      r#"const Deno = 42; await Deno["shutdown"](rid);"#,
      r#"const Deno = 42; const a = await Deno["write"](rid, data);"#,
      r#"const Deno = 42; const a = Deno["writeSync"](rid, data);"#,
      r#"const Deno = 42; await Deno["writeAll"](writer, data);"#,
      r#"const Deno = 42; Deno["writeAllSync"](writer, data);"#,

      // access property with template literal (shadowed)
      r#"const Deno = 42; new Deno[`Buffer`]();"#,
      r#"const Deno = 42; Deno[`close`](closer);"#,
      r#"const Deno = 42; const a = Deno[`copy`](src, dst);"#,
      r#"const Deno = 42; const a = Deno[`customInspect`];"#,
      r#"const Deno = 42; await Deno[`fdatasync`](rid);"#,
      r#"const Deno = 42; Deno[`fdatasyncSync`](rid);"#,
      r#"const Deno = 42; const a = new Deno[`File`]();"#,
      r#"const Deno = 42; await Deno[`flock`](rid, exclusive);"#,
      r#"const Deno = 42; Deno[`flockSync`](rid, exclusive);"#,
      r#"const Deno = 42; const a = await Deno[`fstat`](rid);"#,
      r#"const Deno = 42; const a = Deno[`fstatSync`](rid);"#,
      r#"const Deno = 42; await Deno[`fsync`](rid);"#,
      r#"const Deno = 42; Deno[`fsyncSync`](rid);"#,
      r#"const Deno = 42; await Deno[`ftruncate`](rid, len);"#,
      r#"const Deno = 42; Deno[`ftruncateSync`](rid, len);"#,
      r#"const Deno = 42; await Deno[`funlock`](rid);"#,
      r#"const Deno = 42; Deno[`funlockSync`](rid);"#,
      r#"const Deno = 42; await Deno[`futime`](rid, atime, mtime);"#,
      r#"const Deno = 42; Deno[`futimeSync`](rid, atime, mtime);"#,
      r#"const Deno = 42; const a = Deno[`isatty`](rid);"#,
      r#"const Deno = 42; for await (const x of Deno[`iter`](reader)) {}"#,
      r#"const Deno = 42; for (const x of Deno[`iterSync`](reader)) {}"#,
      r#"const Deno = 42; Deno[`metrics`]();"#,
      r#"const Deno = 42; const a = await Deno[`read`](reader, buffer);"#,
      r#"const Deno = 42; const a = Deno[`readSync`](rid, buffer);"#,
      r#"const Deno = 42; const a = await Deno[`readAll`](reader);"#,
      r#"const Deno = 42; const a = Deno[`readAllSync`](reader);"#,
      r#"const Deno = 42; Deno[`resources`]();"#,
      r#"const Deno = 42; const a = Deno[`run`](options);"#,
      r#"const Deno = 42; const a = await Deno[`seek`](rid, offset, whence);"#,
      r#"const Deno = 42; const a = Deno[`seekSync`](rid, offset, whence);"#,
      r#"const Deno = 42; const a = Deno[`serveHttp`]();"#,
      r#"const Deno = 42; await Deno[`shutdown`](rid);"#,
      r#"const Deno = 42; const a = await Deno[`write`](rid, data);"#,
      r#"const Deno = 42; const a = Deno[`writeSync`](rid, data);"#,
      r#"const Deno = 42; await Deno[`writeAll`](writer, data);"#,
      r#"const Deno = 42; Deno[`writeAllSync`](writer, data);"#,

      // Ignore template literals that include expressions
      r#"const read = "read"; Deno[`${read}All`](reader);"#,

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
          hint: Buffer.hint().unwrap()
        }
      ],
      "Deno.readAll(reader);": [
        {
          col: 0,
          message: ReadAll.message(),
          hint: ReadAll.hint().unwrap()
        }
      ],
      "Deno.readAllSync(reader);": [
        {
          col: 0,
          message: ReadAllSync.message(),
          hint: ReadAllSync.hint().unwrap()
        }
      ],
      "Deno.writeAll(writer, data);": [
        {
          col: 0,
          message: WriteAll.message(),
          hint: WriteAll.hint().unwrap()
        }
      ],
      "Deno.writeAllSync(writer, data);": [
        {
          col: 0,
          message: WriteAllSync.message(),
          hint: WriteAllSync.hint().unwrap()
        }
      ],
      "Deno.iter(reader);": [
        {
          col: 0,
          message: Iter.message(),
          hint: Iter.hint().unwrap()
        }
      ],
      "Deno.iterSync(reader);": [
        {
          col: 0,
          message: IterSync.message(),
          hint: IterSync.hint().unwrap()
        }
      ],
      "Deno.copy(reader, writer);": [
        {
          col: 0,
          message: Copy.message(),
          hint: Copy.hint().unwrap()
        }
      ],
      "Deno.customInspect;": [
        {
          col: 0,
          message: CustomInspect.message(),
          hint: CustomInspect.hint().unwrap()
        }
      ],
      "Deno.File;": [
        {
          col: 0,
          message: File.message(),
          hint: File.hint().unwrap()
        }
      ],
      "let file: Deno.File;": [
        {
          col: 10,
          message: File.message(),
          hint: File.hint().unwrap()
        }
      ],
      "Deno.run(options);": [
        {
          col: 0,
          message: Run.message(),
          hint: Run.hint().unwrap()
        }
      ],
      "Deno.metrics();": [
        {
          col: 0,
          message: Metrics.message(),
        }
      ],
      "Deno.resources();": [
        {
          col: 0,
          message: Resources.message(),
        }
      ],

      // access property with string literal
      r#"new Deno["Buffer"]();"#: [
        {
          col: 4,
          message: Buffer.message(),
          hint: Buffer.hint().unwrap()
        }
      ],
      r#"Deno["readAll"](reader);"#: [
        {
          col: 0,
          message: ReadAll.message(),
          hint: ReadAll.hint().unwrap()
        }
      ],
      r#"Deno["readAllSync"](reader);"#: [
        {
          col: 0,
          message: ReadAllSync.message(),
          hint: ReadAllSync.hint().unwrap()
        }
      ],
      r#"Deno["writeAll"](writer, data);"#: [
        {
          col: 0,
          message: WriteAll.message(),
          hint: WriteAll.hint().unwrap()
        }
      ],
      r#"Deno["writeAllSync"](writer, data);"#: [
        {
          col: 0,
          message: WriteAllSync.message(),
          hint: WriteAllSync.hint().unwrap()
        }
      ],
      r#"Deno["iter"](reader);"#: [
        {
          col: 0,
          message: Iter.message(),
          hint: Iter.hint().unwrap()
        }
      ],
      r#"Deno["iterSync"](reader);"#: [
        {
          col: 0,
          message: IterSync.message(),
          hint: IterSync.hint().unwrap()
        }
      ],
      r#"Deno["copy"](reader, writer);"#: [
        {
          col: 0,
          message: Copy.message(),
          hint: Copy.hint().unwrap()
        }
      ],
      r#"Deno["customInspect"];"#: [
        {
          col: 0,
          message: CustomInspect.message(),
          hint: CustomInspect.hint().unwrap()
        }
      ],

      // access property with template literal
      r#"new Deno[`Buffer`]();"#: [
        {
          col: 4,
          message: Buffer.message(),
          hint: Buffer.hint().unwrap()
        }
      ],
      r#"Deno[`readAll`](reader);"#: [
        {
          col: 0,
          message: ReadAll.message(),
          hint: ReadAll.hint().unwrap()
        }
      ],
      r#"Deno[`readAllSync`](reader);"#: [
        {
          col: 0,
          message: ReadAllSync.message(),
          hint: ReadAllSync.hint().unwrap()
        }
      ],
      r#"Deno[`writeAll`](writer, data);"#: [
        {
          col: 0,
          message: WriteAll.message(),
          hint: WriteAll.hint().unwrap()
        }
      ],
      r#"Deno[`writeAllSync`](writer, data);"#: [
        {
          col: 0,
          message: WriteAllSync.message(),
          hint: WriteAllSync.hint().unwrap()
        }
      ],
      r#"Deno[`iter`](reader);"#: [
        {
          col: 0,
          message: Iter.message(),
          hint: Iter.hint().unwrap()
        }
      ],
      r#"Deno[`iterSync`](reader);"#: [
        {
          col: 0,
          message: IterSync.message(),
          hint: IterSync.hint().unwrap()
        }
      ],
      r#"Deno[`copy`](reader);"#: [
        {
          col: 0,
          message: Copy.message(),
          hint: Copy.hint().unwrap()
        }
      ],
      r#"Deno[`customInspect`];"#: [
        {
          col: 0,
          message: CustomInspect.message(),
          hint: CustomInspect.hint().unwrap()
        }
      ],
      r#"Deno[`File`];"#: [
        {
          col: 0,
          message: File.message(),
          hint: File.hint().unwrap()
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
          hint: ReadAll.hint().unwrap()
        }
      ],
    }
  }

  #[test]
  fn expect_deprecated_api_hint() {
    let tests = vec![
      (
        "Buffer",
        "Use `Buffer` from https://jsr.io/@std/io/doc/buffer/~/Buffer instead",
      ),
      (
        "close",
        "Use `.close()` from the given class instance instead",
      ),
      (
        "copy",
        "Use `copy()` from https://jsr.io/@std/io/doc/copy/~/copy instead",
      ),
      (
        "customInspect",
        "Use `Symbol.for(\"Deno.customInspect\")` instead",
      ),
      ("fdatasync", "Use `.syncData()` from the given class instance instead"),
      (
        "fdatasyncSync",
        "Use `.syncDataSync()` from the given class instance instead",
      ),
      ("File", "Use `Deno.FsFile` instead"),
      ("flock", "Use `.lock()` from the given class instance instead"),
      (
        "flockSync",
        "Use `.lockSync()` from the given class instance instead",
      ),
      ("fstat", "Use `.stat()` from the given class instance instead"),
      (
        "fstatSync",
        "Use `.statSync()` from the given class instance instead",
      ),
      ("fsync", "Use `.sync()` from the given class instance instead"),
      (
        "fsyncSync",
        "Use `.syncSync()` from the given class instance instead",
      ),
      (
        "ftruncate",
        "Use `.truncate()` from the given class instance instead",
      ),
      (
        "ftruncateSync",
        "Use `.truncateSync()` from the given class instance instead",
      ),
      ("funlock", "Use `.unlock()` from the given class instance instead"),
      (
        "funlockSync",
        "Use `.unlockSync()` from the given class instance instead",
      ),
      ("futime", "Use `.utime()` from the given class instance instead"),
      (
        "futimeSync",
        "Use `.utimeSync()` from the given class instance instead",
      ),
      ("isatty", "Use `.isTerminal()` from the given class instance instead"),
      (
        "iter",
        "Use `iterateReader()` from https://jsr.io/@std/io/doc/iterate-reader/~/iterateReader instead",
      ),
      (
        "iterSync",
        "Use `iterateReaderSync()` from https://jsr.io/@std/io/doc/iterate-reader/~/iterateReaderSync instead",
      ),
      ("read", "Use `.read()` from the given class instance instead"),
      ("readSync", "Use `.readSync()` from the given class instance instead"),
      (
        "readAll",
        "Use `readAll()` from https://jsr.io/@std/io/doc/read-all/~/readAll instead",
      ),
      (
        "readAllSync",
        "Use `readAllSync()` from https://jsr.io/@std/io/doc/read-all/~/readAllSync instead",
      ),
      (
        "run",
        "Use `Deno.Command` from https://deno.land/api?s=Deno.Command instead",
      ),
      ("seek", "Use `.seek()` from the given class instance instead"),
      ("seekSync", "Use `.seekSync()` from the given class instance instead"),
      (
        "serveHttp",
        "Use `Deno.serve()` instead",
      ),
      ("shutdown", "Use `.closeWrite()` from the given class instance instead"),
      ("write", "Use `.write()` from the given class instance instead"),
      (
        "writeSync",
        "Use `.writeSync()` from the given class instance instead",
      ),
      (
        "writeAll",
        "Use `writeAll` from https://jsr.io/@std/io/doc/write-all/~/writeAllSync instead",
      ),
      (
        "writeAllSync",
        "Use `writeAllSync` from https://jsr.io/@std/io/doc/write-all/~/writeAllSync instead",
      ),
    ];

    for test in tests {
      let hint = DeprecatedApi::try_from(("Deno", test.0))
        .unwrap()
        .hint()
        .unwrap();
      assert_eq!(hint, test.1);
    }
  }
}
