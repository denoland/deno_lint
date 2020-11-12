// Copyright 2020 the Deno authors. All rights reserved. MIT license.
use deno_lint::rules::{get_all_rules, LintRule};
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct RulesConfig {
  pub tags: Vec<String>,
  pub include: Vec<String>,
  pub exclude: Vec<String>,
}
#[derive(Debug, Deserialize)]
pub struct FilesConfig {
  pub include: Vec<String>,
  pub exclude: Vec<String>,
}
#[derive(Debug, Deserialize)]
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
}

pub fn load_from_json(config_path: &Path) -> Result<Config, std::io::Error> {
  let json_str = std::fs::read_to_string(config_path)?;
  let config: Config = serde_json::from_str(&json_str)?;
  Ok(config)
}
