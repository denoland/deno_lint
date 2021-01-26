// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
export default class LintRule extends Visitor {
  static ruleCode() {
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
