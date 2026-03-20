use crate::context::Context;
use crate::diagnostic::{LintFix, LintFixChange};
use deno_ast::{ModuleSpecifier, SourceRange, SourceTextInfo};
use deno_semver::jsr::JsrPackageReqReference;
use deno_semver::npm::NpmPackageReqReference;
use jsonc_parser::cst::{CstInputValue, CstLeafNode, CstNode, CstRootNode};
use jsonc_parser::ParseOptions;
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct ImportMapEntry {
  key: String,
  value: String,
}

#[derive(Clone, Debug)]
struct DenoConfigFile {
  specifier: ModuleSpecifier,
  text: String,
  imports: Vec<ImportMapEntry>,
  lockfile: Option<LockfileData>,
}

#[derive(Clone, Debug, Default)]
struct LockfileData {
  resolved_packages: Vec<ResolvedPackageVersion>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ResolvedPackageVersion {
  kind: PackageKind,
  name: String,
  version: String,
}

pub fn fix_with_deno_config_import(
  ctx: &Context,
  range: SourceRange,
  specifier_text: &str,
  description: &'static str,
) -> Option<LintFix> {
  let package_specifier = PackageSpecifier::parse(specifier_text);
  if package_specifier
    .as_ref()
    .is_some_and(PackageSpecifier::is_unversioned)
  {
    return None;
  }
  fix_with_deno_config(
    ctx,
    range,
    specifier_text,
    description,
    package_specifier,
  )
}

pub fn fix_with_deno_config_package(
  ctx: &Context,
  range: SourceRange,
  specifier_text: &str,
  description: &'static str,
) -> Option<LintFix> {
  fix_with_deno_config(
    ctx,
    range,
    specifier_text,
    description,
    Some(PackageSpecifier::parse(specifier_text)?),
  )
}

fn fix_with_deno_config(
  ctx: &Context,
  range: SourceRange,
  specifier_text: &str,
  description: &'static str,
  maybe_package_specifier: Option<PackageSpecifier>,
) -> Option<LintFix> {
  let config = DenoConfigFile::for_source_file(ctx.specifier())?;

  if let Some(replacement) =
    rewrite_with_import_map_target(specifier_text, &config.imports)
  {
    return Some(source_only_fix(range, replacement, description));
  }

  if let Some(replacement) =
    rewrite_with_package_match(specifier_text, &config.imports)
  {
    return Some(source_only_fix(range, replacement, description));
  }

  let package_specifier = maybe_package_specifier?;
  let suggested_import =
    package_specifier.suggested_import(config.lockfile.as_ref()?)?;

  if config
    .imports
    .iter()
    .any(|entry| entry.key == suggested_import.key)
  {
    return None;
  }

  let config_text = add_import_to_config(&config.text, &suggested_import)?;

  Some(LintFix {
    description: description.into(),
    changes: vec![
      source_change(range, suggested_import.replacement.clone()),
      config_change(&config.specifier, &config.text, config_text),
    ],
  })
}

fn source_only_fix(
  range: SourceRange,
  replacement: String,
  description: &'static str,
) -> LintFix {
  LintFix {
    description: description.into(),
    changes: vec![source_change(range, replacement)],
  }
}

fn source_change(range: SourceRange, replacement: String) -> LintFixChange {
  LintFixChange {
    specifier: None,
    new_text: format!("\"{}\"", replacement).into(),
    range,
  }
}

fn config_change(
  specifier: &ModuleSpecifier,
  old_text: &str,
  new_text: String,
) -> LintFixChange {
  let text_info = SourceTextInfo::new(old_text.into());
  let range = text_info.range();
  LintFixChange {
    specifier: Some(specifier.clone()),
    new_text: new_text.into(),
    range: SourceRange::new(range.start.as_source_pos(), range.end),
  }
}

impl DenoConfigFile {
  fn for_source_file(source_specifier: &ModuleSpecifier) -> Option<Self> {
    let source_path = source_specifier.to_file_path().ok()?;
    let config_path = find_nearest_deno_config(&source_path)?;
    let text = fs::read_to_string(&config_path).ok()?;
    let specifier = ModuleSpecifier::from_file_path(&config_path).ok()?;
    let config_value =
      jsonc_parser::parse_to_serde_value::<Value>(&text, &Default::default())
        .ok()
        .flatten();
    Some(Self {
      imports: load_imports(&text),
      lockfile: resolve_lockfile(&config_path, config_value.as_ref()),
      specifier,
      text,
    })
  }
}

fn find_nearest_deno_config(file_path: &Path) -> Option<PathBuf> {
  for dir in file_path.parent()?.ancestors() {
    let deno_json = dir.join("deno.json");
    if deno_json.exists() {
      return Some(deno_json);
    }

    let deno_jsonc = dir.join("deno.jsonc");
    if deno_jsonc.exists() {
      return Some(deno_jsonc);
    }
  }

  None
}

fn load_imports(text: &str) -> Vec<ImportMapEntry> {
  let Ok(Some(value)) =
    jsonc_parser::parse_to_serde_value::<Value>(text, &Default::default())
  else {
    return Vec::new();
  };
  let Some(imports) = value.get("imports").and_then(|value| value.as_object())
  else {
    return Vec::new();
  };
  imports
    .iter()
    .filter_map(|(key, value)| {
      value.as_str().map(|value| ImportMapEntry {
        key: key.to_string(),
        value: value.to_string(),
      })
    })
    .collect()
}

fn resolve_lockfile(
  config_path: &Path,
  config_value: Option<&Value>,
) -> Option<LockfileData> {
  let lockfile_path = lockfile_path(config_path, config_value)?;
  let text = fs::read_to_string(lockfile_path).ok()?;
  parse_lockfile(&text)
}

fn lockfile_path(
  config_path: &Path,
  config_value: Option<&Value>,
) -> Option<PathBuf> {
  let config_dir = config_path.parent()?;
  let lock_value = config_value.and_then(|value| value.get("lock"));

  match lock_value {
    Some(Value::Bool(false)) => None,
    Some(Value::String(path)) => Some(config_dir.join(path)),
    Some(Value::Object(object)) => match object.get("path") {
      Some(Value::String(path)) => Some(config_dir.join(path)),
      Some(_) => None,
      None => Some(config_dir.join("deno.lock")),
    },
    Some(_) | None => Some(config_dir.join("deno.lock")),
  }
}

fn parse_lockfile(text: &str) -> Option<LockfileData> {
  let value = serde_json::from_str::<Value>(text).ok()?;
  let root = value.as_object()?;

  let mut resolved_packages = Vec::new();

  if let Some(specifiers) =
    root.get("specifiers").and_then(|value| value.as_object())
  {
    for value in specifiers.values() {
      let Some(value) = value.as_str() else {
        continue;
      };
      if let Some(package) = resolved_package_from_lockfile_value(value) {
        resolved_packages.push(package);
      }
    }
  }

  if let Some(jsr) = root.get("jsr").and_then(|value| value.as_object()) {
    for key in jsr.keys() {
      if let Some(package) =
        resolved_package_from_lockfile_value(&format!("jsr:{key}"))
      {
        resolved_packages.push(package);
      }
    }
  }

  if let Some(npm) = root.get("npm").and_then(|value| value.as_object()) {
    for key in npm.keys() {
      if let Some(package) =
        resolved_package_from_lockfile_value(&format!("npm:{key}"))
      {
        resolved_packages.push(package);
      }
    }
  }

  Some(LockfileData { resolved_packages })
}

fn resolved_package_from_lockfile_value(
  value: &str,
) -> Option<ResolvedPackageVersion> {
  let normalized = if let Some(value) = value.strip_prefix("npm:") {
    format!(
      "npm:{}",
      value
        .split_once('_')
        .map(|(value, _)| value)
        .unwrap_or(value)
    )
  } else {
    value.to_string()
  };
  let package = PackageSpecifier::parse(&normalized)?;
  Some(ResolvedPackageVersion {
    kind: package.kind,
    name: package.name,
    version: package.version_req?,
  })
}

fn add_import_to_config(
  text: &str,
  import: &SuggestedImport,
) -> Option<String> {
  let root = CstRootNode::parse(text, &ParseOptions::default()).ok()?;
  let root_object = root.object_value_or_create()?;
  let imports_object = root_object.object_value_or_create("imports")?;

  if let Some(existing_prop) = imports_object.get(&import.key) {
    let existing_value = existing_prop.value()?;
    let CstNode::Leaf(leaf) = existing_value else {
      return None;
    };
    let CstLeafNode::StringLit(string_lit) = leaf else {
      return None;
    };
    if string_lit.decoded_value().ok()?.as_str() == import.value.as_str() {
      return Some(text.to_string());
    }
    return None;
  }

  imports_object
    .append(&import.key, CstInputValue::String(import.value.clone()));
  Some(root.to_string())
}

fn rewrite_with_import_map_target(
  specifier_text: &str,
  entries: &[ImportMapEntry],
) -> Option<String> {
  entries
    .iter()
    .filter_map(|entry| {
      exact_or_prefix_match(specifier_text, entry)
        .map(|replacement| (entry.value.len(), replacement))
    })
    .max_by_key(|(match_len, _)| *match_len)
    .map(|(_, replacement)| replacement)
}

fn exact_or_prefix_match(
  specifier_text: &str,
  entry: &ImportMapEntry,
) -> Option<String> {
  if specifier_text == entry.value {
    return Some(entry.key.trim_end_matches('/').to_string());
  }

  if entry.key.ends_with('/')
    && entry.value.ends_with('/')
    && specifier_text.starts_with(&entry.value)
  {
    return Some(format!(
      "{}{}",
      entry.key,
      &specifier_text[entry.value.len()..]
    ));
  }

  None
}

fn rewrite_with_package_match(
  specifier_text: &str,
  entries: &[ImportMapEntry],
) -> Option<String> {
  let input = PackageSpecifier::parse(specifier_text)?;
  entries
    .iter()
    .filter_map(|entry| {
      let target = PackageSpecifier::parse(entry.value.trim_end_matches('/'))?;
      if input.kind != target.kind || input.name != target.name {
        return None;
      }
      if target.sub_path.is_some() {
        return None;
      }
      Some((
        entry.key.len(),
        package_replacement(entry.key.as_str(), input.sub_path.as_deref()),
      ))
    })
    .max_by_key(|(match_len, _)| *match_len)
    .map(|(_, replacement)| replacement)
}

fn package_replacement(key: &str, sub_path: Option<&str>) -> String {
  let base = key.trim_end_matches('/');
  match sub_path {
    Some(sub_path) if key.ends_with('/') => format!("{}{}", key, sub_path),
    Some(sub_path) => format!("{}/{}", base, sub_path),
    None => base.to_string(),
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PackageSpecifier {
  kind: PackageKind,
  name: String,
  version_req: Option<String>,
  sub_path: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SuggestedImport {
  key: String,
  replacement: String,
  value: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PackageKind {
  Jsr,
  Npm,
}

impl PackageKind {
  fn scheme(self) -> &'static str {
    match self {
      Self::Jsr => "jsr:",
      Self::Npm => "npm:",
    }
  }
}

impl PackageSpecifier {
  fn parse(specifier_text: &str) -> Option<Self> {
    if let Ok(req_ref) = JsrPackageReqReference::from_str(specifier_text) {
      return Some(Self::from_req_ref(
        PackageKind::Jsr,
        req_ref.req().name.as_str(),
        req_ref.req().version_req.version_text(),
        req_ref.sub_path(),
      ));
    }

    if let Ok(req_ref) = NpmPackageReqReference::from_str(specifier_text) {
      return Some(Self::from_req_ref(
        PackageKind::Npm,
        req_ref.req().name.as_str(),
        req_ref.req().version_req.version_text(),
        req_ref.sub_path(),
      ));
    }

    None
  }

  fn from_req_ref(
    kind: PackageKind,
    name: &str,
    version_text: &str,
    sub_path: Option<&str>,
  ) -> Self {
    Self {
      kind,
      name: name.to_string(),
      version_req: (version_text != "*").then(|| version_text.to_string()),
      sub_path: sub_path.map(ToString::to_string),
    }
  }

  fn is_unversioned(&self) -> bool {
    self.version_req.is_none()
  }

  fn suggested_import(
    &self,
    lockfile: &LockfileData,
  ) -> Option<SuggestedImport> {
    let version_req = match &self.version_req {
      Some(version_req) => lockfile
        .has_package(self.kind, &self.name)
        .then_some(version_req.as_str())?,
      None => lockfile.unique_version(self.kind, &self.name)?,
    };
    let key = if self.sub_path.is_some() {
      format!("{}/", self.name)
    } else {
      self.name.clone()
    };
    let replacement = package_replacement(&key, self.sub_path.as_deref());
    let value = if self.sub_path.is_some() {
      format!("{}{}@{}/", self.kind.scheme(), self.name, version_req)
    } else {
      format!("{}{}@{}", self.kind.scheme(), self.name, version_req)
    };
    Some(SuggestedImport {
      key,
      replacement,
      value,
    })
  }
}

impl LockfileData {
  fn has_package(&self, kind: PackageKind, name: &str) -> bool {
    self
      .resolved_packages
      .iter()
      .any(|package| package.kind == kind && package.name == name)
  }

  fn unique_version(&self, kind: PackageKind, name: &str) -> Option<&str> {
    let mut matches = self
      .resolved_packages
      .iter()
      .filter(|package| package.kind == kind && package.name == name)
      .map(|package| package.version.as_str())
      .collect::<Vec<_>>();
    matches.sort_unstable();
    matches.dedup();
    if matches.len() == 1 {
      matches.into_iter().next()
    } else {
      None
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  fn entry(key: &str, value: &str) -> ImportMapEntry {
    ImportMapEntry {
      key: key.to_string(),
      value: value.to_string(),
    }
  }

  #[test]
  fn rewrites_exact_import_map_targets() {
    let entries = vec![entry("@std/assert", "jsr:@std/assert@^1.0.0")];
    assert_eq!(
      rewrite_with_import_map_target("jsr:@std/assert@^1.0.0", &entries),
      Some("@std/assert".to_string())
    );
  }

  #[test]
  fn rewrites_import_map_prefix_targets() {
    let entries = vec![entry("@std/assert/", "jsr:@std/assert@^1.0.0/")];
    assert_eq!(
      rewrite_with_import_map_target(
        "jsr:@std/assert@^1.0.0/fmt/colors.ts",
        &entries
      ),
      Some("@std/assert/fmt/colors.ts".to_string())
    );
  }

  #[test]
  fn rewrites_package_matches_ignoring_versions() {
    let entries = vec![entry("@std/assert", "jsr:@std/assert@^1.0.0")];
    assert_eq!(
      rewrite_with_package_match("jsr:@std/assert", &entries),
      Some("@std/assert".to_string())
    );
    assert_eq!(
      rewrite_with_package_match("jsr:@std/assert/fmt/colors.ts", &entries),
      Some("@std/assert/fmt/colors.ts".to_string())
    );
  }

  #[test]
  fn uses_lockfile_version_for_unversioned_package() {
    let package = PackageSpecifier::parse("jsr:@std/expect").unwrap();
    let lockfile = parse_lockfile(
      r#"{
  "version": "5",
  "specifiers": {
    "jsr:@std/expect@*": "jsr:@std/expect@1.0.0"
  }
}"#,
    )
    .unwrap();
    assert_eq!(
      package.suggested_import(&lockfile),
      Some(SuggestedImport {
        key: "@std/expect".to_string(),
        replacement: "@std/expect".to_string(),
        value: "jsr:@std/expect@1.0.0".to_string(),
      })
    );
  }

  #[test]
  fn uses_lockfile_version_for_sub_path() {
    let package = PackageSpecifier::parse("jsr:@std/expect/colors.ts").unwrap();
    let lockfile = parse_lockfile(
      r#"{
  "version": "5",
  "jsr": {
    "@std/expect@1.0.0": {
      "integrity": "test"
    }
  }
}"#,
    )
    .unwrap();
    assert_eq!(
      package.suggested_import(&lockfile),
      Some(SuggestedImport {
        key: "@std/expect/".to_string(),
        replacement: "@std/expect/colors.ts".to_string(),
        value: "jsr:@std/expect@1.0.0/".to_string(),
      })
    );
  }

  #[test]
  fn requires_unique_lockfile_version() {
    let package = PackageSpecifier::parse("jsr:@std/expect").unwrap();
    let lockfile = parse_lockfile(
      r#"{
  "version": "5",
  "jsr": {
    "@std/expect@1.0.0": {
      "integrity": "test"
    },
    "@std/expect@1.1.0": {
      "integrity": "test"
    }
  }
}"#,
    )
    .unwrap();
    assert_eq!(package.suggested_import(&lockfile), None);
  }

  #[test]
  fn adds_import_to_jsonc_file() {
    let updated = add_import_to_config(
      "{\n  // comment\n}\n",
      &SuggestedImport {
        key: "@std/expect".to_string(),
        replacement: "@std/expect".to_string(),
        value: "jsr:@std/expect@^1".to_string(),
      },
    )
    .unwrap();

    assert!(updated.contains("// comment"));
    assert!(updated.contains("\"imports\": {"));
    assert!(updated.contains("\"@std/expect\": \"jsr:@std/expect@^1\""));
  }
}
