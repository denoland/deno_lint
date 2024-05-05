// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::{Handler, Traverse};

use deno_ast::view::{
  Expr, JSXAttrName, JSXAttrValue, JSXElementName, JSXExpr, Program,
};
use deno_ast::SourceRanged;

#[derive(Debug)]
pub struct FreshServerEventHandlers;

const CODE: &str = "fresh-server-event-handlers";
const MESSAGE: &str =
  "Server components cannot install client side event handlers.";
const HINT: &str =
  "Remove this property or turn the enclosing component into an island";

impl LintRule for FreshServerEventHandlers {
  fn tags(&self) -> &'static [&'static str] {
    &["fresh"]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view(
    &self,
    context: &mut Context,
    program: Program,
  ) {
    Visitor.traverse(program, context);
  }

  #[cfg(feature = "docs")]
  fn docs(&self) -> &'static str {
    include_str!("../../docs/rules/fresh_server_event_handlers.md")
  }
}

struct Visitor;

impl Handler for Visitor {
  fn jsx_attr(
    &mut self,
    jsx_attr: &deno_ast::view::JSXAttr,
    ctx: &mut Context,
  ) {
    // Fresh only considers components in the routes/ folder to be
    // server components. Files inside an `(_islands)` folder are considered
    // islands though, even if they are inside the `routes` folder.
    let Some(path_segments) = ctx.specifier().path_segments() else {
      return;
    };
    let segments = path_segments.collect::<Vec<_>>();
    if !segments.iter().any(|comp| *comp == "routes")
      || segments.iter().any(|comp| *comp == "(_islands)")
    {
      return;
    }

    // We only care about native HTML elements not namespaced XML
    // like `<foo:bar />` or member expressions like `<foo.bar />`
    let parent_name = match jsx_attr.parent().name {
      JSXElementName::Ident(name) => name.sym(),
      _ => return,
    };

    // Preact doesn't support namespaced attributes like `on:click`
    // so far.
    let attr_name = match jsx_attr.name {
      JSXAttrName::Ident(ident) => ident,
      JSXAttrName::JSXNamespacedName(_) => return,
    };

    // Check that we're dealing with a native HTML element and not a
    // component. A component starts with an upper case letter, like
    // <Foo />
    if *parent_name != parent_name.to_lowercase() {
      return;
    }

    // Check for custom elements where we cannot make assumptions about
    // event listeners being the only attributes to receive a function.
    // They must have a `-` in the name per spec like `<x-foo />`. See:
    // https://html.spec.whatwg.org/multipage/custom-elements.html#prod-potentialcustomelementname
    if parent_name.contains('-') {
      let expr = match jsx_attr.value {
        Some(JSXAttrValue::JSXExprContainer(expr)) => expr,
        _ => return,
      };

      let JSXExpr::Expr(expr_value) = expr.expr else {
        return;
      };

      // If we pass a function expression or an arrow function expression
      // then we know for sure that we can't render that.
      match expr_value {
        Expr::Arrow(_) => {
          ctx.add_diagnostic_with_hint(jsx_attr.range(), CODE, MESSAGE, HINT);
        }
        Expr::Fn(_) => {
          ctx.add_diagnostic_with_hint(jsx_attr.range(), CODE, MESSAGE, HINT);
        }
        _ => return,
      }

      return;
    }

    // Check that the JSX attribute name represents an event handler.
    // All event handlers start with the two letters "on". Preact
    // lowercases the name internally.
    if !attr_name.sym().to_lowercase().starts_with("on") {
      return;
    }

    // Check that the attribute is an expression. Passsing a literal
    // like `<button onClick="console.log()">` may be something we're
    // going to allow in the future.
    if let Some(JSXAttrValue::JSXExprContainer(_)) = jsx_attr.value {
      ctx.add_diagnostic_with_hint(jsx_attr.range(), CODE, MESSAGE, HINT);
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::test_util::assert_lint_ok;

  #[test]
  fn no_server_event_handler() {
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<Foo onClick={() => {}} />",
      "file:///foo.jsx",
    );
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<button onClick={() => {}} />",
      "file:///foo.jsx",
    );
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<button onClick={function () {}} />",
      "file:///foo.jsx",
    );
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<button onclick={function () {}} />",
      "file:///foo.jsx",
    );
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<button onClick=\"console.log('hey')\" />",
      "file:///foo.jsx",
    );
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<button online=\"foo\" />",
      "file:///foo.jsx",
    );
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<x-foo onClick=\"console.log('hey')\" />",
      "file:///foo.jsx",
    );
    assert_lint_ok(
      &FreshServerEventHandlers,
      "<button onClick={function () {}} />",
      "file:///routes/foo/(_islands)/foo.jsx",
    );

    assert_lint_err!(FreshServerEventHandlers, filename: "file:///routes/index.tsx",  r#"<button onClick={() => {}} />"#: [
    {
      col: 8,
      message: MESSAGE,
      hint: HINT,
    }]);
    assert_lint_err!(FreshServerEventHandlers, filename: "file:///routes/index.tsx",  r#"<button onTouchMove={() => {}} />"#: [
    {
      col: 8,
      message: MESSAGE,
      hint: HINT,
    }]);
    assert_lint_err!(FreshServerEventHandlers, filename: "file:///routes/index.tsx",  r#"<button onTouchMove={"console.log('hey')"} />"#: [
    {
      col: 8,
      message: MESSAGE,
      hint: HINT,
    }]);

    assert_lint_err!(FreshServerEventHandlers, filename: "file:///routes/index.tsx",  r#"<foo-button foo={() => {}} />"#: [
    {
      col: 12,
      message: MESSAGE,
      hint: HINT,
    }]);
    assert_lint_err!(FreshServerEventHandlers, filename: "file:///routes/index.tsx",  r#"<foo-button foo={function () {}} />"#: [
    {
      col: 12,
      message: MESSAGE,
      hint: HINT,
    }]);
  }
}
