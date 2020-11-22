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
use std::pin::Pin;
use std::rc::Rc;
use swc_common::Span;
use swc_ecmascript::ast::Program;

#[derive(Deserialize)]
struct DiagnosticFromJS {
  span: Span,
  code: String,
  message: String,
  hint: Option<String>,
}

#[derive(Deserialize)]
struct Code {
  code: String,
}

type Diagnostics = Vec<DiagnosticFromJS>;
type Codes = HashSet<String>;

pub struct JsRuleRunner {
  runtime: JsRuntime,
  dummy_source: String,
}

impl JsRuleRunner {
  /// Create new JsRuntime for running plugin rules.
  /// If `plugin_paths` is empty, this returns `None`.
  pub fn new(plugin_paths: &[&str]) -> Option<Box<Self>> {
    if plugin_paths.is_empty() {
      return None;
    }

    let mut runner = Self {
      runtime: JsRuntime::new(RuntimeOptions {
        module_loader: Some(Rc::new(FsModuleLoader)),
        ..Default::default()
      }),
      dummy_source: create_dummy_source(plugin_paths),
    };
    runner.init();
    Some(Box::new(runner))
  }

  fn init(&mut self) {
    self
      .runtime
      .execute("visitor.js", include_str!("visitor.js"))
      .unwrap();

    self.runtime.register_op(
      "add_diagnostics",
      deno_core::json_op_sync(
        move |state: &mut OpState,
              args: Value,
              _bufs: &mut [ZeroCopyBuf]|
              -> Result<Value, AnyError> {
          let mut diagnostics_from_js: Vec<DiagnosticFromJS> =
            serde_json::from_value(args).unwrap();
          // TODO(magurotuna): To differenciate builtin and plugin rules, adds prefix to plugins.
          // This should be discussed further before it's stabilized.
          diagnostics_from_js
            .iter_mut()
            .for_each(|d| d.code = format!("@deno-lint-plugin/{}", d.code));

          let mut stored =
            state.try_take::<Diagnostics>().unwrap_or_else(Vec::new);
          stored.extend(diagnostics_from_js);
          state.put::<Diagnostics>(stored);

          Ok(serde_json::json!({}))
        },
      ),
    );

    self.runtime.register_op(
      "add_code",
      deno_core::json_op_sync(
        move |state: &mut OpState,
              args: Value,
              _bufs: &mut [ZeroCopyBuf]|
              -> Result<Value, AnyError> {
          let code_from_js: Code = serde_json::from_value(args).unwrap();
          let mut stored =
            state.try_take::<Codes>().unwrap_or_else(HashSet::new);
          // TODO(magurotuna): To differenciate builtin and plugin rules, adds prefix to plugins.
          // This should be discussed further before it's stabilized.
          stored.insert(format!("@deno-lint-plugin/{}", code_from_js.code));
          state.put::<Codes>(stored);

          Ok(serde_json::json!({}))
        },
      ),
    );
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
let plugin;
"#;
  for plugin_number in 0..plugin_paths.len() {
    dummy_source +=
      &format!("plugin = new Plugin{number}();\n", number = plugin_number);
    dummy_source += "Deno.core.jsonOpSync('add_diagnostics', plugin.collectDiagnostics(programAst));\n";
    dummy_source +=
      "Deno.core.jsonOpSync('add_code', { code: plugin.ruleCode() });\n";
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

    // TODO(magurotuna): `futures::executor::block_on` doesn't seem ideal, but works for now
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

    let diagnostics = self
      .runtime
      .op_state()
      .borrow_mut()
      .try_take::<Diagnostics>()
      .unwrap_or_else(Vec::new);
    let codes = self
      .runtime
      .op_state()
      .borrow_mut()
      .try_take::<Codes>()
      .unwrap_or_else(HashSet::new);

    diagnostics.into_iter().for_each(|d| {
      if let Some(hint) = d.hint {
        context.add_diagnostic_with_hint(d.span, d.code, d.message, hint);
      } else {
        context.add_diagnostic(d.span, d.code, d.message);
      }
    });
    context.set_plugin_codes(codes);
  }
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
let plugin;
plugin = new Plugin0();
Deno.core.jsonOpSync('add_diagnostics', plugin.collectDiagnostics(programAst));
Deno.core.jsonOpSync('add_code', { code: plugin.ruleCode() });
plugin = new Plugin1();
Deno.core.jsonOpSync('add_diagnostics', plugin.collectDiagnostics(programAst));
Deno.core.jsonOpSync('add_code', { code: plugin.ruleCode() });
"#
    );
  }
}
