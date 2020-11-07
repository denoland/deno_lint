use std::marker::PhantomData;

use crate::linter::Context;
use crate::rules::LintRule;
use swc_atoms::js_word;
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::AssignExpr;
use swc_ecmascript::ast::CallExpr;
use swc_ecmascript::ast::ClassProp;
use swc_ecmascript::ast::Expr;
use swc_ecmascript::ast::ExprOrSuper;
use swc_ecmascript::ast::Ident;
use swc_ecmascript::ast::Invalid;
use swc_ecmascript::ast::MemberExpr;
use swc_ecmascript::ast::Pat;
use swc_ecmascript::ast::Program;
use swc_ecmascript::ast::Prop;
use swc_ecmascript::ast::UnaryExpr;
use swc_ecmascript::ast::UnaryOp;
use swc_ecmascript::ast::VarDecl;
use swc_ecmascript::ast::VarDeclKind;
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
      rule: S::new(),
      context,
      pat_var_decl_kind: None,
    };

    program.visit_with(&Invalid { span: DUMMY_SP }, &mut visitor);
  }

  fn code(&self) -> &'static str {
    S::code()
  }

  fn tags(&self) -> &'static [&'static str] {
    S::tags()
  }

  fn docs(&self) -> &'static str {
    S::docs()
  }
}

pub trait ScopeRule {
  fn new() -> Self;

  fn ignore_typeof() -> bool {
    false
  }

  fn code() -> &'static str;
  fn tags() -> &'static [&'static str] {
    &[]
  }
  fn docs() -> &'static str {
    ""
  }

  fn assign(&mut self, context: &mut Context, i: &Ident);

  fn check_usage(&mut self, context: &mut Context, i: &Ident);
}

struct ScopedRuleVisitor<'a, S>
where
  S: ScopeRule,
{
  rule: S,
  context: &'a mut Context,
  /// [Some] while folding patterns of variables,
  ///  and [None] while folding patterns of an assigment expressions.
  pat_var_decl_kind: Option<VarDeclKind>,
}

impl<S> Visit for ScopedRuleVisitor<'_, S>
where
  S: ScopeRule,
{
  fn visit_unary_expr(&mut self, expr: &UnaryExpr, _: &dyn Node) {
    if S::ignore_typeof() && expr.op == UnaryOp::TypeOf {
      return;
    }

    expr.visit_children_with(self)
  }

  fn visit_class_prop(&mut self, p: &ClassProp, _: &dyn Node) {
    p.value.visit_with(p, self)
  }

  /// This is required as scoped rules deal with identifiers.
  fn visit_member_expr(&mut self, expr: &MemberExpr, _: &dyn Node) {
    expr.obj.visit_with(expr, self);

    if expr.computed {
      expr.prop.visit_with(expr, self);
    }
  }

  /// This method is required because shorthand properties are usage of an identifier.
  fn visit_prop(&mut self, p: &Prop, _: &dyn Node) {
    p.visit_children_with(self);

    if let Prop::Shorthand(i) = &p {
      self.rule.check_usage(self.context, i);
    }
  }

  fn visit_expr(&mut self, expr: &Expr, _: &dyn Node) {
    expr.visit_children_with(self);

    match expr {
      Expr::Ident(i) => {
        self.rule.check_usage(self.context, i);
      }
      _ => {}
    }
  }

  fn visit_pat(&mut self, pat: &Pat, _: &dyn Node) {
    pat.visit_children_with(self);

    if let Pat::Ident(i) = pat {
      if let None = self.pat_var_decl_kind {
        self.rule.assign(self.context, i);
      }
    }
  }

  fn visit_assign_expr(&mut self, expr: &AssignExpr, _: &dyn Node) {
    let old_pat_kind = self.pat_var_decl_kind;
    self.pat_var_decl_kind = None;

    expr.visit_children_with(self);

    self.pat_var_decl_kind = old_pat_kind;
  }

  fn visit_var_decl(&mut self, var: &VarDecl, _: &dyn Node) {
    let old_pat_kind = self.pat_var_decl_kind;
    self.pat_var_decl_kind = Some(var.kind);

    var.visit_children_with(self);

    self.pat_var_decl_kind = old_pat_kind;
  }

  fn visit_call_expr(&mut self, expr: &CallExpr, _: &dyn Node) {
    if let ExprOrSuper::Expr(callee) = &expr.callee {
      if let Expr::Ident(i) = &**callee {
        if i.sym == js_word!("import") {
          return;
        }
      }
    }

    expr.visit_children_with(self);
  }
}
