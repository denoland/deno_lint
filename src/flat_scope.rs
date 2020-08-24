use std::collections::HashMap;
use swc_common::DUMMY_SP;
use swc_ecmascript::ast::{Invalid, Module};
use swc_ecmascript::utils::Id;
use swc_ecmascript::visit::Visit;
use swc_ecmascript::visit::VisitWith;

#[derive(Debug)]
pub struct FlatScope {
  vars: HashMap<Id, VarInfo>,
}

#[derive(Debug)]
struct VarInfo {
  path: Vec<BindingKind>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingKind {
  Program,
  Module,
  Function,
  Block,
  Loop,
  Class,
  Switch,
  With,
  Catch,
}

pub fn analyze(module: &Module) -> FlatScope {
  let mut scope = FlatScope {
    vars: Default::default(),
  };

  module.visit_with(
    &Invalid { span: DUMMY_SP },
    &mut Analyzer { scope: &mut scope },
  );

  scope
}

struct Analyzer<'a> {
  scope: &'a mut FlatScope,
}

impl Visit for Analyzer<'_> {}
