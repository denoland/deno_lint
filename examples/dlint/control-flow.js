// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
class ControlFlow {
  static isReachable(stmt) {
    const { isReachable } = Deno.core.opSync(
      "op_query_control_flow_by_span",
      {
        span: stmt.span,
      }
    );
    return isReachable;
  }

  static stopsExecution(stmt) {
    const { stopsExectuion } = Deno.core.opSync(
      "op_query_control_flow_by_span",
      {
        span: stmt.span,
      }
    );
    return stopsExectuion;
  }
}
