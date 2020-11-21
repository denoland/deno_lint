export default class Plugin extends Visitor {
  visitImportDeclaration(e) {
    this.diagnostics.push({
      span: e.span,
      message: "foo",
      code: "some-rule-code",
    });
    return e;
  }
}
