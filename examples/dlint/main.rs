// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use anyhow::bail;
use anyhow::Error as AnyError;
use clap::Arg;
use clap::Command;
use core::panic;
use deno_ast::diagnostics::Diagnostic;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::linter::LintConfig;
use deno_lint::linter::LintFileOptions;
use deno_lint::linter::Linter;
use deno_lint::linter::LinterOptions;
use deno_lint::rules::get_all_rules;
use deno_lint::rules::{filtered_rules, recommended_rules};
use log::debug;
use std::borrow::Cow;
use std::collections::BTreeMap;
use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

mod config;
mod diagnostics;
mod grit;
mod rules;

fn create_cli_app<'a>() -> Command<'a> {
  Command::new("dlint")
    .version(clap::crate_version!())
    .subcommand_required(true)
    .subcommand(
      Command::new("rules")
        .arg(
          Arg::new("RULE_NAME")
            .help("Show detailed information about rule. If omitted, show the list of all rules."),
        )
        .arg(Arg::new("json").long("json")),
    )
    .subcommand(
      Command::new("run")
        .arg(
          Arg::new("FILES")
            .help("Set the input file to use")
            .multiple_occurrences(true),
        )
        .arg(
          Arg::new("RULE_CODE")
            .long("rule")
            .help("Run a certain rule")
            .takes_value(true),
        )
        .arg(
          Arg::new("CONFIG")
            .long("config")
            .help("Load config from file")
            .takes_value(true),
        )
        .arg(
          Arg::new("FIX")
            .long("fix")
            .help("Apply configured GritQL rewrites before reporting diagnostics"),
        )
        .arg(
          Arg::new("GRIT_PATTERN")
            .long("grit-pattern")
            .help("Run a GritQL pattern")
            .takes_value(true)
            .multiple_occurrences(true),
        )
        .arg(
          Arg::new("FORMAT")
            .long("format")
            .help("Configure output format")
            .takes_value(true)
            .default_value("pretty")
            .validator(|val: &str| match val {
              "compact" => Ok(()),
              "pretty" => Ok(()),
              _ => Err("Output format must be compact or pretty"),
            }),
        ),
    )
}

fn run_linter(
  paths: Vec<String>,
  filter_rule_name: Option<&str>,
  maybe_config: Option<Arc<config::Config>>,
  format: Option<&str>,
  fix: bool,
  cli_grit_patterns: Vec<String>,
) -> Result<(), AnyError> {
  let cwd = std::env::current_dir()?;
  let mut paths: Vec<PathBuf> =
    paths.into_iter().map(|path| cwd.join(path)).collect();

  if let Some(config) = maybe_config.clone() {
    paths.extend(config.get_files()?);
  }

  paths.sort();
  paths.dedup();

  let all_rules = get_all_rules();
  let all_rule_codes = all_rules
    .iter()
    .map(|rule| rule.code())
    .map(Cow::from)
    .collect::<HashSet<_>>();
  let rules = if let Some(config) = maybe_config.as_ref() {
    config.get_rules()
  } else if let Some(rule_name) = filter_rule_name {
    let include = vec![rule_name.to_string()];
    filtered_rules(get_all_rules(), Some(vec![]), None, Some(include))
  } else {
    recommended_rules(get_all_rules())
  };

  let linter = if rules.is_empty() {
    None
  } else {
    debug!("Configured rules: {}", rules.len());
    Some(Linter::new(LinterOptions {
      rules,
      all_rule_codes,
      custom_ignore_file_directive: None,
      custom_ignore_diagnostic_directive: None,
    }))
  };
  let grit_session =
    resolve_grit_session(maybe_config.as_deref(), cli_grit_patterns)?;

  if linter.is_none() && grit_session.is_none() {
    bail!("No lint rules configured");
  }

  if fix {
    if let Some(grit_session) = grit_session.as_ref() {
      grit_session.apply_fixes(&paths)?;
    }
  }

  let mut error_count = 0usize;
  let mut file_diagnostics = BTreeMap::<String, Vec<LintDiagnostic>>::new();

  for file_path in &paths {
    let source_code = std::fs::read_to_string(file_path)?;

    if let Some(linter) = &linter {
      if should_run_builtin_lint(file_path) {
        let specifier = file_specifier(file_path);
        let (parsed_source, diagnostics) =
          linter.lint_file(LintFileOptions {
            specifier,
            source_code: source_code.clone(),
            media_type: MediaType::from_path(file_path),
            config: LintConfig {
              default_jsx_factory: Some("React.createElement".to_string()),
              default_jsx_fragment_factory: Some("React.Fragment".to_string()),
            },
            external_linter: None,
          })?;

        let mut number_of_errors = diagnostics.len();
        if !parsed_source.diagnostics().is_empty() {
          number_of_errors += parsed_source.diagnostics().len();
          for parsing_diagnostic in parsed_source.diagnostics() {
            eprintln!("{}", parsing_diagnostic.display());
          }
        }
        error_count += number_of_errors;

        for diagnostic in diagnostics {
          file_diagnostics
            .entry(diagnostic.specifier.to_string())
            .or_default()
            .push(diagnostic);
        }
      }
    }

    if let Some(grit_session) = grit_session.as_ref() {
      let diagnostics =
        grit_session.collect_diagnostics(file_path, &source_code)?;
      error_count += diagnostics.len();
      for diagnostic in diagnostics {
        file_diagnostics
          .entry(diagnostic.specifier.to_string())
          .or_default()
          .push(diagnostic);
      }
    }
  }

  for diagnostics in file_diagnostics.values() {
    diagnostics::display_diagnostics(diagnostics, format);
  }

  if error_count > 0 {
    eprintln!(
      "Found {} problem{}",
      error_count,
      if error_count == 1 { "" } else { "s" }
    );
    std::process::exit(1);
  }

  Ok(())
}

