use deno_lint::rules::get_all_rules;
use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub struct Rule {
  code: &'static str,
  docs: &'static str,
  tags: &'static [&'static str],
}

pub fn get_all_rules_metadata() -> Vec<Rule> {
  get_all_rules()
    .into_iter()
    .map(|rule| Rule {
      code: rule.code(),
      docs: rule.docs(),
      tags: rule.tags(),
    })
    .collect()
}

pub fn get_specific_rule_metadata(rule_name: &str) -> Vec<Rule> {
  get_all_rules_metadata()
    .into_iter()
    .filter(|r| r.code == rule_name)
    .collect()
}

pub fn print_rules<F: RuleFormatter>(mut rules: Vec<Rule>) {
  match F::format(&mut rules) {
    Err(e) => {
      eprintln!("{}", e);
      std::process::exit(1);
    }
    Ok(text) => {
      println!("{}", text);
    }
  }
}

pub enum JsonFormatter {}
pub enum PrettyFormatter {}

pub trait RuleFormatter {
  fn format(rules: &mut [Rule]) -> Result<String, &'static str>;
}

impl RuleFormatter for JsonFormatter {
  fn format(rules: &mut [Rule]) -> Result<String, &'static str> {
    if rules.is_empty() {
      return Err("Rule not found!");
    }
    serde_json::to_string_pretty(rules).map_err(|_| "failed to format!")
  }
}

impl RuleFormatter for PrettyFormatter {
  fn format(rules: &mut [Rule]) -> Result<String, &'static str> {
    if rules.is_empty() {
      return Err("Rule not found!");
    }

    if rules.len() == 1 {
      let rule = &rules[0];
      let docs = if rule.docs.is_empty() {
        "documentation not available"
      } else {
        rule.docs
      };
      return Ok(format!("- {code}\n\n{docs}", code = rule.code, docs = docs));
    }

    rules.sort_by_key(|r| r.code);
    let mut list = Vec::with_capacity(1 + rules.len());
    list.push("Available rules (trailing ✔️ mark indicates it is included in the recommended rule set):".to_string());
    list.extend(rules.iter().map(|r| {
      let mut s = format!(" - {}", r.code);
      if r.tags.contains(&"recommended") {
        s += " ✔️";
      }
      s
    }));
    Ok(list.join("\n"))
  }
}
