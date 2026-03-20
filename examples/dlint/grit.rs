// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::BTreeMap;
use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Context;
use anyhow::Error as AnyError;
use deno_ast::ModuleSpecifier;
use deno_ast::SourceRange;
use deno_ast::SourceTextInfo;
use deno_lint::diagnostic::LintDiagnostic;
use deno_lint::diagnostic::LintDiagnosticDetails;
use deno_lint::diagnostic::LintDiagnosticRange;
use deno_lint::diagnostic::LintDocsUrl;
use marzano_core::api::MatchReason;
use marzano_core::api::MatchResult;
use marzano_core::pattern_compiler::src_to_problem_libs;
use marzano_core::problem::Problem;
use marzano_language::target_language::PatternLanguage;
use marzano_language::target_language::TargetLanguage;
use marzano_util::rich_path::RichFile;
use marzano_util::runtime::ExecutionContext;

#[derive(Clone, Debug)]
pub struct GritOptions {
  pub patterns: Vec<String>,
}

#[derive(Debug)]
struct CompiledPattern {
  code: String,
  language: PatternLanguage,
  pattern: String,
  problem: Problem,
}

#[derive(Debug)]
pub struct GritSession {
  patterns: Vec<CompiledPattern>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum FileLanguage {
  Js,
  Json,
}

#[derive(Clone, Debug)]
struct PlannedFileChange {
  path: PathBuf,
  original_text: String,
  updated_text: String,
}

impl GritSession {
  pub fn new(options: GritOptions) -> Result<Self, AnyError> {
    let default_language = default_target_language()?;
    let mut patterns = Vec::new();

    for configured_pattern in configure_patterns(options.patterns) {
      let compiled = src_to_problem_libs(
        configured_pattern.pattern.clone(),
        &BTreeMap::new(),
        default_language,
        None,
        None,
        None,
        None,
      )
      .with_context(|| {
        format!(
          "Failed to compile GritQL pattern '{}'",
          configured_pattern.pattern,
        )
      })?;

      patterns.push(CompiledPattern {
        code: configured_pattern.code,
        language: PatternLanguage::from(&compiled.problem.language),
        pattern: configured_pattern.pattern,
        problem: compiled.problem,
      });
    }

    Ok(Self { patterns })
  }

  pub fn collect_diagnostics(
    &self,
    file_path: &Path,
    source_code: &str,
  ) -> Result<Vec<LintDiagnostic>, AnyError> {
    let mut diagnostics = Vec::new();

    for pattern in self.applicable_patterns(file_path) {
      let results = pattern.execute(file_path, source_code)?;
      diagnostics.extend(pattern.diagnostics_from_results(
        file_path,
        source_code,
        &results,
      )?);
    }

    Ok(diagnostics)
  }

  pub fn apply_fixes(&self, file_paths: &[PathBuf]) -> Result<(), AnyError> {
    let mut planned_changes = Vec::new();

    for file_path in file_paths {
      let Some(file_language) = detect_file_language(file_path) else {
        continue;
      };

      let original_text = fs::read_to_string(file_path).with_context(|| {
        format!("Failed to read '{}' for GritQL fixes", file_path.display())
      })?;
      let mut current_text = original_text.clone();

      for pattern in self.patterns.iter().filter(|pattern| {
        matches_pattern_language(pattern.language, file_language)
      }) {
        let results = pattern.execute(file_path, &current_text)?;
        if let Some(next_text) = pattern.extract_supported_rewrite(
          file_path,
          &current_text,
          &results,
        )? {
          current_text = next_text;
        }
      }

      if current_text != original_text {
        planned_changes.push(PlannedFileChange {
          path: file_path.clone(),
          original_text,
          updated_text: current_text,
        });
      }
    }

    write_planned_changes(&planned_changes)
  }

