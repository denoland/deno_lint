use std::marker::PhantomData;

use crate::linter::Context;
use crate::rules::LintRule;
use crate::scopes::BindingKind;
use swc_common::Span;
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::Invalid;
use swc_ecmascript::ast::Program;
use swc_ecmascript::utils::Id;
use swc_ecmascript::visit::Node;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

pub struct ScopedRule<S>
where
  S: ScopeRule,
{
  _marker: PhantomData<S>,
}

impl<S> LintRule for ScopedRule<S>
where
  S: ScopeRule,
{
  fn new() -> Box<Self>
  where
    Self: Sized,
  {
    Box::new(ScopedRule {
      _marker: PhantomData,
    })
  }

  fn lint_program(&self, context: &mut Context, program: &Program) {
    let mut visitor = ScopedRuleVisitor {
      context,
      rule: S::new(),
    };

    program.visit_with(&Invalid { span: DUMMY_SP }, &mut visitor);
  }

  fn code(&self) -> &'static str {
    S::new().code()
  }

  fn tags(&self) -> &'static [&'static str] {
    S::new().tags()
  }

  fn docs(&self) -> &'static str {
    S::new().docs()
  }
}

pub trait ScopeRule {
  fn new() -> Self;

  fn code(&self) -> &'static str;
  fn tags(&self) -> &'static [&'static str] {
    &[]
  }
  fn docs(&self) -> &'static str {
    ""
  }

  fn declare(&mut self, id: Id, kind: BindingKind);

  fn assign(&mut self, id: Id);

  fn lint_usage(&mut self, context: &mut Context, span: Span, id: Id);
}

struct ScopedRuleVisitor<'a, S>
where
  S: ScopeRule,
{
  rule: S,
  context: &'a mut Context,
}

impl<S> Visit for ScopedRuleVisitor<'_, S>
where
  S: ScopeRule,
{
  fn visit_expr(&mut self, expr: &Expr, _: &dyn Node) {}
}