fn file_specifier(file_path: &Path) -> ModuleSpecifier {
  ModuleSpecifier::from_file_path(file_path).unwrap_or_else(|_| {
    panic!(
      "Failed to convert path to module specifier: {}",
      file_path.display()
    )
  })
}

fn resolve_grit_session(
  maybe_config: Option<&config::Config>,
  cli_grit_patterns: Vec<String>,
) -> Result<Option<grit::GritSession>, AnyError> {
  let mut patterns = maybe_config
    .map(|config| config.grit.patterns.clone())
    .unwrap_or_default();
  patterns.extend(cli_grit_patterns);
  if patterns.is_empty() {
    return Ok(None);
  }

  Ok(Some(grit::GritSession::new(grit::GritOptions {
    patterns,
  })?))
}

fn should_run_builtin_lint(file_path: &Path) -> bool {
  matches!(
    file_path.extension().and_then(|ext| ext.to_str()),
    Some("js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts")
  )
}

fn main() -> Result<(), AnyError> {
  env_logger::init();

  let cli_app = create_cli_app();
  let matches = cli_app.get_matches();

  match matches.subcommand() {
    Some(("run", run_matches)) => {
      let maybe_config = if let Some(p) = run_matches.value_of("CONFIG") {
        let path = PathBuf::from(p);

        let c = match path.extension().and_then(|s| s.to_str()) {
          Some("json") => config::load_from_json(&path)?,
          ext => bail!("Unknown extension: \"{:#?}\". Use .json instead.", ext),
        };
        Some(Arc::new(c))
      } else {
        None
      };

      debug!("Config: {:#?}", maybe_config);

      let paths: Vec<String> = run_matches
        .values_of("FILES")
        .unwrap_or_default()
        .map(|p| p.to_string())
        .collect();
      run_linter(
        paths,
        run_matches.value_of("RULE_CODE"),
        maybe_config,
        run_matches.value_of("FORMAT"),
        run_matches.is_present("FIX"),
        run_matches
          .values_of("GRIT_PATTERN")
          .map(|values| values.map(|value| value.to_string()).collect())
          .unwrap_or_default(),
      )?;
    }
    Some(("rules", rules_matches)) => {
      let rules = if let Some(rule_name) = rules_matches.value_of("RULE_NAME") {
        rules::get_specific_rule_metadata(rule_name)
      } else {
        rules::get_all_rules_metadata()
      };
      if rules_matches.is_present("json") {
        rules::print_rules::<rules::JsonFormatter>(rules);
      } else {
        rules::print_rules::<rules::PrettyFormatter>(rules);
      }
    }
    _ => unreachable!(),
  };

  Ok(())
}

#[cfg(test)]
mod tests {
  use os_pipe::pipe;
  use std::fs;
  use std::io::Read;
  use std::io::Write;
  use std::path::PathBuf;
  use std::process::Command;
  use std::process::Stdio;
  use std::time::{SystemTime, UNIX_EPOCH};

  // TODO(bartlomieju): this code is copy-pasted from `deno/test_util/src/lib.rs`

