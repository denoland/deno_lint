export default class Plugin extends Visitor {
  ruleCode() {
    return "some-rule-code";
  }

  visitImportDeclaration(e) {
    this.addDiagnostic({
      span: e.span,
      message: "foo",
    });
    return e;
  }
}
