use deno_core::error::AnyError;
use deno_core::futures::future::FutureExt;
use deno_core::JsRuntime;
use deno_core::ModuleLoader;
use deno_core::ModuleSpecifier;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use deno_core::ZeroCopyBuf;
use deno_lint::diagnostic::{LintDiagnostic, Position, Range};
use serde::Deserialize;
use serde_json::Value;
use std::cell::RefCell;
use std::pin::Pin;
use std::rc::Rc;
use swc_common::{SourceMap, Span};
use swc_ecmascript::ast::Program;

#[derive(Deserialize)]
struct DiagnosticsFromJS {
  span: Span,
  code: String,
  message: String,
  hint: Option<String>,
}

type Diagnostics = Vec<LintDiagnostic>;

pub struct JsRuleRunner {
  runtime: JsRuntime,
  source_map: Rc<SourceMap>,
  filename: String,
  dummy_source: String,
}

impl JsRuleRunner {
  pub fn new(
    source_map: Rc<SourceMap>,
    filename: String,
    plugin_paths: &[&str],
  ) -> Self {
    let mut runner = Self {
      runtime: JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FsModuleLoader)),
        ..Default::default()
      }),
      source_map,
      filename,
      dummy_source: create_dummy_source(plugin_paths),
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
          let converted = diagnostics_from_js.into_iter().map(|d| {
            let start = Position::new(
              source_map.lookup_byte_offset(d.span.lo()).pos,
              source_map.lookup_char_pos(d.span.lo()),
            );
            let end = Position::new(
              source_map.lookup_byte_offset(d.span.hi()).pos,
              source_map.lookup_char_pos(d.span.hi()),
            );

            LintDiagnostic {
              range: Range { start, end },
              filename: filename.clone(),
              message: d.message,
              code: d.code,
              hint: d.hint,
            }
          });

          let mut stored =
            state.try_take::<Diagnostics>().unwrap_or_else(Vec::new);
          stored.extend(converted);
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
// This feature is going to be added to `deno_core`, then we should delegate  to it.
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

fn create_dummy_source(plugin_paths: &[&str]) -> String {
  let mut dummy_source = String::new();
  for (i, p) in plugin_paths.iter().enumerate() {
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
  for plugin_number in 0..plugin_paths.len() {
    dummy_source += &format!(
      "res = new Plugin{number}().collectDiagnostics(programAst);\n",
      number = plugin_number
    );
    dummy_source += "Deno.core.jsonOpSync('add_diagnostics', res);\n";
  }

  dummy_source
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_create_dummy_source() {
    let input = ["./foo.ts", "../bar.js"];
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
