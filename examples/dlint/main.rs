// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use anyhow::bail;
use anyhow::Error as AnyError;
use clap::Arg;
use clap::Command;
use deno_ast::MediaType;
use deno_ast::SourceTextInfo;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::linter::LinterBuilder;
use deno_lint::rules::{get_filtered_rules, get_recommended_rules};
use log::debug;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

mod color;
mod config;
mod diagnostics;
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
        ).arg(
          Arg::new("FORMAT")
            .long("format")
            .help("Configure output format")
            .takes_value(true)
            .default_value("pretty")
            .validator(|val: &str| match val {
              "compact" => Ok(()),
              "pretty" => Ok(()),
              _ => Err("Output format must be compact or pretty")
            }),
        )
    )
}

fn run_linter(
  paths: Vec<String>,
  filter_rule_name: Option<&str>,
  maybe_config: Option<Arc<config::Config>>,
  format: Option<&str>,
) -> Result<(), AnyError> {
  let mut paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

  if let Some(config) = maybe_config.clone() {
    paths.extend(config.get_files()?);
  }

  let error_counts = Arc::new(AtomicUsize::new(0));

  struct FileDiagnostics {
    filename: String,
    text_info: SourceTextInfo,
    diagnostics: Vec<LintDiagnostic>,
  }

  let rules = if let Some(config) = maybe_config {
    config.get_rules()
  } else if let Some(rule_name) = filter_rule_name {
    let include = vec![rule_name.to_string()];
    get_filtered_rules(Some(vec![]), None, Some(include))
  } else {
    get_recommended_rules()
  };
  let file_diagnostics = Arc::new(Mutex::new(BTreeMap::new()));
  paths
    .par_iter()
    .try_for_each(|file_path| -> Result<(), AnyError> {
      let source_code = std::fs::read_to_string(file_path)?;

      debug!("Configured rules: {}", rules.len());

      if rules.is_empty() {
        bail!("There's no rule to be run!");
      }

      let linter_builder = LinterBuilder::default()
        .rules(rules.clone())
        .media_type(MediaType::from_path(file_path));

      let linter = linter_builder.build();

      let (parsed_source, diagnostics) =
        linter.lint(file_path.to_string_lossy().to_string(), source_code)?;

      error_counts.fetch_add(diagnostics.len(), Ordering::Relaxed);

      let mut lock = file_diagnostics.lock().unwrap();

      lock.insert(
        file_path,
        FileDiagnostics {
          filename: file_path.to_string_lossy().to_string(),
          diagnostics,
          text_info: parsed_source.text_info().clone(),
        },
      );

      Ok(())
    })?;

  for d in file_diagnostics.lock().unwrap().values() {
    diagnostics::display_diagnostics(
      &d.diagnostics,
      &d.text_info,
      &d.filename,
      format,
    );
  }

  let err_count = error_counts.load(Ordering::Relaxed);
  if err_count > 0 {
    eprintln!(
      "Found {} problem{}",
      err_count,
      if err_count == 1 { "" } else { "s" }
    );
    std::process::exit(1);
  }

  Ok(())
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
  use std::io::Read;
  use std::io::Write;
  use std::path::PathBuf;
  use std::process::Command;
  use std::process::Stdio;

  // TODO(bartlomieju): this code is copy-pasted from `deno/test_util/src/lib.rs`

  pub fn strip_ansi_codes(s: &str) -> std::borrow::Cow<str> {
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

      // Very important when using pipes: This parent process is still
      // holding its copies of the write ends, and we have to close them
      // before we read, otherwise the read end will never report EOF. The
      // Command object owns the writers now, and dropping it closes them.
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

  // TODO(bartlomieju): update to the current version in `deno` repo
  fn pattern_match(pattern: &str, s: &str, wildcard: &str) -> bool {
    // Normalize line endings
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

    // If the first line of the pattern is just a wildcard the newline character
    // needs to be pre-pended so it can safely match anything or nothing and
    // continue matching.
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
}
