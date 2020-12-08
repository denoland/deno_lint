class ControlFlow {
  static isReachable(stmt) {
    const { isReachable } = Deno.core.jsonOpSync("query_control_flow_by_span", {
      span: stmt.span,
    });
    return isReachable;
  }

  static stopsExecution(stmt) {
    const { stopsExectuion } = Deno.core.jsonOpSync(
      "query_control_flow_by_span",
      {
        span: stmt.span,
      }
    );
    return stopsExectuion;
  }
}
