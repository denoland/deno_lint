use deno_core::error::AnyError;
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;
use deno_lint::diagnostic::LintDiagnostic;
use serde_json::Value;

pub fn create_js_runtime() -> JsRuntime {
  let mut runtime = JsRuntime::new(Default::default());
  prepare(&mut runtime);
  runtime
}

fn prepare(js_runtime: &mut JsRuntime) {
  js_runtime
    .execute("visitor.js", include_str!("visitor.js"))
    .unwrap();
  js_runtime.register_op(
    "report",
    deno_core::json_op_sync(
      move |state: &mut OpState,
            args: Value,
            _bufs: &mut [ZeroCopyBuf]|
            -> Result<Value, AnyError> {
        let diagnostics: Vec<LintDiagnostic> =
          serde_json::from_value(args).unwrap();
        state.put::<Vec<LintDiagnostic>>(diagnostics);
        Ok(serde_json::json!({}))
      },
    ),
  );
}

pub fn run_visitor(
  program: swc_ecmascript::ast::Program,
  js_runtime: &mut JsRuntime,
) -> Vec<LintDiagnostic> {
  js_runtime.register_op(
    "get_program",
    deno_core::json_op_sync(
      move |_state: &mut OpState,
            _args: Value,
            _bufs: &mut [ZeroCopyBuf]|
            -> Result<Value, AnyError> { Ok(serde_json::json!(program)) },
    ),
  );
  js_runtime
    .execute("plugin.js", include_str!("test_plugin.js"))
    .unwrap();
  js_runtime
    .op_state()
    .borrow_mut()
    .take::<Vec<LintDiagnostic>>()
}
