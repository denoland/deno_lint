// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use clap::App;
use clap::Arg;

fn create_cli_app<'a, 'b>(rule_list: &'b str) -> App<'a, 'b> {
  App::new("dlint").after_help(rule_list).arg(
    Arg::with_name("FILES")
      .help("Sets the input file to use")
      .required(true)
      .multiple(true),
  )
}

fn main() {
  use deno_lint::linter::Linter;
  use deno_lint::rules::get_all_rules;

  #[cfg(windows)]
  colors::enable_ansi();

  let rules = get_all_rules();

  let mut rule_names = rules
    .iter()
    .map(|r| r.code())
    .map(|name| format!(" - {}", name))
    .collect::<Vec<String>>();

  rule_names.sort();
  rule_names.insert(0, "Available rules:".to_string());

  let rule_list = rule_names.join("\n");
  let cli_app = create_cli_app(&rule_list);
  let matches = cli_app.get_matches();
  let file_names = matches.values_of("FILES").unwrap();

  let mut error_counts = 0;

  for file_name in file_names {
    let source_code =
      std::fs::read_to_string(&file_name).expect("Failed to read file");

    let mut linter = Linter::default();

    let rules = get_all_rules();

    let file_diagnostics = linter
      .lint(file_name.to_string(), source_code, rules)
      .expect("Failed to lint");

    error_counts += file_diagnostics.len();
    for d in file_diagnostics.iter() {
      eprintln!("{}", d.to_pretty_string());
    }
  }

  if error_counts > 0 {
    eprintln!("Found {} problems", error_counts);
  }
}