  fn applicable_patterns(
    &self,
    file_path: &Path,
  ) -> impl Iterator<Item = &CompiledPattern> {
    let file_language = detect_file_language(file_path);
    self.patterns.iter().filter(move |pattern| {
      file_language
        .map(|file_language| {
          matches_pattern_language(pattern.language, file_language)
        })
        .unwrap_or(false)
    })
  }
}

impl CompiledPattern {
  fn execute(
    &self,
    file_path: &Path,
    source_code: &str,
  ) -> Result<Vec<MatchResult>, AnyError> {
    let file = RichFile::new(
      file_path.to_string_lossy().to_string(),
      source_code.to_string(),
    );
    let context = ExecutionContext::default();
    let results = self.problem.execute_files(vec![file], &context);
    ensure_no_runtime_errors(&results, &self.pattern)?;
    Ok(results)
  }

  fn diagnostics_from_results(
    &self,
    file_path: &Path,
    source_code: &str,
    results: &[MatchResult],
  ) -> Result<Vec<LintDiagnostic>, AnyError> {
    let mut diagnostics = Vec::new();

    for result in results {
      match result {
        MatchResult::Match(item) => diagnostics.push(create_diagnostic(
          &self.code,
          &item.source_file,
          item.content.as_deref().unwrap_or(source_code),
          item
            .ranges
            .first()
            .map(|range| (range.start_byte as usize, range.end_byte as usize)),
          item.reason.as_ref(),
          "Matched GritQL pattern",
        )?),
        MatchResult::Rewrite(item) => {
          diagnostics.push(create_diagnostic(
            &self.code,
            &item.original.source_file,
            item.original.content.as_deref().unwrap_or(source_code),
            item.original.ranges.first().map(|range| {
              (range.start_byte as usize, range.end_byte as usize)
            }),
            item.reason.as_ref(),
            rewrite_fallback_message(file_path, item),
          )?)
        }
        MatchResult::CreateFile(item) => diagnostics.push(create_diagnostic(
          &self.code,
          &item.rewritten.source_file,
          item.rewritten.content.as_deref().unwrap_or_default(),
          None,
          item.reason.as_ref(),
          "GritQL wants to create this file",
        )?),
        MatchResult::RemoveFile(item) => {
          diagnostics.push(create_diagnostic(
            &self.code,
            &item.original.source_file,
            item.original.content.as_deref().unwrap_or_default(),
            item.original.ranges.first().map(|range| {
              (range.start_byte as usize, range.end_byte as usize)
            }),
            item.reason.as_ref(),
            "GritQL wants to remove this file",
          )?)
        }
        MatchResult::PatternInfo(_)
        | MatchResult::AllDone(_)
        | MatchResult::InputFile(_)
        | MatchResult::DoneFile(_)
        | MatchResult::AnalysisLog(_) => {}
      }
    }

    Ok(diagnostics)
  }

