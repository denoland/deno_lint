class Plugin extends Visitor {
    visitImportDeclaration(e) {
      this.addDiagnostic(e);
      return e;
    }
}
Deno.core.ops();
let mod = Deno.core.jsonOpSync('module', {});
let res = new Plugin().collectDiagnostics(mod);
Deno.core.jsonOpSync('report', res);