  pub fn strip_ansi_codes(s: &str) -> std::borrow::Cow<'_, str> {
    console_static_text::ansi::strip_ansi_codes(s)
  }

  fn target_dir() -> PathBuf {
    let current_exe = std::env::current_exe().unwrap();
    let target_dir = current_exe.parent().unwrap().parent().unwrap();
    target_dir.into()
  }

  fn dlint_exe_path() -> PathBuf {
    // Something like /Users/src/deno_lint/target/debug/examples/dlint
    let mut p = target_dir().join("examples").join("dlint");
    if cfg!(windows) {
      p.set_extension("exe");
    }
    p
  }

  fn root_path() -> PathBuf {
    PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR")))
  }

  fn testdata_path() -> PathBuf {
    root_path().join("examples").join("dlint").join("testdata")
  }

  fn dlint_cmd() -> Command {
    let exe_path = dlint_exe_path();
    assert!(exe_path.exists());
    Command::new(exe_path)
  }

  #[derive(Debug, Default)]
  struct CheckOutputIntegrationTest {
    pub args: &'static str,
    pub args_vec: Vec<&'static str>,
    pub output: &'static str,
    pub input: Option<&'static str>,
    pub output_str: Option<&'static str>,
    pub exit_code: i32,
    pub envs: Vec<(String, String)>,
  }

  impl CheckOutputIntegrationTest {
    pub fn run(&self) {
      let args = if self.args_vec.is_empty() {
        std::borrow::Cow::Owned(
          self.args.split_whitespace().collect::<Vec<_>>(),
        )
      } else {
        assert!(
          self.args.is_empty(),
          "Do not provide args when providing args_vec."
        );
        std::borrow::Cow::Borrowed(&self.args_vec)
      };
      let dlint_exe = dlint_exe_path();
      println!("dlint_exe path {}", dlint_exe.display());

      let (mut reader, writer) = pipe().unwrap();
      let testdata_dir = testdata_path();
      let mut command = dlint_cmd();
      println!("dlint_exe args {}", self.args);
      println!("dlint_exe testdata path {:?}", &testdata_dir);
      command.args(args.iter());
      command.envs(self.envs.clone());
      command.current_dir(&testdata_dir);
      command.stdin(Stdio::piped());
      let writer_clone = writer.try_clone().unwrap();
      command.stderr(writer_clone);
      command.stdout(writer);

      let mut process = command.spawn().expect("failed to execute process");

      if let Some(input) = self.input {
        let mut p_stdin = process.stdin.take().unwrap();
        write!(p_stdin, "{}", input).unwrap();
      }

      drop(command);

      let mut actual = String::new();
      reader.read_to_string(&mut actual).unwrap();

      let status = process.wait().expect("failed to finish process");

      if let Some(exit_code) = status.code() {
        if self.exit_code != exit_code {
          println!("OUTPUT\n{}\nOUTPUT", actual);
          panic!(
            "bad exit code, expected: {:?}, actual: {:?}",
            self.exit_code, exit_code
          );
        }
      } else {
        #[cfg(unix)]
        {
          use std::os::unix::process::ExitStatusExt;
          let signal = status.signal().unwrap();
          println!("OUTPUT\n{}\nOUTPUT", actual);
          panic!(
            "process terminated by signal, expected exit code: {:?}, actual signal: {:?}",
            self.exit_code, signal
          );
        }
        #[cfg(not(unix))]
        {
          println!("OUTPUT\n{}\nOUTPUT", actual);
          panic!("process terminated without status code on non unix platform, expected exit code: {:?}", self.exit_code);
        }
      }

      actual = strip_ansi_codes(&actual).to_string();

      let expected = if let Some(s) = self.output_str {
        s.to_owned()
      } else {
        let output_path = testdata_dir.join(self.output);
        println!("output path {}", output_path.display());
        std::fs::read_to_string(output_path).expect("cannot read output")
      };

      if !expected.contains("[WILDCARD]") {
        assert_eq!(actual, expected)
      } else if !wildcard_match(&expected, &actual) {
        println!("OUTPUT\n{}\nOUTPUT", actual);
        println!("EXPECTED\n{}\nEXPECTED", expected);
        panic!("pattern match failed");
      }
    }
  }

  fn wildcard_match(pattern: &str, s: &str) -> bool {
    pattern_match(pattern, s, "[WILDCARD]")
  }

  fn pattern_match(pattern: &str, s: &str, wildcard: &str) -> bool {
    let mut s = s.replace("\r\n", "\n");
    let pattern = pattern.replace("\r\n", "\n");

    if pattern == wildcard {
      return true;
    }

    let parts = pattern.split(wildcard).collect::<Vec<&str>>();
    if parts.len() == 1 {
      return pattern == s;
    }

    if !s.starts_with(parts[0]) {
      return false;
    }

    if pattern.lines().next() == Some(wildcard) {
      s.insert(0, '\n');
    }

    let mut t = s.split_at(parts[0].len());

    for (i, part) in parts.iter().enumerate() {
      if i == 0 {
        continue;
      }
      dbg!(part, i);
      if i == parts.len() - 1 && (part.is_empty() || *part == "\n") {
        dbg!("exit 1 true", i);
        return true;
      }
      if let Some(found) = t.1.find(*part) {
        dbg!("found ", found);
        t = t.1.split_at(found + part.len());
      } else {
        dbg!("exit false ", i);
        return false;
      }
    }

    dbg!("end ", t.1.len());
    t.1.is_empty()
  }

  #[macro_export]
  macro_rules! itest(
  ($name:ident {$( $key:ident: $value:expr,)*})  => {
    #[test]
    fn $name() {
      (CheckOutputIntegrationTest {
        $(
          $key: $value,
        )*
        .. Default::default()
      }).run()
    }
  }
  );

  itest!(simple_test {
    args: "run simple.ts",
    output: "simple.out",
    exit_code: 1,
  });

  itest!(issue1145_no_trailing_newline {
    args: "run issue1145_no_trailing_newline.ts",
    output: "issue1145_no_trailing_newline.out",
    exit_code: 1,
  });

  itest!(grit_pattern_cli {
    args_vec: vec![
      "run",
      "--format",
      "compact",
      "--grit-pattern",
      "language js\n`console.log($value)`",
      "grit_match.ts",
    ],
    output: "grit_match.out",
    exit_code: 1,
  });

  itest!(grit_pattern_config {
    args: "run --format compact --config grit_config.json grit_match.ts",
    output: "grit_match.out",
    exit_code: 1,
  });

  itest!(grit_json_pattern_cli {
    args_vec: vec![
      "run",
      "--format",
      "compact",
      "--grit-pattern",
      "language json\n`{\"foo\": $value}`",
      "grit_match.json",
    ],
    output: "grit_match_json.out",
    exit_code: 1,
  });

  itest!(grit_jsonc_pattern_cli {
    args_vec: vec![
      "run",
      "--format",
      "compact",
      "--grit-pattern",
      "language json\n`{\"foo\": $value}`",
      "grit_match.jsonc",
    ],
    output: "grit_match_jsonc.out",
    exit_code: 1,
  });

  fn run_grit_fix_test(
    file_name: &str,
    initial_text: &str,
    pattern: &str,
    expected_text: &str,
  ) {
    let temp_dir = std::env::temp_dir().join(format!(
      "deno_lint_grit_fix_{}_{}",
      std::process::id(),
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos()
    ));
    fs::create_dir_all(&temp_dir).unwrap();

    let file_path = temp_dir.join(file_name);
    fs::write(&file_path, initial_text).unwrap();

    let output = dlint_cmd()
      .current_dir(&temp_dir)
      .args(["run", "--fix", "--grit-pattern", pattern, file_name])
      .output()
      .unwrap();

    if !output.status.success() {
      panic!(
        "dlint failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
      );
    }

    assert_eq!(fs::read_to_string(&file_path).unwrap(), expected_text);

    fs::remove_dir_all(temp_dir).unwrap();
  }

  #[test]
  fn grit_fix_rewrites_js_file() {
    run_grit_fix_test(
      "grit_fix.ts",
      "hello();\n",
      "language js\n`hello()` => `greet()`",
      "greet();\n",
    );
  }

  #[test]
  fn grit_fix_rewrites_json_file() {
    run_grit_fix_test(
      "grit_fix.json",
      "{\"foo\": 1}\n",
      "language json\n`{\"foo\": $value}` => `{\"bar\": $value}`",
      "{\"bar\": 1}\n",
    );
  }

  #[test]
  fn grit_fix_rewrites_jsonc_file() {
    run_grit_fix_test(
      "grit_fix.jsonc",
      "{\n  // comment\n  \"foo\": 1\n}\n",
      "language json\n`{\"foo\": $value}` => `{\"bar\": $value}`",
      "{\"bar\": 1}\n",
    );
  }
}
