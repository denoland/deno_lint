// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use crate::control_flow::ControlFlow;
use crate::diagnostic::{
  LintDiagnostic, LintDiagnosticDetails, LintDiagnosticRange, LintDocsUrl,
  LintFix,
};
use crate::ignore_directives::{
  parse_line_ignore_directives, CodeStatus, FileIgnoreDirective,
  LineIgnoreDirective,
};
use crate::linter::{LintConfig, LinterContext};
use crate::rules;
use deno_ast::oxc::ast::ast::Comment;
use deno_ast::oxc::ast::ast::Program;
use deno_ast::oxc::semantic::{Scoping, SemanticBuilder};
use deno_ast::oxc::span::Span;
use deno_ast::ParsedSource;
use deno_ast::Scope;
use deno_ast::SourceTextInfo;
use deno_ast::{MediaType, ModuleSpecifier};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

/// `Context` stores all data needed to perform linting of a particular file.
pub struct Context<'a> {
  parsed_source: &'a ParsedSource<'a>,
  diagnostics: Vec<LintDiagnostic>,
  file_ignore_directive: Option<FileIgnoreDirective>,
  line_ignore_directives: HashMap<usize, LineIgnoreDirective>,
  scope: Scope,
  control_flow: ControlFlow,
  traverse_flow: TraverseFlow,
  check_unknown_rules: bool,
  /// OXC semantic scoping information (symbol table + scope tree).
  /// Populated by running SemanticBuilder on the program.
  scoping: Scoping,
  /// The resolved JSX factory name (e.g. "React.createElement").
  /// Determined from @jsx pragma comment or config default.
  jsx_factory: Option<String>,
  /// The resolved JSX fragment factory name (e.g. "React.Fragment").
  /// Determined from @jsxFrag pragma comment or config default.
  jsx_fragment_factory: Option<String>,
}

impl<'a> Context<'a> {
  pub(crate) fn new(
    linter_ctx: &LinterContext,
    parsed_source: &'a ParsedSource<'a>,
    config: &LintConfig,
    file_ignore_directive: Option<FileIgnoreDirective>,
  ) -> Self {
    let line_ignore_directives = parse_line_ignore_directives(
      linter_ctx.ignore_diagnostic_directive,
      parsed_source,
    );
    let scope = Scope::analyze(parsed_source.program());
    let control_flow = ControlFlow::analyze(parsed_source);

    // Run OXC semantic analysis to populate symbol_id and reference_id
    // on AST nodes (BindingIdentifier and IdentifierReference).
    let semantic_ret = SemanticBuilder::new().build(parsed_source.program());
    let scoping = semantic_ret.semantic.into_scoping();

    // Resolve JSX factory from pragma comments, falling back to config defaults.
    let (jsx_factory, jsx_fragment_factory) =
      resolve_jsx_pragmas(parsed_source, config);

    Self {
      file_ignore_directive,
      line_ignore_directives,
      scope,
      control_flow,
      parsed_source,
      diagnostics: Vec::new(),
      traverse_flow: TraverseFlow::default(),
      check_unknown_rules: linter_ctx.check_unknown_rules,
      scoping,
      jsx_factory,
      jsx_fragment_factory,
    }
  }

  /// File specifier on which the lint rule is run.
  pub fn specifier(&self) -> &ModuleSpecifier {
    self.parsed_source.specifier()
  }

  /// The media type which linter was configured with. Can be used
  /// to skip checking some rules.
  pub fn media_type(&self) -> MediaType {
    self.parsed_source.media_type()
  }

