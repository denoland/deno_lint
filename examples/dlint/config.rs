// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use anyhow::bail;
use anyhow::Error as AnyError;
use deno_lint::rules::{get_all_rules, LintRule};
use serde::Deserialize;
use std::path::Path;
use std::path::PathBuf;

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct RulesConfig {
  pub tags: Vec<String>,
  pub include: Vec<String>,
  pub exclude: Vec<String>,
}
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct FilesConfig {
  pub include: Vec<String>,
  pub exclude: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct Config {
  pub rules: RulesConfig,
  pub files: FilesConfig,
}

impl Config {
  pub fn get_rules(&self) -> Vec<Box<dyn LintRule>> {
    let mut rules = get_all_rules();

    if !self.rules.tags.is_empty() {
      rules = rules
        .into_iter()
        .filter(|rule| {
          for tag in rule.tags().to_owned() {
            if self.rules.tags.contains(&tag.to_string()) {
              return true;
            }
          }
          false
        })
        .collect();
    }

    if !self.rules.exclude.is_empty() {
      rules = rules
        .into_iter()
        .filter(|rule| !self.rules.exclude.contains(&rule.code().to_string()))
        .collect();
    }

    if !self.rules.include.is_empty() {
      for include_rule in &self.rules.include {
        if let Some(rule) = get_all_rules()
          .into_iter()
          .find(|rule| rule.code() == include_rule)
        {
          rules.push(rule);
        }
      }
    }

    rules
  }

  pub fn get_files(&self) -> Result<Vec<PathBuf>, AnyError> {
    resolve_file_paths(&self.files)
  }
}

pub fn load_from_json(config_path: &Path) -> Result<Config, std::io::Error> {
  let json_str = std::fs::read_to_string(config_path)?;
  let config: Config = serde_json::from_str(&json_str)?;
  Ok(config)
}

pub fn load_from_toml(config_path: &Path) -> Result<Config, std::io::Error> {
  let toml_str = std::fs::read_to_string(config_path)?;
  let config: Config = toml::from_str(&toml_str)?;
  Ok(config)
}

// Ported from dprint
// https://github.com/dprint/dprint/blob/358c91fbf0a545a0c9736cc496dc1d998028ae65/crates/dprint/src/cli/run_cli.rs#L686-L756
fn resolve_file_paths(config: &FilesConfig) -> Result<Vec<PathBuf>, AnyError> {
  let mut file_patterns = get_file_patterns(config);
  let absolute_paths = take_absolute_paths(&mut file_patterns);

  let cwd = std::env::current_dir()?;
  let mut file_paths = glob(&cwd, &file_patterns)?;
  file_paths.extend(absolute_paths);
  return Ok(file_paths);

  fn get_file_patterns(config: &FilesConfig) -> Vec<String> {
    let mut file_patterns = Vec::new();

    file_patterns.extend(config.include.clone());

    file_patterns.extend(config.exclude.clone().into_iter().map(|exclude| {
      if exclude.starts_with('!') {
        exclude
      } else {
        format!("!{}", exclude)
      }
    }));

    // glob walker doesn't support having `./` at the front of paths, so just remove them when they appear
    for file_pattern in file_patterns.iter_mut() {
      if file_pattern.starts_with("./") {
        *file_pattern = String::from(&file_pattern[2..]);
      }
      if file_pattern.starts_with("!./") {
        *file_pattern = format!("!{}", &file_pattern[3..]);
      }
    }

    file_patterns
  }

  fn take_absolute_paths(file_patterns: &mut Vec<String>) -> Vec<PathBuf> {
    let len = file_patterns.len();
    let mut file_paths = Vec::new();
    for i in (0..len).rev() {
      if is_absolute_path(&file_patterns[i]) {
        file_paths.push(PathBuf::from(file_patterns.swap_remove(i))); // faster
      }
    }
    file_paths
  }

  fn is_absolute_path(file_pattern: &str) -> bool {
    return !has_glob_chars(file_pattern)
      && PathBuf::from(file_pattern).is_absolute();

    fn has_glob_chars(text: &str) -> bool {
      for c in text.chars() {
        match c {
          '*' | '{' | '}' | '[' | ']' | '!' => return true,
          _ => {}
        }
      }

      false
    }
  }
}

// Ported from dprint
// https://github.com/dprint/dprint/blob/358c91fbf0a545a0c9736cc496dc1d998028ae65/crates/dprint/src/environment/real_environment.rs#L99-L123
fn glob(
  base: &Path,
  file_patterns: &[String],
) -> Result<Vec<PathBuf>, AnyError> {
  let base = base.canonicalize()?;
  let walker = globwalk::GlobWalkerBuilder::from_patterns(base, file_patterns)
    .follow_links(false)
    .file_type(globwalk::FileType::FILE)
    .build();
  let walker = match walker {
    Ok(walker) => walker,
    Err(err) => bail!("Error parsing file patterns: {}", err),
  };

  let mut file_paths = Vec::new();
  for result in walker.into_iter() {
    match result {
      Ok(result) => file_paths.push(result.into_path()),
      Err(err) => bail!("Error walking files: {}", err),
    }
  }

  Ok(file_paths)
}
