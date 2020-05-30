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

  let mut error_counts = 0;
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
      rules::ValidTypeof::new(),
      rules::NoThrowLiteral::new(),
      rules::NoNewSymbol::new(),
      rules::DefaultParamLast::new(),
      rules::NoEmpty::new(),
      rules::NoCondAssign::new(),
    ];

    let file_diagnostics = linter
      .lint(file_name, source_code, rules)
      .expect("Failed to lint");

    error_counts += file_diagnostics.len();
    if !file_diagnostics.is_empty() {
      eprintln!("{} =>", file_diagnostics[0].location.filename);
      for d in file_diagnostics.iter() {
        eprintln!(
          "  {}| {}\n  {}^ \n  ({}) {}\n",
          d.location.line,
          d.line_src,
          " ".repeat(d.location.col + 3),
          d.code,
          d.message
        );
      }
    }
  }
  eprintln!("Found {} problems", error_counts);
}
