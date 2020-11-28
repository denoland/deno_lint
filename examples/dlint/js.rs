use deno_core::error::AnyError;
use deno_core::futures::future::FutureExt;
use deno_core::JsRuntime;
use deno_core::ModuleLoader;
use deno_core::ModuleSpecifier;
use deno_core::OpState;
use deno_core::RuntimeOptions;
use deno_core::ZeroCopyBuf;
use deno_lint::linter::{Context, Plugin};
use serde::Deserialize;
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::pin::Pin;
use std::rc::Rc;
use swc_common::Span;
use swc_ecmascript::ast::Program;

#[derive(Deserialize)]
struct DiagnosticsFromJS {
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

fn op_add_diagnostics(
  state: &mut OpState,
  args: Value,
  _bufs: &mut [ZeroCopyBuf],
) -> Result<Value, AnyError> {
  let DiagnosticsFromJS { code, diagnostics } =
    serde_json::from_value(args).unwrap();

  let mut stored = state.try_take::<Diagnostics>().unwrap_or_else(HashMap::new);
  // TODO(magurotuna): should add some prefix to `code` to prevent from conflicting with builtin
  // rules
  stored.insert(code, diagnostics);
  state.put::<Diagnostics>(stored);

  Ok(serde_json::json!({}))
}

fn op_add_rule_code(
  state: &mut OpState,
  args: Value,
  _bufs: &mut [ZeroCopyBuf],
) -> Result<Value, AnyError> {
  let code_from_js: Code = serde_json::from_value(args).unwrap();

  let mut stored = state.try_take::<Codes>().unwrap_or_else(HashSet::new);
  stored.insert(code_from_js.code);
  state.put::<Codes>(stored);

  Ok(serde_json::json!({}))
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
      .execute("visitor.js", include_str!("visitor.js"))
      .unwrap();
    runtime.register_op(
      "add_diagnostics",
      deno_core::json_op_sync(op_add_diagnostics),
    );
    runtime
      .register_op("add_rule_code", deno_core::json_op_sync(op_add_rule_code));

    // TODO(magurotuna): `futures::executor::block_on` doesn't seem ideal, but works for now
    let module_id =
      deno_core::futures::executor::block_on(runtime.load_module(
        &ModuleSpecifier::resolve_url_or_path("dummy.js").unwrap(),
        Some(create_dummy_source(plugin_path)),
      ))
      .unwrap();

    Box::new(Self { runtime, module_id })
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

impl Plugin for JsRuleRunner {
  // TODO(magurotuna): this method sometimes panics, so maybe should return `Result`?
  fn run(&mut self, context: &mut Context, program: Program) {
    // TODO(magurotuna): `futures::executor::block_on` doesn't seem ideal, but works for now
    deno_core::futures::executor::block_on(
      self.runtime.mod_evaluate(self.module_id),
    )
    .unwrap();

    let codes = self
      .runtime
      .op_state()
      .borrow_mut()
      .try_take::<Codes>()
      .unwrap_or_else(HashSet::new);

    context.set_plugin_codes(codes.clone());

    self
      .runtime
      .execute(
        "runPlugins",
        &format!(
          "runPlugins({ast}, {rule_codes});",
          ast = serde_json::to_string(&program).unwrap(),
          rule_codes = serde_json::to_string(&codes).unwrap()
        ),
      )
      .unwrap();

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
  Deno.core.jsonOpSync('add_rule_code', { code });
}
globalThis.runPlugins = function(programAst, ruleCodes) {
  for (const code of ruleCodes) {
    const rule = rules.get(code);
    if (rule === undefined) {
      continue;
    }
    const diagnostics = new rule().collectDiagnostics(programAst);
    Deno.core.jsonOpSync('add_diagnostics', { code, diagnostics });
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
  Deno.core.jsonOpSync('add_rule_code', { code });
}
globalThis.runPlugins = function(programAst, ruleCodes) {
  for (const code of ruleCodes) {
    const rule = rules.get(code);
    if (rule === undefined) {
      continue;
    }
    const diagnostics = new rule().collectDiagnostics(programAst);
    Deno.core.jsonOpSync('add_diagnostics', { code, diagnostics });
  }
};
registerRule(Plugin);
"#
    );
  }
}