  /// The source text of the file.
  pub fn source_text(&self) -> &'a str {
    self.parsed_source.text()
  }

  /// Stores diagnostics that are generated while linting
  pub fn diagnostics(&self) -> &[LintDiagnostic] {
    &self.diagnostics
  }

  /// Parsed source of the program.
  pub fn parsed_source(&self) -> &'a ParsedSource<'a> {
    self.parsed_source
  }

  /// Information about the file text.
  pub fn text_info(&self) -> &SourceTextInfo {
    self.parsed_source.text_info_lazy()
  }

  /// The AST program.
  pub fn program(&self) -> &'a Program<'a> {
    self.parsed_source.program()
  }

  /// File-level ignore directive (`deno-lint-ignore-file`)
  pub fn file_ignore_directive(&self) -> Option<&FileIgnoreDirective> {
    self.file_ignore_directive.as_ref()
  }

  /// The map that stores line-level ignore directives (`deno-lint-ignore`).
  /// The key of the map is line number.
  pub fn line_ignore_directives(&self) -> &HashMap<usize, LineIgnoreDirective> {
    &self.line_ignore_directives
  }

  /// Scope analysis result
  pub fn scope(&self) -> &Scope {
    &self.scope
  }

  /// OXC semantic scoping info (symbol table + scope tree + references).
  pub fn scoping(&self) -> &Scoping {
    &self.scoping
  }

  /// Resolve an `IdentifierReference` to its declaration's `BindingKind`
  /// using OXC's semantic analysis. Returns `None` if unresolved (global).
  pub fn binding_kind_of_ident_ref(
    &self,
    ident: &deno_ast::oxc::ast::ast::IdentifierReference,
  ) -> Option<deno_ast::BindingKind> {
    let ref_id = ident.reference_id.get()?;
    let reference = self.scoping.get_reference(ref_id);
    let symbol_id = reference.symbol_id()?;
    let flags = self.scoping.symbol_flags(symbol_id);
    Some(deno_ast::BindingKind::from_symbol_flags(flags))
  }

  /// The resolved JSX factory expression (e.g. "React.createElement").
  pub fn jsx_factory(&self) -> Option<&str> {
    self.jsx_factory.as_deref()
  }

  /// The resolved JSX fragment factory expression (e.g. "React.Fragment").
  pub fn jsx_fragment_factory(&self) -> Option<&str> {
    self.jsx_fragment_factory.as_deref()
  }

  /// Control-flow analysis result
  pub fn control_flow(&self) -> &ControlFlow {
    &self.control_flow
  }

  /// Get all comments in the source file.
  pub fn all_comments(&self) -> impl Iterator<Item = &'a Comment> {
    self.parsed_source.comments().iter()
  }

  /// Get leading comments before a given byte position.
  pub fn leading_comments_at(
    &self,
    pos: u32,
  ) -> impl Iterator<Item = &'a Comment> {
    self
      .parsed_source
      .comments()
      .iter()
      .filter(move |c| c.span.end <= pos)
  }

  /// Get the text content of a comment (excluding the // or /* */ delimiters).
  pub fn comment_text(&self, comment: &Comment) -> &str {
    let span = comment.content_span();
    &self.source_text()[span.start as usize..span.end as usize]
  }

  pub(crate) fn assert_traverse_init(&self) {
    self.traverse_flow.assert_init();
  }

  pub(crate) fn should_stop_traverse(&mut self) -> bool {
    self.traverse_flow.should_stop()
  }

  pub(crate) fn stop_traverse(&mut self) {
    self.traverse_flow.set_stop_traverse();
  }

  /// Mark ignore directives as used if that directive actually suppresses some
  /// diagnostic, and return a list of diagnostics that are not ignored.
  /// Make sure that this method is called after all lint rules have been
  /// executed.
  pub(crate) fn check_ignore_directive_usage(&mut self) -> Vec<LintDiagnostic> {
    let mut filtered = Vec::new();

    for diagnostic in self.diagnostics.iter().cloned() {
      if let Some(f) = self.file_ignore_directive.as_mut() {
        if f.check_used(&diagnostic.details.code) {
          continue;
        }
      }
      let Some(range) = diagnostic.range.as_ref() else {
        continue;
      };

      let diagnostic_line =
        range.text_info.line_index(range.range.start as usize);
      if diagnostic_line > 0 {
        if let Some(l) =
          self.line_ignore_directives.get_mut(&(diagnostic_line - 1))
        {
          if l.check_used(&diagnostic.details.code) {
            continue;
          }
        }
      }

      filtered.push(diagnostic);
    }

    filtered
  }

  /// Lint rule implementation for `ban-unused-ignore`.
  /// This should be run after all normal rules have been finished because this
  /// works for diagnostics reported by other rules.
  pub(crate) fn ban_unused_ignore(
    &self,
    known_rules_codes: &HashSet<Cow<'static, str>>,
  ) -> Vec<LintDiagnostic> {
    const CODE: &str = "ban-unused-ignore";

    // If there's a file-level ignore directive containing `ban-unused-ignore`,
    // exit without running this rule.
    if self
      .file_ignore_directive
      .as_ref()
      .is_some_and(|file_ignore| file_ignore.has_code(CODE))
    {
      return vec![];
    }

    let is_unused_code = |&(code, status): &(&String, &CodeStatus)| {
      let is_unknown = !known_rules_codes.contains(code.as_str());
      !status.used && !is_unknown
    };

    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for (unused_code, _status) in
        file_ignore.codes().iter().filter(is_unused_code)
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(file_ignore.range())),
          self.create_diagnostic_details(
            CODE,
            format!("Ignore for code \"{}\" was not used.", unused_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    for line_ignore in self.line_ignore_directives.values() {
      for (unused_code, _status) in
        line_ignore.codes().iter().filter(is_unused_code)
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(line_ignore.range())),
          self.create_diagnostic_details(
            CODE,
            format!("Ignore for code \"{}\" was not used.", unused_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    diagnostics
  }

  // TODO(bartlomieju): this should be a regular lint rule, not a method on this
  // struct.
  /// Lint rule implementation for `ban-unknown-rule-code`.
  /// This should be run after all normal rules.
  pub(crate) fn ban_unknown_rule_code(
    &mut self,
    enabled_rules: &HashSet<Cow<'static, str>>,
  ) -> Vec<LintDiagnostic> {
    let mut diagnostics = Vec::new();

    if let Some(file_ignore) = self.file_ignore_directive.as_ref() {
      for unknown_rule_code in file_ignore
        .codes()
        .keys()
        .filter(|code| !enabled_rules.contains(code.as_str()))
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(file_ignore.range())),
          self.create_diagnostic_details(
            rules::ban_unknown_rule_code::CODE,
            format!("Unknown rule for code \"{}\"", unknown_rule_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    for line_ignore in self.line_ignore_directives.values() {
      for unknown_rule_code in line_ignore
        .codes()
        .keys()
        .filter(|code| !enabled_rules.contains(code.as_str()))
      {
        let d = self.create_diagnostic(
          Some(self.create_diagnostic_range(line_ignore.range())),
          self.create_diagnostic_details(
            rules::ban_unknown_rule_code::CODE,
            format!("Unknown rule for code \"{}\"", unknown_rule_code),
            None,
            Vec::new(),
          ),
        );
        diagnostics.push(d);
      }
    }

    if !diagnostics.is_empty() {
      if let Some(f) = self.file_ignore_directive.as_mut() {
        f.check_used(rules::ban_unknown_rule_code::CODE);
      }
    }

    if self.check_unknown_rules
      && !self
        .file_ignore_directive()
        .map(|f| f.has_code(rules::ban_unknown_rule_code::CODE))
        .unwrap_or(false)
    {
      diagnostics
    } else {
      vec![]
    }
  }

  pub fn add_diagnostic(
    &mut self,
    span: Span,
    code: impl ToString,
    message: impl ToString,
  ) {
    self.add_diagnostic_details(
      Some(self.create_diagnostic_range(span)),
      self.create_diagnostic_details(
        code,
        message.to_string(),
        None,
        Vec::new(),
      ),
    );
  }

  pub fn add_diagnostic_with_hint(
    &mut self,
    span: Span,
    code: impl ToString,
    message: impl ToString,
    hint: impl ToString,
  ) {
    self.add_diagnostic_details(
      Some(self.create_diagnostic_range(span)),
      self.create_diagnostic_details(
        code,
        message,
        Some(hint.to_string()),
        Vec::new(),
      ),
    );
  }

  pub fn add_diagnostic_with_fixes(
    &mut self,
    span: Span,
    code: impl ToString,
    message: impl ToString,
    hint: Option<String>,
    fixes: Vec<LintFix>,
  ) {
    self.add_diagnostic_details(
      Some(self.create_diagnostic_range(span)),
      self.create_diagnostic_details(code, message, hint, fixes),
    );
  }

  pub fn add_diagnostic_details(
    &mut self,
    maybe_range: Option<LintDiagnosticRange>,
    details: LintDiagnosticDetails,
  ) {
    self
      .diagnostics
      .push(self.create_diagnostic(maybe_range, details));
  }

  /// Add fully constructed diagnostics.
  ///
  /// This function can be used by the "external linter" to provide its own
  /// diagnostics.
  pub fn add_external_diagnostics(&mut self, diagnostics: &[LintDiagnostic]) {
    self.diagnostics.extend_from_slice(diagnostics);
  }

  pub(crate) fn create_diagnostic(
    &self,
    maybe_range: Option<LintDiagnosticRange>,
    details: LintDiagnosticDetails,
  ) -> LintDiagnostic {
    LintDiagnostic {
      specifier: self.specifier().clone(),
      range: maybe_range,
      details,
    }
  }

  pub(crate) fn create_diagnostic_details(
    &self,
    code: impl ToString,
    message: impl ToString,
    maybe_hint: Option<String>,
    fixes: Vec<LintFix>,
  ) -> LintDiagnosticDetails {
    LintDiagnosticDetails {
      message: message.to_string(),
      code: code.to_string(),
      hint: maybe_hint,
      fixes,
      custom_docs_url: LintDocsUrl::Default,
      info: vec![],
    }
  }

  pub(crate) fn create_diagnostic_range(
    &self,
    span: Span,
  ) -> LintDiagnosticRange {
    LintDiagnosticRange {
      range: span,
      text_info: self.text_info().clone(),
      description: None,
    }
  }
}

/// Resolve JSX pragma and fragment pragma from source comments, falling back
/// to the provided `LintConfig` defaults.
///
/// Searches for `@jsx` and `@jsxFrag` directives in leading comments, e.g.:
/// ```js
/// /** @jsx h */
/// /** @jsxFrag Fragment */
/// ```
fn resolve_jsx_pragmas(
  parsed_source: &ParsedSource<'_>,
  config: &LintConfig,
) -> (Option<String>, Option<String>) {
  let mut jsx_factory: Option<String> = None;
  let mut jsx_fragment_factory: Option<String> = None;

  let source_text = parsed_source.text();

  for comment in parsed_source.comments().iter() {
    let span = comment.content_span();
    let text = &source_text[span.start as usize..span.end as usize];

    // Look for @jsx pragma
    if jsx_factory.is_none() {
      if let Some(pos) = text.find("@jsx ") {
        let rest = &text[pos + 5..];
        // Take the next non-whitespace token
        let pragma = rest.split_whitespace().next();
        if let Some(pragma) = pragma {
          // Filter out @jsxFrag which also starts with @jsx
          if !text[pos..].starts_with("@jsxFrag") {
            jsx_factory = Some(pragma.to_string());
          }
        }
      }
    }

    // Look for @jsxFrag pragma
    if jsx_fragment_factory.is_none() {
      if let Some(pos) = text.find("@jsxFrag ") {
        let rest = &text[pos + 9..];
        let pragma = rest.split_whitespace().next();
        if let Some(pragma) = pragma {
          jsx_fragment_factory = Some(pragma.to_string());
        }
      }
    }
  }

  // Fall back to config defaults
  if jsx_factory.is_none() {
    jsx_factory = config.default_jsx_factory.clone();
  }
  if jsx_fragment_factory.is_none() {
    jsx_fragment_factory = config.default_jsx_fragment_factory.clone();
  }

  (jsx_factory, jsx_fragment_factory)
}

/// Extract the leftmost identifier from a dotted expression like "React.createElement".
/// Returns "React" for "React.createElement", "h" for "h", etc.
pub(crate) fn leftmost_identifier(expr: &str) -> &str {
  expr.split('.').next().unwrap_or(expr)
}

/// A struct containing a boolean value to control whether a node's children
/// will be traversed or not.
/// If there's no need to further traverse children nodes, you can call
/// `ctx.stop_traverse()` from inside a handler method, which will cancel
/// further traverse.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct TraverseFlow {
  stop_traverse: bool,
}

impl TraverseFlow {
  fn set_stop_traverse(&mut self) {
    self.stop_traverse = true;
  }

  fn reset(&mut self) {
    self.stop_traverse = false;
  }

  fn assert_init(&self) {
    assert!(!self.stop_traverse);
  }

  fn should_stop(&mut self) -> bool {
    let stop = self.stop_traverse;
    self.reset();
    stop
  }
}
