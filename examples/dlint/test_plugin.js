class Plugin extends Visitor {
    visitImportDeclaration(e) {
      Deno.core.print(e);
      this.addDiagnostic("x");
      return e;
    }
}

let mod = Deno.core.jsonOpSync('module', {});
console.log(mod)
new Plugin().collectDiagnostics(mod);