class Plugin extends Visitor {
    visitImportDeclaration(e) {
      Deno.core.print(e);
      this.addDiagnostic("x");
      return e;
    }
}
  