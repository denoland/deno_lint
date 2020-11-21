class Plugin extends Visitor {
  visitImportDeclaration(e) {
    this.addDiagnostic({
      filename: "test.ts",
      message: "Import found",
      code: "",
      range: {
        start: {
          line: 1,
          col: 1,
          bytePos: 0,
        },
        end: {
          line: 1,
          col: e.span.end,
          bytePos: 0,
        },
      },
    });
    return e;
  }
}
Deno.core.ops();
let programAst = Deno.core.jsonOpSync("get_program", {});
let res = new Plugin().collectDiagnostics(programAst);
Deno.core.jsonOpSync("report", res);