  fn extract_supported_rewrite(
    &self,
    file_path: &Path,
    current_text: &str,
    results: &[MatchResult],
  ) -> Result<Option<String>, AnyError> {
    let current_path = file_path.to_string_lossy();
    let mut rewrite_text = None;

    for result in results {
      match result {
        MatchResult::Rewrite(item) => {
          if item.original.source_file != current_path
            || item.rewritten.source_file != current_path
          {
            bail!(
              "Pattern '{}' produced an unsupported cross-file rewrite: '{}' -> '{}'",
              self.pattern,
              item.original.source_file,
              item.rewritten.source_file,
            );
          }

          if let Some(original_content) = item.original.content.as_deref() {
            if original_content != current_text {
              bail!(
                "Pattern '{}' produced a rewrite for '{}' using stale source text",
                self.pattern,
                current_path,
              );
            }
          }

          let next_text = item.rewritten.content.clone().ok_or_else(|| {
            anyhow!(
              "Pattern '{}' produced a rewrite without rewritten content",
              self.pattern,
            )
          })?;

          match rewrite_text.as_ref() {
            Some(existing_text) if existing_text != &next_text => {
              bail!(
                "Pattern '{}' produced conflicting rewrites for '{}'",
                self.pattern,
                current_path,
              );
            }
            Some(_) => {}
            None => {
              rewrite_text = Some(next_text);
            }
          }
        }
        MatchResult::CreateFile(_) => {
          bail!(
            "Pattern '{}' produced an unsupported file creation effect during --fix",
            self.pattern,
          );
        }
        MatchResult::RemoveFile(_) => {
          bail!(
            "Pattern '{}' produced an unsupported file removal effect during --fix",
            self.pattern,
          );
        }
        MatchResult::PatternInfo(_)
        | MatchResult::AllDone(_)
        | MatchResult::InputFile(_)
        | MatchResult::DoneFile(_)
        | MatchResult::AnalysisLog(_)
        | MatchResult::Match(_) => {}
      }
    }

    Ok(rewrite_text)
  }
}

fn default_target_language() -> Result<TargetLanguage, AnyError> {
  PatternLanguage::Tsx.try_into().map_err(|err| {
    anyhow!("Failed to initialize default Grit target language: {err}")
  })
}

fn ensure_no_runtime_errors(
  results: &[MatchResult],
  pattern: &str,
) -> Result<(), AnyError> {
  let errors = results
    .iter()
    .filter_map(|result| match result {
      MatchResult::AnalysisLog(item) if item.level < 400 => {
        Some(if item.file.is_empty() {
          item.message.clone()
        } else {
          format!("{} ({})", item.message, item.file)
        })
      }
      _ => None,
    })
    .collect::<Vec<_>>();

  if errors.is_empty() {
    Ok(())
  } else {
    Err(anyhow!(
      "GritQL pattern '{}' failed: {}",
      pattern,
      errors.join("; "),
    ))
  }
}

fn create_diagnostic(
  code: &str,
  filename: &str,
  source_text: &str,
  byte_range: Option<(usize, usize)>,
  reason: Option<&MatchReason>,
  fallback_message: &str,
) -> Result<LintDiagnostic, AnyError> {
  let specifier = ModuleSpecifier::from_file_path(Path::new(filename))
    .map_err(|_| {
      anyhow!("Failed to convert '{}' to a module specifier", filename)
    })?;
  let text_info = SourceTextInfo::from_string(source_text.to_string());
  let range = byte_range.map(|(start_byte, end_byte)| {
    let file_start = text_info.range().start;
    let max_len = text_info.text_str().len();
    let start_byte = std::cmp::min(start_byte, max_len);
    let end_byte = std::cmp::min(std::cmp::max(end_byte, start_byte), max_len);
    LintDiagnosticRange {
      text_info: text_info.clone(),
      range: SourceRange::new(file_start + start_byte, file_start + end_byte),
      description: None,
    }
  });
  let (message, hint) = reason_parts(reason, fallback_message);

  Ok(LintDiagnostic {
    specifier,
    range,
    details: LintDiagnosticDetails {
      message,
      code: code.to_string(),
      hint,
      fixes: vec![],
      custom_docs_url: LintDocsUrl::None,
      info: vec![],
    },
  })
}

fn reason_parts(
  reason: Option<&MatchReason>,
  fallback_message: &str,
) -> (String, Option<String>) {
  if let Some(reason) = reason {
    let message = reason
      .title
      .clone()
      .or_else(|| reason.name.clone())
      .unwrap_or_else(|| fallback_message.to_string());
    return (message, reason.explanation.clone());
  }

  (fallback_message.to_string(), None)
}

fn rewrite_fallback_message(
  file_path: &Path,
  rewrite: &marzano_core::api::Rewrite,
) -> &'static str {
  if rewrite.original.source_file == rewrite.rewritten.source_file
    && rewrite.original.source_file == file_path.to_string_lossy()
  {
    "Matched GritQL rewrite"
  } else {
    "GritQL wants to rewrite another file"
  }
}

fn detect_file_language(path: &Path) -> Option<FileLanguage> {
  let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();

  match extension.as_str() {
    "js" | "jsx" | "ts" | "tsx" | "mjs" | "cjs" | "mts" | "cts" => {
      Some(FileLanguage::Js)
    }
    "json" | "jsonc" => Some(FileLanguage::Json),
    _ => None,
  }
}

fn matches_pattern_language(
  pattern_language: PatternLanguage,
  file_language: FileLanguage,
) -> bool {
  match file_language {
    FileLanguage::Js => matches!(
      pattern_language,
      PatternLanguage::JavaScript
        | PatternLanguage::TypeScript
        | PatternLanguage::Tsx
    ),
    FileLanguage::Json => pattern_language == PatternLanguage::Json,
  }
}

