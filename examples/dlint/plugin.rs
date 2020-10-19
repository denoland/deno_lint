// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use deno_lint::linter::Context;
use deno_lint::rules::LintRule;
use swc_ecmascript::ast::ImportDecl;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;

pub struct WarnRawGitImport;

impl LintRule for WarnRawGitImport {
  fn new() -> Box<Self> {
    Box::new(WarnRawGitImport)
  }

  fn code(&self) -> &'static str {
    "no-raw-git-import"
  }

  fn lint_module(
    &self,
    context: &mut Context,
    module: &swc_ecmascript::ast::Module,
  ) {
    let mut visitor = WarnRawGitImportVisitor::new(context);
    visitor.visit_module(module, module);
  }
}

struct WarnRawGitImportVisitor<'c> {
  context: &'c mut Context,
}

impl<'c> WarnRawGitImportVisitor<'c> {
  fn new(context: &'c mut Context) -> Self {
    Self { context }
  }
}

impl<'c> Visit for WarnRawGitImportVisitor<'c> {
  fn visit_import_decl(
    &mut self,
    import_decl: &ImportDecl,
    _parent: &dyn Node,
  ) {
    let src_str = import_decl.src.value.to_string();
    // This is a demonstration so we'll avoid using any extra dependencies to match import URL
    if src_str.starts_with("https://raw.githubusercontent.com/") {
      self.context.add_diagnostic(
        import_decl.span,
        "no-raw-git-import",
        "Importing from raw git is not allowed.",
      );
    }
  }
}
