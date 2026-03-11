// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use super::{Context, LintRule};
use crate::handler::Handler;
use crate::tags::{self, Tags};
use deno_ast::oxc::ast::ast::{
  JSXAttributeItem, JSXAttributeName, JSXAttributeValue, JSXElementName,
  JSXExpression, JSXOpeningElement, Program,
};


#[derive(Debug)]
pub struct FreshServerEventHandlers;

const CODE: &str = "fresh-server-event-handlers";
const MESSAGE: &str =
  "Server components cannot install client side event handlers.";
const HINT: &str =
  "Remove this property or turn the enclosing component into an island";

impl LintRule for FreshServerEventHandlers {
  fn tags(&self) -> Tags {
    &[tags::FRESH]
  }

  fn code(&self) -> &'static str {
    CODE
  }

  fn lint_program_with_ast_view<'a>(
    &self,
    context: &mut Context<'a>,
    program: &Program<'a>,
  ) {
    let mut handler = FreshServerEventHandlersHandler;
    crate::handler::traverse_program(&mut handler, program, context);
  }
}

struct FreshServerEventHandlersHandler;

impl Handler<'_> for FreshServerEventHandlersHandler {
  fn jsx_opening_element(
    &mut self,
    node: &JSXOpeningElement,
    ctx: &mut Context,
  ) {
    // Fresh only considers components in the routes/ folder to be
    // server components. Files inside an `(_islands)` folder are considered
    // islands though, even if they are inside the `routes` folder.
    let Some(path_segments) = ctx.specifier().path_segments() else {
      return;
    };
    let segments = path_segments.collect::<Vec<_>>();
    if !segments.contains(&"routes") || segments.contains(&"(_islands)") {
      return;
    }

    // We only care about native HTML elements not namespaced XML
    // like `<foo:bar />` or member expressions like `<foo.bar />`
    let parent_name = match &node.name {
      JSXElementName::Identifier(id) => id.name.as_str(),
      _ => return,
    };

    // Check that we're dealing with a native HTML element and not a
    // component. A component starts with an upper case letter, like
    // <Foo />
    if parent_name != parent_name.to_lowercase() {
      return;
    }

    let is_custom_element = parent_name.contains('-');

    for attr in &node.attributes {
      let JSXAttributeItem::Attribute(attr) = attr else {
        continue;
      };

      let JSXAttributeName::Identifier(attr_name) = &attr.name else {
        // Preact doesn't support namespaced attributes like `on:click`
        continue;
      };

      if is_custom_element {
        // Check for custom elements where we cannot make assumptions about
        // event listeners being the only attributes to receive a function.
        // They must have a `-` in the name per spec like `<x-foo />`.
        let Some(JSXAttributeValue::ExpressionContainer(expr)) =
          &attr.value
        else {
          continue;
        };

        match &expr.expression {
          JSXExpression::ArrowFunctionExpression(_)
          | JSXExpression::FunctionExpression(_) => {
            ctx.add_diagnostic_with_hint(
              attr.span, CODE, MESSAGE, HINT,
            );
          }
          _ => {}
        }
        continue;
      }

      // Check that the JSX attribute name represents an event handler.
      // All event handlers start with the two letters "on". Preact
      // lowercases the name internally.
      if !attr_name.name.as_str().to_lowercase().starts_with("on") {
        continue;
      }

      // Check that the attribute is an expression. Passing a literal
      // like `<button onClick="console.log()">` may be something we're
      // going to allow in the future.
      if let Some(JSXAttributeValue::ExpressionContainer(_)) = &attr.value {
        ctx.add_diagnostic_with_hint(attr.span, CODE, MESSAGE, HINT);
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn no_server_event_handler() {
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///foo.jsx",
      "<Foo onClick={() => {}} />",
    );
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///foo.jsx",
      "<button onClick={() => {}} />",
    );
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///foo.jsx",
      "<button onClick={function () {}} />",
    );
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///foo.jsx",
      "<button onclick={function () {}} />",
    );
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///foo.jsx",
      "<button onClick=\"console.log('hey')\" />",
    );
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///foo.jsx",
      "<button online=\"foo\" />",
    );
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///foo.jsx",
      "<x-foo onClick=\"console.log('hey')\" />",
    );
    assert_lint_ok!(
      FreshServerEventHandlers,
      filename: "file:///routes/foo/(_islands)/foo.jsx",
      "<button onClick={function () {}} />",
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
