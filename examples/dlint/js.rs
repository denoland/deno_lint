use deno_core::error::AnyError;
use deno_core::JsRuntime;
use deno_core::OpState;
use deno_core::ZeroCopyBuf;
use deno_lint::diagnostic::{LintDiagnostic, Position, Range};
use serde::Deserialize;
use serde_json::Value;
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
}

impl JsRuleRunner {
  pub fn new(source_map: Rc<SourceMap>, filename: String) -> Self {
    let mut runner = Self {
      runtime: JsRuntime::new(Default::default()),
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

  pub fn run_visitor(&mut self, program: Program) {
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
    self
      .runtime
      .execute("test_plugin.js", include_str!("test_plugin.js"))
      .unwrap();
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
