// Copyright 2020 the Deno authors. All rights reserved. MIT license.
mod linter;
mod rules;

use linter::Linter;
use rules::LintRule;

#[cfg(test)]
mod test_util;

fn main() {
  let args: Vec<String> = std::env::args().collect();

  if args.len() < 2 {
    eprintln!("Missing file name");
    std::process::exit(1);
  }

  let file_names: Vec<String> = args[1..].to_vec();

  let mut diagnostics = vec![];

  for file_name in file_names {
    let source_code =
      std::fs::read_to_string(&file_name).expect("Failed to read file");

    let mut linter = Linter::default();

    let rules: Vec<Box<dyn LintRule>> = vec![
      rules::NoExplicitAny::new(),
      rules::NoDebugger::new(),
      rules::NoVar::new(),
      rules::SingleVarDeclarator::new(),
      rules::ExplicitFunctionReturnType::new(),
      rules::NoEval::new(),
      rules::NoEmptyInterface::new(),
      rules::NoDeleteVar::new(),
      rules::UseIsNaN::new(),
      rules::NoEmptyFunction::new(),
      rules::NoAsyncPromiseExecutor::new(),
      rules::NoSparseArray::new(),
      rules::NoDuplicateCase::new(),
      rules::NoDupeArgs::new(),
      rules::BanTsIgnore::new(),
      rules::BanUntaggedTodo::new(),
      rules::GetterReturn::new(),
      rules::NoSetterReturn::new(),
      rules::Eqeqeq::new(),
      rules::NoDupeKeys::new(),
      rules::NoCompareNegZero::new(),
      rules::NoUnsafeFinally::new(),
      rules::NoThrowLiteral::new(),
    ];

    let file_diagnostics = linter
      .lint(file_name, source_code, rules)
      .expect("Failed to lint");

    diagnostics.extend(file_diagnostics)
  }

  if !diagnostics.is_empty() {
    for d in diagnostics.iter() {
      eprintln!(
        "error: {} ({}) at {}:{}:{}",
        d.message, d.code, d.location.filename, d.location.line, d.location.col
      );
    }
    eprintln!("Found {} problems", diagnostics.len());
  }
}
