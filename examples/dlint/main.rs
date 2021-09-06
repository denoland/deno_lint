// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
use anyhow::bail;
use anyhow::Error as AnyError;
use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use deno_ast::swc::parser::{EsConfig, Syntax, TsConfig};
use deno_ast::SourceTextInfo;
use deno_lint::ast_parser::{get_default_es_config, get_default_ts_config};
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::linter::{LinterBuilder, Plugin};
use deno_lint::rules::{get_filtered_rules, get_recommended_rules};
use log::debug;
use rayon::prelude::*;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

mod color;
mod config;
mod diagnostics;
mod js;
mod lexer;
mod rules;

fn create_cli_app<'a, 'b>() -> App<'a, 'b> {
  App::new("dlint")
    .version(clap::crate_version!())
    .setting(AppSettings::SubcommandRequiredElseHelp)
    .subcommand(
      SubCommand::with_name("rules")
        .arg(
          Arg::with_name("RULE_NAME")
            .help("Show detailed information about rule. If omitted, show the list of all rules."),
        )
        .arg(Arg::with_name("json").long("json")),
    )
    .subcommand(
      SubCommand::with_name("run")
        .arg(
          Arg::with_name("FILES")
            .help("Set the input file to use")
            .multiple(true),
        )
        .arg(
          Arg::with_name("RULE_CODE")
            .long("rule")
            .help("Run a certain rule")
            .takes_value(true),
        )
        .arg(
          Arg::with_name("CONFIG")
            .long("config")
            .help("Load config from file")
            .takes_value(true),
        )
        .arg(
          Arg::with_name("PLUGIN")
            .long("plugin")
            .help("Specify plugin paths")
            .multiple(true)
            .takes_value(true),
        ),
    )
}

fn run_linter(
  paths: Vec<String>,
  filter_rule_name: Option<&str>,
  maybe_config: Option<Arc<config::Config>>,
  plugin_paths: Vec<&str>,
) -> Result<(), AnyError> {
  let mut paths: Vec<PathBuf> = paths.iter().map(PathBuf::from).collect();

  if let Some(config) = maybe_config.clone() {
    paths.extend(config.get_files()?);
  }

  let error_counts = Arc::new(AtomicUsize::new(0));

  struct FileDiagnostics {
    text_info: SourceTextInfo,
    diagnostics: Vec<LintDiagnostic>,
  }

  let rules = if let Some(config) = maybe_config.clone() {
    config.get_rules()
  } else if let Some(rule_name) = filter_rule_name {
    let include = vec![rule_name.to_string()];
    get_filtered_rules(None, None, Some(include))
  } else {
    get_recommended_rules()
  };
  let plugins = Arc::new(
    plugin_paths
      .into_iter()
      .map(js::PluginRunner::new)
      .collect::<Vec<Box<dyn Plugin>>>(),
  );

  let file_diagnostics = Arc::new(Mutex::new(BTreeMap::new()));
  paths
    .par_iter()
    .try_for_each(|file_path| -> Result<(), AnyError> {
      let source_code = std::fs::read_to_string(&file_path)?;

      debug!("Configured rules: {}", rules.len());

      if rules.is_empty() {
        bail!("There's no rule to be run!");
      }

      let linter_builder = LinterBuilder::default()
        .rules(Arc::clone(&rules))
        .plugins(Arc::clone(&plugins))
        .syntax(determine_syntax(file_path));

      let linter = linter_builder.build();

      let (parsed_source, diagnostics) =
        linter.lint(file_path.to_string_lossy().to_string(), source_code)?;

      error_counts.fetch_add(diagnostics.len(), Ordering::Relaxed);

      let mut lock = file_diagnostics.lock().unwrap();

      lock.insert(
        file_path,
        FileDiagnostics {
          diagnostics,
          text_info: parsed_source.source().to_owned(),
        },
      );

      Ok(())
    })?;

  for d in file_diagnostics.lock().unwrap().values() {
    diagnostics::display_diagnostics(&d.diagnostics, &d.text_info);
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

/// Determine what syntax should be used as parse config from the file path.
fn determine_syntax(path: &Path) -> Syntax {
  match path.extension() {
    Some(os_str) => match os_str.to_str() {
      Some("ts") => get_default_ts_config(),
      Some("js") | Some("mjs") | Some("cjs") => get_default_es_config(),
      Some("tsx") => Syntax::Typescript(TsConfig {
        tsx: true,
        dynamic_import: true,
        decorators: true,
        ..Default::default()
      }),
      Some("jsx") => Syntax::Es(EsConfig {
        jsx: true,
        num_sep: true,
        class_private_props: false,
        class_private_methods: false,
        class_props: false,
        export_default_from: true,
        export_namespace_from: true,
        dynamic_import: true,
        nullish_coalescing: true,
        optional_chaining: true,
        import_meta: true,
        top_level_await: true,
        ..Default::default()
      }),
      _ => get_default_ts_config(),
    },
    None => get_default_ts_config(),
  }
}

fn main() -> Result<(), AnyError> {
  env_logger::init();

  let cli_app = create_cli_app();
  let matches = cli_app.get_matches();

  match matches.subcommand() {
    ("run", Some(run_matches)) => {
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

      let plugins: Vec<&str> = run_matches
        .values_of("PLUGIN")
        .unwrap_or_default()
        .collect();

      let paths: Vec<String> = run_matches
        .values_of("FILES")
        .unwrap_or_default()
        .map(|p| p.to_string())
        .collect();
      run_linter(
        paths,
        run_matches.value_of("RULE_CODE"),
        maybe_config,
        plugins,
      )?;
    }
    ("rules", Some(rules_matches)) => {
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
