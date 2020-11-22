use deno_core::error::AnyError;
use deno_core::futures::future::FutureExt;
use deno_core::JsRuntime;
use deno_core::ModuleLoader;
use deno_core::ModuleSpecifier;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use deno_core::ZeroCopyBuf;
use deno_lint::linter::{Context, Plugins};
use serde::Deserialize;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::HashSet;
use std::path::Path;
use std::pin::Pin;
use std::rc::Rc;
use swc_common::{SourceMap, Span};
use swc_ecmascript::ast::Program;

#[derive(Deserialize)]
struct DiagnosticsFromJS {
  span: Span,
  message: String,
  hint: Option<String>,
}

struct PluginMeta {
  path: String,
  code: Option<String>,
}

impl PluginMeta {
  fn new<T, U>(path: T, code: U) -> Self
  where
    T: Into<String>,
    U: Into<Option<String>>,
  {
    Self {
      path: path.into(),
      code: code.into(),
    }
  }

  fn get_code(&self) -> String {
    let raw_code = if let Some(code) = self.code.as_deref() {
      code
    } else {
      let p = Path::new(&self.path);
      p.file_stem()
        .and_then(|s| s.to_str())
        .expect("Failed to get plugin's code")
    };

    // TODO(magurotuna): To differenciate builtin and plugin rules, adds prefix to plugins.
    // This should be discussed further before it's stabilized.
    format!("@deno-lint-plugin/{}", raw_code)
  }
}

type Diagnostics = Vec<DiagnosticsFromJS>;

pub struct JsRuleRunner {
  runtime: JsRuntime,
  source_map: Rc<SourceMap>,
  plugins: Vec<PluginMeta>,
  filename: String,
  dummy_source: String,
}

impl JsRuleRunner {
  pub fn new(
    source_map: Rc<SourceMap>,
    filename: String,
    plugins: Vec<PluginMeta>,
  ) -> Self {
    let mut runner = Self {
      runtime: JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FsModuleLoader)),
        ..Default::default()
      }),
      dummy_source: create_dummy_source(&plugins),
      plugins,
      source_map,
      filename,
    };
    runner.init();
    runner
  }

  fn init(&mut self) {
    self
      .runtime
      .execute("visitor.js", include_str!("visitor.js"))
      .unwrap();

    let source_map = Rc::clone(&self.source_map);
    let filename = self.filename.clone();

    self.runtime.register_op(
      "add_diagnostics",
      deno_core::json_op_sync(
        move |state: &mut OpState,
              args: Value,
              _bufs: &mut [ZeroCopyBuf]|
              -> Result<Value, AnyError> {
          let diagnostics_from_js: Vec<DiagnosticsFromJS> =
            serde_json::from_value(args).unwrap();
          let mut stored =
            state.try_take::<Diagnostics>().unwrap_or_else(Vec::new);
          stored.extend(diagnostics_from_js);
          state.put::<Diagnostics>(stored);

          Ok(serde_json::json!({}))
        },
      ),
    );
  }

  pub async fn run_visitor(&mut self, program: Program) {
    self.runtime.register_op(
      "get_program",
      deno_core::json_op_sync(
        move |_state: &mut OpState,
              _args: Value,
              _bufs: &mut [ZeroCopyBuf]|
              -> Result<Value, AnyError> {
          Ok(serde_json::json!(program))
        },
      ),
    );

    let specifier =
      deno_core::ModuleSpecifier::resolve_url_or_path("dummy.js").unwrap();
    let module_id = self
      .runtime
      .load_module(&specifier, Some(self.dummy_source.clone()))
      .await
      .unwrap();
    self.runtime.mod_evaluate(module_id).await.unwrap();
  }

  pub fn output(mut self) -> Diagnostics {
    self
      .runtime
      .op_state()
      .borrow_mut()
      .try_take::<Diagnostics>()
      .unwrap_or_else(Vec::new)
  }
}

// TODO(magurotuna): FsModuleLoader is copied from:
// https://github.com/denoland/deno/pull/8381/files#diff-f7e2ff9248fdb8e71463e0858bfa7070680a09d9704db54d678bf86e49fce3e4
// This feature is going to be added to `deno_core`, then we should delegate to it.
struct FsModuleLoader;

impl ModuleLoader for FsModuleLoader {
  fn resolve(
    &self,
    _op_state: Rc<RefCell<OpState>>,
    specifier: &str,
    referrer: &str,
    _is_main: bool,
  ) -> Result<ModuleSpecifier, AnyError> {
    Ok(ModuleSpecifier::resolve_import(specifier, referrer)?)
  }

  fn load(
    &self,
    _op_state: Rc<RefCell<OpState>>,
    module_specifier: &ModuleSpecifier,
    _maybe_referrer: Option<ModuleSpecifier>,
    _is_dynamic: bool,
  ) -> Pin<Box<deno_core::ModuleSourceFuture>> {
    let module_specifier = module_specifier.clone();
    async move {
      let path = module_specifier.as_url().to_file_path().unwrap();
      let content = std::fs::read_to_string(path)?;
      let module = deno_core::ModuleSource {
        code: content,
        module_url_specified: module_specifier.to_string(),
        module_url_found: module_specifier.to_string(),
      };
      Ok(module)
    }
    .boxed_local()
  }
}

fn create_dummy_source(plugins: &[PluginMeta]) -> String {
  let mut dummy_source = String::new();
  for (i, p) in plugins.iter().map(|p| &p.path).enumerate() {
    dummy_source += &format!(
      "import Plugin{number} from '{path}';\n",
      number = i,
      path = p
    );
  }
  dummy_source += r#"Deno.core.ops();
const programAst = Deno.core.jsonOpSync('get_program', {});
let res;
"#;
  for plugin_number in 0..plugins.len() {
    dummy_source += &format!(
      "res = new Plugin{number}().collectDiagnostics(programAst);\n",
      number = plugin_number
    );
    dummy_source += "Deno.core.jsonOpSync('add_diagnostics', res);\n";
  }

  dummy_source
}

impl Plugins for JsRuleRunner {
  fn run(&mut self, context: &mut Context, program: Program) {
    self.runtime.register_op(
      "get_program",
      deno_core::json_op_sync(
        move |_state: &mut OpState,
              _args: Value,
              _bufs: &mut [ZeroCopyBuf]|
              -> Result<Value, AnyError> {
          Ok(serde_json::json!(program))
        },
      ),
    );

    let specifier = ModuleSpecifier::resolve_url_or_path("dummy.js").unwrap();
    let module_id = deno_core::futures::executor::block_on(
      self
        .runtime
        .load_module(&specifier, Some(self.dummy_source.clone())),
    )
    .unwrap();
    deno_core::futures::executor::block_on(
      self.runtime.mod_evaluate(module_id),
    )
    .unwrap();
  }

  fn codes(&self) -> HashSet<String> {
    self.plugins.iter().map(|p| p.get_code()).collect()
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_create_dummy_source() {
    let input = [
      PluginMeta::new("./foo.ts", None),
      PluginMeta::new("../bar.js", None),
    ];
    assert_eq!(
      create_dummy_source(&input),
      r#"import Plugin0 from './foo.ts';
import Plugin1 from '../bar.js';
Deno.core.ops();
const programAst = Deno.core.jsonOpSync('get_program', {});
let res;
res = new Plugin0().collectDiagnostics(programAst);
Deno.core.jsonOpSync('add_diagnostics', res);
res = new Plugin1().collectDiagnostics(programAst);
Deno.core.jsonOpSync('add_diagnostics', res);
"#
    );
  }
}
