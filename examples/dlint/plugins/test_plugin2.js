// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
export default class LintRule extends Visitor {
  static ruleCode() {
    return "forbidden-ident-name";
  }

  visitIdentifier(n) {
    if (n.value === "forbiddenIdentifier") {
      this.addDiagnostic({
        span: n.span,
        message: "forbidden identifier name",
      });
    }
    return n;
  }
}
