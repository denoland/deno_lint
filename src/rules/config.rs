// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

//! Rule configuration: turning "some data" (severity + options, the eslint
//! `[severity, options]` shape) into runnable, configured rules.
//!
//! ## Why this exists / the design
//!
//! The original model conflated two distinct concepts into a single unit
//! struct registered as `Box<dyn LintRule>`:
//!
//!   * the *definition* of a rule — its code, tags, default severity, and how
//!     to build it from options; and
//!   * a *configured, runnable instance* of that rule.
//!
//! That conflation is why a `from_configuration(value) -> Self` constructor
//! can't live on `LintRule`: constructing `Self` isn't object-safe, so it
//! can't be reached through `Box<dyn LintRule>`. eslint (`{ meta, create }`)
//! and oxlint (a rule descriptor + `from_configuration`) both keep the two
//! halves apart. This module reintroduces that split:
//!
//!   * [`RuleDef`] is the *definition* — cheap, copyable-ish static metadata
//!     plus a `configure` function pointer ("options -> runnable rule").
//!   * [`ConfiguredRule`] is the runnable [`LintRule`] instance plus the
//!     [`LintDiagnosticSeverity`] its diagnostics should carry.
//!
//! Enablement and severity are unified the way eslint does it: a rule whose
//! effective [`RuleSeverity`] is `Off` is simply never built. `default_severity`
//! on the definition encodes "recommended rules are on by default".
//!
//! ## Performance / footprint (intentionally not yet optimal)
//!
//! This is a starter implementation; a few things to revisit soon:
//!
//!   * `RuleDef` holds only `&'static` data + a `fn` pointer, so the registry
//!     can become a `&'static [RuleDef]` with zero per-call allocation (better
//!     than today's `get_all_rules()` which rebuilds a `Vec` of boxed rules
//!     every call). The prototype builds a small `Vec` for convenience.
//!   * `configure` is called once per enabled rule per lint *session*, not per
//!     file, so option parsing cost is amortized across files.
//!   * `serde` derives add codegen, but only for the handful of rules that
//!     actually take options; option-less rules share [`no_options`], which is
//!     a single monomorphic function — no per-rule codegen.
//!   * Severity is currently applied as an O(diagnostics) post-pass in the
//!     linter; tagging at emit time would avoid the extra walk.
//!   * Hand-writing a `RuleDef` per rule won't scale to ~120 rules; a
//!     `declare_rules!` macro (or build-time codegen) should generate them.

use crate::diagnostic::LintDiagnosticSeverity;
use crate::rules::LintRule;
use crate::tags::Tags;
use std::borrow::Cow;
use std::collections::HashMap;

/// The severity a rule may be configured with.
///
/// Unlike [`LintDiagnosticSeverity`] (a property of an emitted diagnostic),
/// this includes `Off`: a rule configured `Off` is never constructed or run, so
/// there is nothing to emit. This mirrors eslint, where `"off"` both silences
/// and disables a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuleSeverity {
  Off,
  Warn,
  Error,
}

impl RuleSeverity {
  /// The diagnostic severity to stamp on this rule's output, or `None` when the
  /// rule is `Off` (and therefore should not run at all).
  fn diagnostic_severity(self) -> Option<LintDiagnosticSeverity> {
    match self {
      RuleSeverity::Off => None,
      RuleSeverity::Warn => Some(LintDiagnosticSeverity::Warning),
      RuleSeverity::Error => Some(LintDiagnosticSeverity::Error),
    }
  }
}

/// Per-rule configuration as it might arrive from a config file, the CLI, or the
/// LSP. Models eslint's `[severity, options]`: severity is optional (fall back
/// to the rule's default) and options are an opaque JSON blob the rule itself
/// knows how to interpret.
#[derive(Debug, Clone, Default)]
pub struct RuleConfig {
  /// `None` means "use the rule's `default_severity`".
  pub severity: Option<RuleSeverity>,
  /// Rule-specific options. `None` means "use option defaults".
  pub options: Option<serde_json::Value>,
}

impl RuleConfig {
  /// Convenience: just turn a rule on at its default severity.
  pub fn on() -> Self {
    RuleConfig {
      severity: Some(RuleSeverity::Error),
      options: None,
    }
  }
}

