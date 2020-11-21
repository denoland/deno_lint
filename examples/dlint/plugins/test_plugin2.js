export default class Plugin extends Visitor {
  visitIdentifier(n) {
    if (n.value === "forbiddenIdentifier") {
      this.diagnostics.push({
        span: n.span,
        message: "forbidden identifier name",
        code: "forbidden-ident-name",
      });
    }
    return n;
  }
}
