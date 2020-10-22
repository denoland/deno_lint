use deno_core::error::AnyError;
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;
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
      |_state: &mut OpState,
       args: Value,
       _bufs: &mut [ZeroCopyBuf]|
       -> Result<Value, AnyError> {
        println!("{}", args);
        Ok(serde_json::json!({}))
      },
    ),
  );
}

pub fn run_visitor(
  module: swc_ecmascript::ast::Module,
  js_runtime: &mut JsRuntime,
) {
  js_runtime.register_op(
    "module",
    deno_core::json_op_sync(
      move |_state: &mut OpState,
            _args: Value,
            _bufs: &mut [ZeroCopyBuf]|
            -> Result<Value, AnyError> { Ok(serde_json::json!(module)) },
    ),
  );
  js_runtime
    .execute("plugin.js", include_str!("test_plugin.js"))
    .unwrap();
}