/// Failure to apply configuration to a rule.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleConfigError {
  /// Config referenced a rule code that isn't in the registry.
  UnknownRule { code: String },
  /// Options were supplied for a rule that takes none.
  DoesNotSupportOptions { code: &'static str },
  /// Options were supplied but failed to deserialize into the rule's schema.
  InvalidOptions { code: &'static str, message: String },
}

impl std::fmt::Display for RuleConfigError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      RuleConfigError::UnknownRule { code } => {
        write!(f, "Unknown lint rule '{code}'")
      }
      RuleConfigError::DoesNotSupportOptions { code } => {
        write!(f, "Lint rule '{code}' does not accept options")
      }
      RuleConfigError::InvalidOptions { code, message } => {
        write!(f, "Invalid options for lint rule '{code}': {message}")
      }
    }
  }
}

impl std::error::Error for RuleConfigError {}

/// A function that builds a configured, runnable rule from optional JSON
/// options (`None` = defaults). This is the constructor that the runnable
/// `LintRule` trait can't host because it isn't object-safe.
pub type ConfigureFn =
  fn(Option<&serde_json::Value>) -> Result<Box<dyn LintRule>, RuleConfigError>;

/// The *definition* half of a rule: static metadata plus a constructor.
///
/// The registry stores these instead of pre-built instances, so a rule is
/// "created" by applying configuration to its definition.
pub struct RuleDef {
  pub code: &'static str,
  pub tags: Tags,
  /// Severity used when a rule is enabled without an explicit severity. Encodes
  /// "on by default": recommended rules use `Error`, others use `Off`.
  pub default_severity: RuleSeverity,
  /// Builds the runnable instance from options. Severity is applied separately
  /// by [`RuleDef::configure`].
  pub configure_options: ConfigureFn,
}

impl RuleDef {
  /// Resolve this definition against user-supplied [`RuleConfig`] into a
  /// runnable [`ConfiguredRule`], or `Ok(None)` if the rule is effectively
  /// `Off`.
  pub fn configure(
    &self,
    config: &RuleConfig,
  ) -> Result<Option<ConfiguredRule>, RuleConfigError> {
    let severity = config.severity.unwrap_or(self.default_severity);
    let Some(diagnostic_severity) = severity.diagnostic_severity() else {
      return Ok(None);
    };
    let rule = (self.configure_options)(config.options.as_ref())?;
    Ok(Some(ConfiguredRule {
      rule,
      severity: diagnostic_severity,
    }))
  }
}

/// A runnable rule instance together with the severity its diagnostics carry.
#[derive(Debug)]
pub struct ConfiguredRule {
  pub rule: Box<dyn LintRule>,
  pub severity: LintDiagnosticSeverity,
}

/// Per-rule diagnostic severities, keyed by rule code. This is one of the two
/// inputs the linter consumes (alongside the runnable rules).
pub type RuleSeverities = HashMap<Cow<'static, str>, LintDiagnosticSeverity>;

/// A [`ConfigureFn`] for rules that take no options: errors if any options are
/// supplied, otherwise builds the default instance. Shared across all
/// option-less rules so they add no per-rule codegen.
pub fn no_options<R>(
  options: Option<&serde_json::Value>,
) -> Result<Box<dyn LintRule>, RuleConfigError>
where
  R: LintRule + Default + 'static,
{
  match options {
    // An explicit empty object/null is treated as "no options".
    None => Ok(Box::new(R::default())),
    Some(v) if v.is_null() => Ok(Box::new(R::default())),
    Some(v) if v.as_object().is_some_and(|o| o.is_empty()) => {
      Ok(Box::new(R::default()))
    }
    Some(_) => Err(RuleConfigError::DoesNotSupportOptions {
      code: R::default().code(),
    }),
  }
}

