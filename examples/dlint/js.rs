// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use anyhow::Context as _;
use deno_core::error::AnyError;
use deno_core::resolve_url_or_path;
use deno_core::FsModuleLoader;
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use deno_core::ZeroCopyBuf;
use deno_lint::context::Context;
use deno_lint::control_flow::ControlFlow;
use deno_lint::linter::Plugin;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use swc_common::Span;
use swc_ecmascript::ast::Program;

#[derive(Deserialize)]
struct DiagnosticsFromJs {
  code: String,
  diagnostics: Vec<InnerDiagnostics>,
}

#[derive(Deserialize)]
struct InnerDiagnostics {
  span: Span,
  message: String,
  hint: Option<String>,
}

#[derive(Deserialize)]
struct Code {
  code: String,
}

type Diagnostics = HashMap<String, Vec<InnerDiagnostics>>;
type Codes = HashSet<String>;

#[allow(clippy::unnecessary_wraps)]
fn op_add_diagnostics(
  state: &mut OpState,
  args: Value,
  _maybe_buf: Option<ZeroCopyBuf>,
) -> anyhow::Result<Value> {
  let DiagnosticsFromJs { code, diagnostics } =
    serde_json::from_value(args).unwrap();

  let mut stored = state.try_take::<Diagnostics>().unwrap_or_else(HashMap::new);
  // TODO(magurotuna): should add some prefix to `code` to prevent from conflicting with builtin
  // rules
  stored.insert(code, diagnostics);
  state.put::<Diagnostics>(stored);

  Ok(serde_json::json!({}))
}

#[allow(clippy::unnecessary_wraps)]
fn op_add_rule_code(
  state: &mut OpState,
  args: Value,
  _maybe_buf: Option<ZeroCopyBuf>,
) -> Result<Value, AnyError> {
  let code_from_js: Code = serde_json::from_value(args).unwrap();

  let mut stored = state.try_take::<Codes>().unwrap_or_else(HashSet::new);
  stored.insert(code_from_js.code);
  state.put::<Codes>(stored);

  Ok(serde_json::json!({}))
}

fn op_query_control_flow_by_span(
  state: &mut OpState,
  args: Value,
  _maybe_buf: Option<ZeroCopyBuf>,
) -> Result<Value, AnyError> {
  let control_flow = state
    .try_borrow::<ControlFlow>()
    .context("ControlFlow is not set")?;

  #[derive(Deserialize)]
  struct SpanFromJs {
    span: Span,
  }
  let span_from_js: SpanFromJs = serde_json::from_value(args).unwrap();
  let meta = control_flow.meta(span_from_js.span.lo());

  let is_reachable = meta.map(|m| !m.unreachable);
  let stops_execution = meta.map(|m| m.stops_execution());

  #[derive(Serialize)]
  #[serde(rename_all = "camelCase")]
  struct ReturnValue {
    is_reachable: Option<bool>,
    stops_execution: Option<bool>,
  }
  serde_json::to_value(ReturnValue {
    is_reachable,
    stops_execution,
  })
  .map_err(Into::into)
}

pub struct JsRuleRunner {
  runtime: JsRuntime,
  module_id: i32,
}

impl JsRuleRunner {
  /// Create new JsRuntime for running plugin rules.
  pub fn new(plugin_path: &str) -> Box<Self> {
    let mut runtime = JsRuntime::new(RuntimeOptions {
      module_loader: Some(Rc::new(FsModuleLoader)),
      ..Default::default()
    });

    runtime
      .execute_script("visitor.js", include_str!("visitor.js"))
      .unwrap();
    runtime
      .execute_script("control-flow.js", include_str!("control-flow.js"))
      .unwrap();
    runtime.register_op(
      "op_add_diagnostics",
      deno_core::op_sync(op_add_diagnostics),
    );
    runtime
      .register_op("op_add_rule_code", deno_core::op_sync(op_add_rule_code));
    runtime.register_op(
      "op_query_control_flow_by_span",
      deno_core::op_sync(op_query_control_flow_by_span),
    );
    runtime.sync_ops_cache();

    let module_id =
      deno_core::futures::executor::block_on(runtime.load_module(
        &resolve_url_or_path("dummy.js").unwrap(),
        Some(create_dummy_source(plugin_path)),
      ))
      .unwrap();

    Box::new(Self { runtime, module_id })
  }
}

impl Plugin for JsRuleRunner {
  fn run(
    &mut self,
    context: &mut Context,
    program: Program,
  ) -> Result<(), AnyError> {
    self
      .runtime
      .op_state()
      .borrow_mut()
      .put(context.control_flow().clone());

    let _ = self.runtime.mod_evaluate(self.module_id);
    deno_core::futures::executor::block_on(self.runtime.run_event_loop(false))
      .unwrap();

    let codes = self
      .runtime
      .op_state()
      .borrow_mut()
      .try_take::<Codes>()
      .unwrap_or_else(HashSet::new);

    context.set_plugin_codes(codes.clone());

    self.runtime.execute_script(
      "runPlugins",
      &format!(
        "runPlugins({ast}, {rule_codes});",
        ast = serde_json::to_string(&program).unwrap(),
        rule_codes = serde_json::to_string(&codes).unwrap()
      ),
    )?;

    let diagnostic_map = self
      .runtime
      .op_state()
      .borrow_mut()
      .try_take::<Diagnostics>();

    if let Some(diagnostic_map) = diagnostic_map {
      for (code, diagnostics) in diagnostic_map {
        for d in diagnostics {
          if let Some(hint) = d.hint {
            context.add_diagnostic_with_hint(d.span, &code, d.message, hint);
          } else {
            context.add_diagnostic(d.span, &code, d.message);
          }
        }
      }
    }

    Ok(())
  }
}

fn create_dummy_source(plugin_path: &str) -> String {
  let mut dummy_source = String::new();
  dummy_source += &format!("import Plugin from '{}';\n", plugin_path);
  dummy_source += r#"Deno.core.ops();
const rules = new Map();
function registerRule(ruleClass) {
  const code = ruleClass.ruleCode();
  rules.set(code, ruleClass);
  Deno.core.opSync('op_add_rule_code', { code });
}
globalThis.runPlugins = function(programAst, ruleCodes) {
  for (const code of ruleCodes) {
    const rule = rules.get(code);
    if (rule === undefined) {
      continue;
    }
    const diagnostics = new rule().collectDiagnostics(programAst);
    Deno.core.opSync('op_add_diagnostics', { code, diagnostics });
  }
};
registerRule(Plugin);
"#;

  dummy_source
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_create_dummy_source() {
    assert_eq!(
      create_dummy_source("./foo.ts"),
      r#"import Plugin from './foo.ts';
Deno.core.ops();
const rules = new Map();
function registerRule(ruleClass) {
  const code = ruleClass.ruleCode();
  rules.set(code, ruleClass);
  Deno.core.opSync('op_add_rule_code', { code });
}
globalThis.runPlugins = function(programAst, ruleCodes) {
  for (const code of ruleCodes) {
    const rule = rules.get(code);
    if (rule === undefined) {
      continue;
    }
    const diagnostics = new rule().collectDiagnostics(programAst);
    Deno.core.opSync('op_add_diagnostics', { code, diagnostics });
  }
};
registerRule(Plugin);
"#
    );
  }
}
