class Plugin extends Visitor {
  visitImportDeclaration(e) {
    this.diagnostics.push({
      span: e.span,
      message: "foo",
      code: "some-rule-code",
    });
    return e;
  }
}

Deno.core.ops();
let programAst = Deno.core.jsonOpSync("get_program", {});
let res = new Plugin().collectDiagnostics(programAst);
Deno.core.jsonOpSync("add_diagnostics", res);