/// Resolve a whole registry of definitions against user configuration keyed by
/// rule code, producing the set of runnable rules (those not `Off`).
///
/// Unknown rule codes in `user` are reported rather than silently ignored.
pub fn configure_rules(
  defs: &[RuleDef],
  user: &HashMap<String, RuleConfig>,
) -> Result<Vec<ConfiguredRule>, RuleConfigError> {
  // Surface configuration for codes that don't exist.
  for code in user.keys() {
    if !defs.iter().any(|d| d.code == code) {
      return Err(RuleConfigError::UnknownRule { code: code.clone() });
    }
  }

  let default_config = RuleConfig::default();
  let mut configured = Vec::new();
  for def in defs {
    let config = user.get(def.code).unwrap_or(&default_config);
    if let Some(rule) = def.configure(config)? {
      configured.push(rule);
    }
  }
  Ok(configured)
}

/// Split configured rules into the two inputs `LinterOptions` wants: the
/// runnable rules and the per-code severity map. This is the bridge from "rule
/// configuration" to "linter input".
pub fn split_configured(
  configured: Vec<ConfiguredRule>,
) -> (Vec<Box<dyn LintRule>>, RuleSeverities) {
  let mut rules = Vec::with_capacity(configured.len());
  let mut severities = HashMap::with_capacity(configured.len());
  for configured_rule in configured {
    // `code()` returns `&'static str`, so this borrows nothing from the rule.
    severities.insert(
      Cow::Borrowed(configured_rule.rule.code()),
      configured_rule.severity,
    );
    rules.push(configured_rule.rule);
  }
  (rules, severities)
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::rules::eqeqeq::Eqeqeq;
  use crate::rules::no_console::NoConsole;
  use crate::rules::no_empty::NoEmpty;

  fn registry() -> Vec<RuleDef> {
    vec![NoConsole::def(), NoEmpty::def(), Eqeqeq::def()]
  }

  fn json(s: &str) -> serde_json::Value {
    serde_json::from_str(s).unwrap()
  }

  #[test]
  fn recommended_default_on_others_off() {
    // With no user config, only rules whose `default_severity` isn't `Off`
    // (here: the recommended `no-empty`) are created.
    let configured = configure_rules(&registry(), &HashMap::new()).unwrap();
    let codes: Vec<_> = configured.iter().map(|c| c.rule.code()).collect();
    assert_eq!(codes, vec!["no-empty"]);
    assert_eq!(configured[0].severity, LintDiagnosticSeverity::Error);
  }

  #[test]
  fn severity_off_excludes_rule() {
    let mut user = HashMap::new();
    user.insert(
      "no-empty".to_string(),
      RuleConfig {
        severity: Some(RuleSeverity::Off),
        options: None,
      },
    );
    let configured = configure_rules(&registry(), &user).unwrap();
    assert!(configured.is_empty());
  }

  #[test]
  fn severity_warn_is_carried() {
    let mut user = HashMap::new();
    user.insert(
      "no-console".to_string(),
      RuleConfig {
        severity: Some(RuleSeverity::Warn),
        options: None,
      },
    );
    let configured = configure_rules(&registry(), &user).unwrap();
    let c = configured
      .iter()
      .find(|c| c.rule.code() == "no-console")
      .unwrap();
    assert_eq!(c.severity, LintDiagnosticSeverity::Warning);
  }

  #[test]
  fn unknown_rule_is_an_error() {
    let mut user = HashMap::new();
    user.insert("no-such-rule".to_string(), RuleConfig::on());
    let err = configure_rules(&registry(), &user).unwrap_err();
    assert_eq!(
      err,
      RuleConfigError::UnknownRule {
        code: "no-such-rule".to_string()
      }
    );
  }

  #[test]
  fn options_for_optionless_rule_error() {
    // `no-empty` accepts options, but feeding garbage to an option-less rule
    // (simulated via the shared `no_options` path) is rejected. Here we feed
    // an unexpected shape to a rule and expect an invalid-options error.
    let mut user = HashMap::new();
    user.insert(
      "eqeqeq".to_string(),
      RuleConfig {
        severity: Some(RuleSeverity::Error),
        options: Some(json(r#""nonsense-mode""#)),
      },
    );
    let err = configure_rules(&registry(), &user).unwrap_err();
    assert!(matches!(
      err,
      RuleConfigError::InvalidOptions { code: "eqeqeq", .. }
    ));
  }
}