fn configure_patterns(patterns: Vec<String>) -> Vec<ConfiguredPattern> {
  let mut configured_patterns = Vec::new();
  let mut seen_codes = HashSet::new();

  for pattern in patterns {
    let base_code = format!("grit-{}", slugify(&pattern));
    let mut code = base_code.clone();
    let mut index = 2;
    while !seen_codes.insert(code.clone()) {
      code = format!("{}-{}", base_code, index);
      index += 1;
    }

    configured_patterns.push(ConfiguredPattern { code, pattern });
  }

  configured_patterns
}

#[derive(Clone, Debug)]
struct ConfiguredPattern {
  code: String,
  pattern: String,
}

fn slugify(text: &str) -> String {
  let mut slug = String::new();
  let mut previous_dash = false;

  for ch in text.trim().chars() {
    if ch.is_ascii_alphanumeric() {
      slug.push(ch.to_ascii_lowercase());
      previous_dash = false;
    } else if !previous_dash {
      slug.push('-');
      previous_dash = true;
    }
  }

  let slug = slug.trim_matches('-');
  if slug.is_empty() {
    "pattern".to_string()
  } else {
    slug.to_string()
  }
}

fn write_planned_changes(
  changes: &[PlannedFileChange],
) -> Result<(), AnyError> {
  if changes.is_empty() {
    return Ok(());
  }

  let mut temp_files = Vec::with_capacity(changes.len());
  for change in changes {
    let temp_path = temp_file_path(&change.path);
    fs::write(&temp_path, &change.updated_text).with_context(|| {
      format!(
        "Failed to write temporary file for '{}'",
        change.path.display(),
      )
    })?;

    if let Ok(metadata) = fs::metadata(&change.path) {
      let permissions = metadata.permissions();
      fs::set_permissions(&temp_path, permissions).with_context(|| {
        format!("Failed to copy permissions for '{}'", change.path.display())
      })?;
    }

    temp_files.push(temp_path);
  }

  let mut applied_indices: Vec<usize> = Vec::new();
  for (index, change) in changes.iter().enumerate() {
    if let Err(err) = fs::rename(&temp_files[index], &change.path) {
      for applied_index in applied_indices {
        let applied_change = &changes[applied_index];
        let _ = fs::write(&applied_change.path, &applied_change.original_text);
      }

      for temp_file in temp_files.iter().skip(index) {
        let _ = fs::remove_file(temp_file);
      }

      return Err(err).with_context(|| {
        format!(
          "Failed to replace '{}' with the planned GritQL rewrite",
          change.path.display(),
        )
      });
    }

    applied_indices.push(index);
  }

  Ok(())
}

fn temp_file_path(path: &Path) -> PathBuf {
  let parent = path.parent().unwrap_or_else(|| Path::new("."));
  let file_name = path
    .file_name()
    .and_then(|name| name.to_str())
    .unwrap_or("grit-fix");
  let nanos = SystemTime::now()
    .duration_since(UNIX_EPOCH)
    .unwrap()
    .as_nanos();
  parent.join(format!(
    ".{file_name}.grit-tmp-{}-{nanos}",
    std::process::id()
  ))
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn slugifies_patterns() {
    assert_eq!(slugify("rename_console"), "rename-console");
    assert_eq!(
      slugify("`console.log($x)` => `logger.info($x)`"),
      "console-log-x-logger-info-x"
    );
    assert_eq!(slugify("***"), "pattern");
  }

  #[test]
  fn detects_supported_file_languages() {
    assert_eq!(
      detect_file_language(Path::new("test.ts")),
      Some(FileLanguage::Js)
    );
    assert_eq!(
      detect_file_language(Path::new("test.json")),
      Some(FileLanguage::Json)
    );
    assert_eq!(
      detect_file_language(Path::new("test.jsonc")),
      Some(FileLanguage::Json)
    );
    assert_eq!(detect_file_language(Path::new("test.css")), None);
  }
}
