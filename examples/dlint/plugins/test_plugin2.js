export default class Plugin extends Visitor {
  ruleCode() {
    return "forbidden-ident-name";
  }

  visitIdentifier(n) {
    if (n.value === "forbiddenIdentifier") {
      this.diagnostics.push({
        span: n.span,
        message: "forbidden identifier name",
      });
    }
    return n;
  }
}
