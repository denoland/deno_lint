export default class Plugin extends Visitor {
  ruleCode() {
    return "some-rule-code";
  }

  visitImportDeclaration(e) {
    this.diagnostics.push({
      span: e.span,
      message: "foo",
    });
    return e;
  }
}
