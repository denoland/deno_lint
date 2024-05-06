// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::collections::HashSet;
use std::ops::{Deref, DerefMut};

use super::reader::Reader;
use super::unicode::*;

fn is_syntax_character(cp: char) -> bool {
  cp == '^'
    || cp == '$'
    || cp == '\\'
    || cp == '.'
    || cp == '*'
    || cp == '+'
    || cp == '?'
    || cp == '('
    || cp == ')'
    || cp == '['
    || cp == ']'
    || cp == '{'
    || cp == '}'
    || cp == '|'
}

fn is_unicode_property_name_character(cp: char) -> bool {
  cp.is_ascii_alphabetic() || cp == '_'
}

fn is_unicode_property_value_character(cp: char) -> bool {
  is_unicode_property_name_character(cp) || cp.is_ascii_digit()
}

fn is_regexp_identifier_start(cp: char) -> bool {
  is_id_start(cp) || cp == '$' || cp == '_'
}

fn is_regexp_identifier_part(cp: char) -> bool {
  is_id_continue(cp) ||
    cp == '$' ||
    cp == '_' ||
    cp == '\u{200c}' ||  // unicode zero-width non-joiner
    cp == '\u{200d}' // unicode zero-width joiner
}

fn is_id_start(cp: char) -> bool {
  if (cp as u32) < 0x41 {
    false
  } else if (cp as u32) < 0x5b {
    true
  } else if (cp as u32) < 0x61 {
    false
  } else if (cp as u32) < 0x7b {
    true
  } else {
    is_large_id_start(cp)
  }
}

fn is_id_continue(cp: char) -> bool {
  if (cp as u32) < 0x30 {
    false
  } else if (cp as u32) < 0x3a {
    true
  } else if (cp as u32) < 0x41 {
    false
  } else if (cp as u32) < 0x5b || (cp as u32) == 0x5f {
    true
  } else if (cp as u32) < 0x61 {
    false
  } else if (cp as u32) < 0x7b {
    true
  } else {
    is_large_id_start(cp) || is_large_id_continue(cp)
  }
}

fn is_valid_unicode(cp: i64) -> bool {
  cp <= 0x10ffff
}

fn is_lead_surrogate(cp: i64) -> bool {
  (0xd800..=0xdbff).contains(&cp)
}

fn is_trail_surrogate(cp: i64) -> bool {
  (0xdc00..=0xdfff).contains(&cp)
}

fn combine_surrogate_pair(lead: i64, trail: i64) -> i64 {
  (lead - 0xd800) * 0x400 + (trail - 0xdc00) + 0x10000
}

#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Ord, PartialOrd, Eq, PartialEq, Hash)]
pub enum EcmaVersion {
  Es5,
  Es2015,
  Es2016,
  Es2017,
  Es2018,
  Es2019,
  Es2020,
  Es2021,
  Es2022,
}

#[derive(Debug)]
pub struct EcmaRegexValidator {
  reader: Reader,
  strict: bool,
  ecma_version: EcmaVersion,
  u_flag: bool,
  n_flag: bool,
  last_int_value: i64,
  last_min_value: i64,
  last_max_value: i64,
  last_str_value: String,
  last_key_value: String,
  last_val_value: String,
  last_assertion_is_quantifiable: bool,
  num_capturing_parens: u32,
  group_names: HashSet<String>,
  backreference_names: HashSet<String>,
}

impl Deref for EcmaRegexValidator {
  type Target = Reader;

  fn deref(&self) -> &Self::Target {
    &self.reader
  }
}

impl DerefMut for EcmaRegexValidator {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.reader
  }
}

impl EcmaRegexValidator {
  pub fn new(ecma_version: EcmaVersion) -> Self {
    EcmaRegexValidator {
      reader: Reader::new(),
      strict: false,
      ecma_version,
      u_flag: false,
      n_flag: false,
      last_int_value: 0,
      last_min_value: 0,
      last_max_value: 0,
      last_str_value: "".to_string(),
      last_key_value: "".to_string(),
      last_val_value: "".to_string(),
      last_assertion_is_quantifiable: false,
      num_capturing_parens: 0,
      group_names: HashSet::new(),
      backreference_names: HashSet::new(),
    }
  }

  /// Validates flags of a EcmaScript regular expression.
  pub fn validate_flags(&self, flags: &str) -> Result<(), String> {
    let mut existing_flags = HashSet::<char>::new();

    for flag in flags.chars() {
      if existing_flags.contains(&flag) {
        return Err(format!("Duplicated flag {}", flag));
      }
      existing_flags.insert(flag);

      if flag == 'g'
        || flag == 'i'
        || flag == 'm'
        || (flag == 'u' && self.ecma_version >= EcmaVersion::Es2015)
        || (flag == 'y' && self.ecma_version >= EcmaVersion::Es2015)
        || (flag == 's' && self.ecma_version >= EcmaVersion::Es2018)
        || (flag == 'd' && self.ecma_version >= EcmaVersion::Es2022)
        || (flag == 'v' && self.ecma_version >= EcmaVersion::Es2022)
      {
        // do nothing
      } else {
        return Err(format!("Invalid flag {}", flag));
      }
    }
    Ok(())
  }

  /// Validates the pattern of a EcmaScript regular expression.
  pub fn validate_pattern(
    &mut self,
    source: &str,
    u_flag: bool,
  ) -> Result<(), String> {
    self.strict = u_flag; // TODO: allow toggling strict independently of u flag
    self.u_flag = u_flag && self.ecma_version >= EcmaVersion::Es2015;
    self.n_flag = u_flag && self.ecma_version >= EcmaVersion::Es2018;
    //self.reset(source, 0, source.len(), u_flag);
    self.reset(source, 0, source.chars().count(), u_flag);
    self.consume_pattern()?;

    if !self.n_flag
      && self.ecma_version >= EcmaVersion::Es2018
      && !self.group_names.is_empty()
    {
      self.n_flag = true;
      self.rewind(0);
      self.consume_pattern()?;
    }

    Ok(())
  }

  /// Validate the next characters as a RegExp `Pattern` production.
  /// ```grammar
  /// Pattern[U, N]::
  ///     Disjunction[?U, ?N]
  /// ```
  fn consume_pattern(&mut self) -> Result<(), String> {
    self.num_capturing_parens = self.count_capturing_parens();
    self.group_names.clear();
    self.backreference_names.clear();

    self.consume_disjunction()?;

    if let Some(cp) = self.code_point_with_offset(0) {
      if cp == ')' {
        return Err("Unmatched ')'".to_string());
      } else if cp == '\\' {
        return Err("\\ at end of pattern".to_string());
      } else if cp == ']' || cp == '}' {
        return Err("Lone quantifier brackets".to_string());
      }
      return Err(format!("Unexpected character {}", cp));
    }

    if let Some(name) = self
      .backreference_names
      .difference(&self.group_names)
      .next()
    {
      return Err(format!("Invalid named capture referenced: {}", name));
    }
    Ok(())
  }

  /// Validate the next characters as a RegExp `Disjunction` production.
  /// ```grammar
  /// Disjunction[U, N]::
  ///      Alternative[?U, ?N]
  ///      Alternative[?U, ?N] `|` Disjunction[?U, ?N]
  /// ```
  fn consume_disjunction(&mut self) -> Result<(), String> {
    self.consume_alternative()?;
    while self.eat('|') {
      self.consume_alternative()?;
    }

    if self.consume_quantifier(true)? {
      Err("Nothing to repeat".to_string())
    } else if self.eat('{') {
      Err("Lone quantifier brackets".to_string())
    } else {
      Ok(())
    }
  }

  /// Validate the next characters as a RegExp `Alternative` production.
  /// ```grammar
  /// Alternative[U, N]::
  ///      ε
  ///      Alternative[?U, ?N] Term[?U, ?N]
  /// ```
  fn consume_alternative(&mut self) -> Result<(), String> {
    while self.code_point_with_offset(0).is_some() && self.consume_term()? {
      // do nothing
    }
    Ok(())
  }

  /// Validate the next characters as a RegExp `Term` production if possible.
  /// ```grammar
  /// Term[U, N]::
  ///      [strict] Assertion[+U, ?N]
  ///      [strict] Atom[+U, ?N]
  ///      [strict] Atom[+U, ?N] Quantifier
  ///      [annexB][+U] Assertion[+U, ?N]
  ///      [annexB][+U] Atom[+U, ?N]
  ///      [annexB][+U] Atom[+U, ?N] Quantifier
  ///      [annexB][~U] QuantifiableAssertion[?N] Quantifier
  ///      [annexB][~U] Assertion[~U, ?N]
  ///      [annexB][~U] ExtendedAtom[?N] Quantifier
  ///      [annexB][~U] ExtendedAtom[?N]
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_term(&mut self) -> Result<bool, String> {
    if self.u_flag || self.strict {
      Ok(
        self.consume_assertion()?
          || (self.consume_atom()? && self.consume_optional_quantifier()?),
      )
    } else {
      Ok(
        (self.consume_assertion()?
          && (!self.last_assertion_is_quantifiable
            || self.consume_optional_quantifier()?))
          || (self.consume_extended_atom()?
            && self.consume_optional_quantifier()?),
      )
    }
  }

  fn consume_optional_quantifier(&mut self) -> Result<bool, String> {
    self.consume_quantifier(false)?;
    Ok(true)
  }

  /// Validate the next characters as a RegExp `Term` production if possible.
  /// Set `self.last_assertion_is_quantifiable` if the consumed assertion was a
  /// `QuantifiableAssertion` production.
  /// ```grammar
  /// Assertion[U, N]::
  ///      `^`
  ///      `$`
  ///      `\b`
  ///      `\B`
  ///      [strict] `(?=` Disjunction[+U, ?N] `)`
  ///      [strict] `(?!` Disjunction[+U, ?N] `)`
  ///      [annexB][+U] `(?=` Disjunction[+U, ?N] `)`
  ///      [annexB][+U] `(?!` Disjunction[+U, ?N] `)`
  ///      [annexB][~U] QuantifiableAssertion[?N]
  ///      `(?<=` Disjunction[?U, ?N] `)`
  ///      `(?<!` Disjunction[?U, ?N] `)`
  /// QuantifiableAssertion[N]::
  ///      `(?=` Disjunction[~U, ?N] `)`
  ///      `(?!` Disjunction[~U, ?N] `)`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_assertion(&mut self) -> Result<bool, String> {
    let start = self.index();
    self.last_assertion_is_quantifiable = false;

    if self.eat('^')
      || self.eat('$')
      || self.eat2('\\', 'B')
      || self.eat2('\\', 'b')
    {
      return Ok(true);
    }

    // Lookahead / Lookbehind
    if self.eat2('(', '?') {
      let lookbehind =
        self.ecma_version >= EcmaVersion::Es2018 && self.eat('<');
      let mut flag = self.eat('=');
      if !flag {
        flag = self.eat('!');
      }
      if flag {
        self.consume_disjunction()?;
        if !self.eat(')') {
          return Err("Unterminated group".to_string());
        }
        self.last_assertion_is_quantifiable = !lookbehind && !self.strict;
        return Ok(true);
      }
      self.rewind(start);
    }
    Ok(false)
  }

  /// Validate the next characters as a RegExp `Quantifier` production if possible.
  /// ```grammar
  /// Quantifier::
  ///      QuantifierPrefix
  ///      QuantifierPrefix `?`
  /// QuantifierPrefix::
  ///      `*`
  ///      `+`
  ///      `?`
  ///      `{` DecimalDigits `}`
  ///      `{` DecimalDigits `,}`
  ///      `{` DecimalDigits `,` DecimalDigits `}`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_quantifier(&mut self, no_consume: bool) -> Result<bool, String> {
    // QuantifierPrefix
    if !self.eat('*')
      && !self.eat('+')
      && !self.eat('?')
      && !self.eat_braced_quantifier(no_consume)?
    {
      Ok(false)
    } else {
      self.eat('?');
      Ok(true)
    }
  }

  /// Eats the next characters as the following alternatives if possible.
  /// Sets `self.last_min_value` and `self.last_max_value` if it consumed the next characters
  /// successfully.
  /// ```grammar
  ///      `{` DecimalDigits `}`
  ///      `{` DecimalDigits `,}`
  ///      `{` DecimalDigits `,` DecimalDigits `}`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn eat_braced_quantifier(&mut self, no_error: bool) -> Result<bool, &str> {
    let start = self.index();
    if self.eat('{') {
      self.last_min_value = 0;
      self.last_max_value = i64::MAX;
      if self.eat_decimal_digits() {
        self.last_min_value = self.last_int_value;
        self.last_max_value = self.last_int_value;
        if self.eat(',') {
          self.last_max_value = if self.eat_decimal_digits() {
            self.last_int_value
          } else {
            i64::MAX
          }
        }
        if self.eat('}') {
          if !no_error && self.last_max_value < self.last_min_value {
            return Err("numbers out of order in {} quantifier");
          }
          return Ok(true);
        }
      }
      if !no_error && (self.u_flag || self.strict) {
        return Err("Incomplete quantifier");
      }
      self.rewind(start);
    }
    Ok(false)
  }

  /// Validate the next characters as a RegExp `Atom` production if possible.
  /// ```grammar
  /// Atom[U, N]::
  ///      PatternCharacter
  ///      `.`
  ///      `\\` AtomEscape[?U, ?N]
  ///      CharacterClass[?U]
  ///      `(?:` Disjunction[?U, ?N] )
  ///      `(` GroupSpecifier[?U] Disjunction[?U, ?N] `)`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_atom(&mut self) -> Result<bool, String> {
    Ok(
      self.consume_pattern_character()
        || self.eat('.')
        || self.consume_reverse_solidus_atom_escape()?
        || self.consume_character_class()?
        || self.consume_uncapturing_group()?
        || self.consume_capturing_group()?,
    )
  }

  /// Validate the next characters as the following alternatives if possible.
  /// ```grammar
  ///      `\\` AtomEscape[?U, ?N]
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_reverse_solidus_atom_escape(&mut self) -> Result<bool, String> {
    let start = self.index();
    if self.eat('\\') {
      if self.consume_atom_escape()? {
        return Ok(true);
      }
      self.rewind(start);
    }
    Ok(false)
  }

  /// Validate the next characters as the following alternatives if possible.
  /// ```grammar
  ///      `(?:` Disjunction[?U, ?N] )
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_uncapturing_group(&mut self) -> Result<bool, String> {
    if self.eat3('(', '?', ':') {
      self.consume_disjunction()?;
      if !self.eat(')') {
        Err("Unterminated group".to_string())
      } else {
        Ok(true)
      }
    } else {
      Ok(false)
    }
  }

  /// Validate the next characters as the following alternatives if possible.
  /// ```grammar
  ///      `(` GroupSpecifier[?U] Disjunction[?U, ?N] `)`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_capturing_group(&mut self) -> Result<bool, String> {
    if !self.eat('(') {
      return Ok(false);
    }

    if self.ecma_version >= EcmaVersion::Es2018 {
      self.consume_group_specifier()?;
    } else if self.code_point_with_offset(0) == Some('?') {
      return Err("Invalid group".to_string());
    }

    self.consume_disjunction()?;
    if !self.eat(')') {
      return Err("Unterminated group".to_string());
    }
    Ok(true)
  }

  /// Validate the next characters as a RegExp `ExtendedAtom` production if possible.
  /// ```grammar
  /// ExtendedAtom[N]::
  ///      `.`
  ///      `\` AtomEscape[~U, ?N]
  ///      `\` [lookahead = c]
  ///      CharacterClass[~U]
  ///      `(?:` Disjunction[~U, ?N] `)`
  ///      `(` Disjunction[~U, ?N] `)`
  ///      InvalidBracedQuantifier
  ///      ExtendedPatternCharacter
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_extended_atom(&mut self) -> Result<bool, String> {
    Ok(
      self.eat('.')
        || self.consume_reverse_solidus_atom_escape()?
        || self.consume_reverse_solidus_followed_by_c()
        || self.consume_character_class()?
        || self.consume_uncapturing_group()?
        || self.consume_capturing_group()?
        || self.consume_invalid_braced_quantifier()?
        || self.consume_extended_pattern_character(),
    )
  }

  /// Validate the next characters as the following alternatives if possible.
  /// ```grammar
  ///      `\` [lookahead = c]
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_reverse_solidus_followed_by_c(&mut self) -> bool {
    if self.code_point_with_offset(0) == Some('\\')
      && self.code_point_with_offset(1) == Some('c')
    {
      self.last_int_value = '\\' as i64;
      self.advance();
      true
    } else {
      false
    }
  }

  /// Validate the next characters as a RegExp `InvalidBracedQuantifier`
  /// production if possible.
  /// ```grammar
  /// InvalidBracedQuantifier::
  ///      `{` DecimalDigits `}`
  ///      `{` DecimalDigits `,}`
  ///      `{` DecimalDigits `,` DecimalDigits `}`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_invalid_braced_quantifier(&mut self) -> Result<bool, &str> {
    if self.eat_braced_quantifier(true)? {
      Err("Nothing to repeat")
    } else {
      Ok(false)
    }
  }

  /// Validate the next characters as a RegExp `PatternCharacter` production if
  /// possible.
  /// ```grammar
  /// PatternCharacter::
  ///      SourceCharacter but not SyntaxCharacter
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_pattern_character(&mut self) -> bool {
    if let Some(cp) = self.code_point_with_offset(0) {
      if !is_syntax_character(cp) {
        self.advance();
        return true;
      }
    }
    false
  }

  /// Validate the next characters as a RegExp `ExtendedPatternCharacter`
  /// production if possible.
  /// ```grammar
  /// ExtendedPatternCharacter::
  ///      SourceCharacter but not one of ^ $ \ . * + ? ( ) [ |
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_extended_pattern_character(&mut self) -> bool {
    if let Some(cp) = self.code_point_with_offset(0) {
      if cp != '^'
        && cp != '$'
        && cp != '\\'
        && cp != '.'
        && cp != '*'
        && cp != '+'
        && cp != '?'
        && cp != '('
        && cp != ')'
        && cp != '['
        && cp != '|'
      {
        self.advance();
        return true;
      }
    }
    false
  }

  /// Validate the next characters as a RegExp `GroupSpecifier` production.
  /// Set `self.last_str_value` if the group name existed.
  /// ```grammar
  /// GroupSpecifier[U]::
  ///      ε
  ///      `?` GroupName[?U]
  /// ```
  /// Returns `true` if the group name existed.
  fn consume_group_specifier(&mut self) -> Result<bool, String> {
    if self.eat('?') {
      if self.eat_group_name()? {
        if !self.group_names.contains(&self.last_str_value) {
          self.group_names.insert(self.last_str_value.clone());
          Ok(true)
        } else {
          Err("Duplicate capture group name".to_string())
        }
      } else {
        Err("Invalid group".to_string())
      }
    } else {
      Ok(false)
    }
  }

  /// Validate the next characters as a RegExp `AtomEscape` production if possible.
  /// ```grammar
  /// AtomEscape[U, N]::
  ///      [strict] DecimalEscape
  ///      [annexB][+U] DecimalEscape
  ///      [annexB][~U] DecimalEscape but only if the CapturingGroupNumber of DecimalEscape is <= NcapturingParens
  ///      CharacterClassEscape[?U]
  ///      [strict] CharacterEscape[?U]
  ///      [annexB] CharacterEscape[?U, ?N]
  ///      [+N] `k` GroupName[?U]
  /// ```
  /// Returns `Ok(true)` if it consumed the next characters successfully.
  fn consume_atom_escape(&mut self) -> Result<bool, String> {
    if self.consume_backreference()?
      || self.consume_character_class_escape()?
      || self.consume_character_escape()?
      || (self.n_flag && self.consume_k_group_name()?)
    {
      Ok(true)
    } else if self.strict || self.u_flag {
      Err("Invalid escape".to_string())
    } else {
      Ok(false)
    }
  }

  /// Validate the next characters as the follwoing alternatives if possible.
  /// ```grammar
  ///      [strict] DecimalEscape
  ///      [annexB][+U] DecimalEscape
  ///      [annexB][~U] DecimalEscape but only if the CapturingGroupNumber of DecimalEscape is <= NcapturingParens
  /// ```
  /// Returns `Ok(true)` if it consumed the next characters successfully.
  fn consume_backreference(&mut self) -> Result<bool, &str> {
    let start = self.index();
    if self.eat_decimal_escape() {
      if self.last_int_value <= self.num_capturing_parens as i64 {
        return Ok(true);
      } else if self.strict || self.u_flag {
        return Err("Invalid escape");
      }
      self.rewind(start);
    }
    Ok(false)
  }

  /// Validate the next characters as a RegExp `DecimalEscape` production if possible.
  /// Set `-1` to `self.last_int_value` as meaning of a character set if it ate the next
  /// characters successfully.
  /// ```grammar
  /// CharacterClassEscape[U]::
  ///      `d`
  ///      `D`
  ///      `s`
  ///      `S`
  ///      `w`
  ///      `W`
  ///      [+U] `p{` UnicodePropertyValueExpression `}`
  ///      [+U] `P{` UnicodePropertyValueExpression `}`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_character_class_escape(&mut self) -> Result<bool, String> {
    if self.eat('d')
      || self.eat('D')
      || self.eat('s')
      || self.eat('S')
      || self.eat('w')
      || self.eat('W')
    {
      self.last_int_value = -1;
      return Ok(true);
    }

    if self.u_flag
      && self.ecma_version >= EcmaVersion::Es2018
      && (self.eat('p') || self.eat('P'))
    {
      self.last_int_value = -1;
      if self.eat('{')
        && self.eat_unicode_property_value_expression()?
        && self.eat('}')
      {
        return Ok(true);
      }
      return Err("Invalid property name".to_string());
    }
    Ok(false)
  }

  /// Validate the next characters as a RegExp `CharacterEscape` production if possible.
  /// ```grammar
  /// CharacterEscape[U, N]::
  ///      ControlEscape
  ///      `c` ControlLetter
  ///      `0` [lookahead ∉ DecimalDigit]
  ///      HexEscapeSequence
  ///      RegExpUnicodeEscapeSequence[?U]
  ///      [annexB][~U] LegacyOctalEscapeSequence
  ///      IdentityEscape[?U, ?N]
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_character_escape(&mut self) -> Result<bool, String> {
    Ok(
      self.eat_control_escape()
        || self.eat_c_control_letter()
        || self.eat_zero()
        || self.eat_hex_escape_sequence()?
        || self.eat_regexp_unicode_escape_sequence(false)?
        || (!self.strict
          && !self.u_flag
          && self.eat_legacy_octal_escape_sequence())
        || self.eat_identity_escape(),
    )
  }

  /// Validate the next characters as the follwoing alternatives if possible.
  /// ```grammar
  ///      `k` GroupName[?U]
  /// ```
  /// Returns `Ok(true)` if it consumed the next characters successfully.
  fn consume_k_group_name(&mut self) -> Result<bool, String> {
    if self.eat('k') {
      if self.eat_group_name()? {
        let group_name = self.last_str_value.clone();
        self.backreference_names.insert(group_name);
        return Ok(true);
      }
      return Err("Invalid named reference".to_string());
    }
    Ok(false)
  }

  /// Validate the next characters as a RegExp `CharacterClass` production if possible.
  /// ```grammar
  /// CharacterClass[U]::
  ///      `[` [lookahead ≠ ^] ClassRanges[?U] `]`
  ///      `[^` ClassRanges[?U] `]`
  /// ```
  /// Returns `true` if it consumed the next characters successfully.
  fn consume_character_class(&mut self) -> Result<bool, String> {
    if !self.eat('[') {
      return Ok(false);
    }
    self.consume_class_ranges()?;
    if !self.eat(']') {
      return Err("Unterminated character class".to_string());
    }
    Ok(true)
  }

  /// Validate the next characters as a RegExp `ClassRanges` production.
  /// ```grammar
  /// ClassRanges[U]::
  ///      ε
  ///      NonemptyClassRanges[?U]
  /// NonemptyClassRanges[U]::
  ///      ClassAtom[?U]
  ///      ClassAtom[?U] NonemptyClassRangesNoDash[?U]
  ///      ClassAtom[?U] `-` ClassAtom[?U] ClassRanges[?U]
  /// NonemptyClassRangesNoDash[U]::
  ///      ClassAtom[?U]
  ///      ClassAtomNoDash[?U] NonemptyClassRangesNoDash[?U]
  ///      ClassAtomNoDash[?U] `-` ClassAtom[?U] ClassRanges[?U]
  /// ```
  fn consume_class_ranges(&mut self) -> Result<(), String> {
    loop {
      // Consume the first ClassAtom
      if !self.consume_class_atom()? {
        break;
      }
      let min = self.last_int_value;

      // Consume `-`
      if !self.eat('-') {
        continue;
      }

      // Consume the second ClassAtom
      if !self.consume_class_atom()? {
        break;
      }
      let max = self.last_int_value;

      // Validate
      if min == -1 || max == -1 {
        if self.strict {
          return Err("Invalid character class".to_string());
        }
        continue;
      }

      if min > max {
        return Err("Range out of order in character class".to_string());
      }
    }
    Ok(())
  }

  /// Validate the next characters as a RegExp `ClassAtom` production if possible.
  /// Set `self.last_int_value` if it consumed the next characters successfully.
  /// ```grammar
  /// ClassAtom[U, N]::
  ///      `-`
  ///      ClassAtomNoDash[?U, ?N]
  /// ClassAtomNoDash[U, N]::
  ///      SourceCharacter but not one of \ ] -
  ///      `\` ClassEscape[?U, ?N]
  ///      [annexB] `\` [lookahead = c]
  /// ```
  /// Returns `Ok(true)` if it consumed the next characters successfully.
  fn consume_class_atom(&mut self) -> Result<bool, String> {
    let start = self.index();

    if let Some(cp) = self.code_point_with_offset(0) {
      if cp != '\\' && cp != ']' {
        self.advance();
        self.last_int_value = cp as i64;
        return Ok(true);
      }
    }

    if self.eat('\\') {
      if self.consume_class_escape()? {
        return Ok(true);
      }
      if !self.strict && self.code_point_with_offset(0) == Some('c') {
        self.last_int_value = '\\' as i64;
        return Ok(true);
      }
      if self.strict || self.u_flag {
        return Err("Invalid escape".to_string());
      }
      self.rewind(start);
    }
    Ok(false)
  }

  /// Validate the next characters as a RegExp `ClassEscape` production if possible.
  /// Set `self.last_int_value` if it consumed the next characters successfully.
  /// ```grammar
  /// ClassEscape[U, N]::
  ///      `b`
  ///      [+U] `-`
  ///      [annexB][~U] `c` ClassControlLetter
  ///      CharacterClassEscape[?U]
  ///      CharacterEscape[?U, ?N]
  /// ClassControlLetter::
  ///      DecimalDigit
  ///      `_`
  /// ```
  /// Returns `Ok(true)` if it consumed the next characters successfully.
  fn consume_class_escape(&mut self) -> Result<bool, String> {
    if self.eat('b') {
      self.last_int_value = 0x08; // backspace
      return Ok(true);
    }

    // [+U] `-`
    if self.u_flag && self.eat('-') {
      self.last_int_value = '-' as i64;
      return Ok(true);
    }

    // [annexB][~U] `c` ClassControlLetter
    if !self.strict
      && !self.u_flag
      && self.code_point_with_offset(0) == Some('c')
    {
      if let Some(cp) = self.code_point_with_offset(1) {
        if cp.is_ascii_digit() || cp == '_' {
          self.advance();
          self.advance();
          self.last_int_value = cp as i64 % 0x20;
          return Ok(true);
        }
      }
    }

    Ok(
      self.consume_character_class_escape()?
        || self.consume_character_escape()?,
    )
  }

  /// Eat the next characters as a RegExp `GroupName` production if possible.
  /// Set `self.last_str_value` if the group name existed.
  /// ```grammar
  /// GroupName[U]::
  ///      `<` RegExpIdentifierName[?U] `>`
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_group_name(&mut self) -> Result<bool, String> {
    if self.eat('<') {
      if self.eat_regexp_identifier_name()? && self.eat('>') {
        Ok(true)
      } else {
        Err("Invalid capture group name".to_string())
      }
    } else {
      Ok(false)
    }
  }

  /// Eat the next characters as a RegExp `RegExpIdentifierName` production if
  /// possible.
  /// Set `self.last_str_value` if the identifier name existed.
  /// ```grammar
  /// RegExpIdentifierName[U]::
  ///      RegExpIdentifierStart[?U]
  ///      RegExpIdentifierName[?U] RegExpIdentifierPart[?U]
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_regexp_identifier_name(&mut self) -> Result<bool, String> {
    if self.eat_regexp_identifier_start()? {
      self.last_str_value = std::char::from_u32(self.last_int_value as u32)
        .unwrap()
        .to_string();
      while self.eat_regexp_identifier_part()? {
        self
          .last_str_value
          .push(std::char::from_u32(self.last_int_value as u32).unwrap());
      }
      Ok(true)
    } else {
      Ok(false)
    }
  }

  /// Eat the next characters as a RegExp `RegExpIdentifierStart` production if
  /// possible.
  /// Set `self.last_int_value` if the identifier start existed.
  /// ```grammar
  /// RegExpIdentifierStart[U] ::
  ///      UnicodeIDStart
  ///      `$`
  ///      `_`
  ///      `\` RegExpUnicodeEscapeSequence[+U]
  ///      [~U] UnicodeLeadSurrogate UnicodeTrailSurrogate
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_regexp_identifier_start(&mut self) -> Result<bool, String> {
    let start = self.index();
    let force_u_flag = !self.u_flag && self.ecma_version >= EcmaVersion::Es2020;

    if let Some(mut cp) = self.code_point_with_offset(0) {
      self.advance();
      let cp1 = self.code_point_with_offset(0);
      if cp == '\\' && self.eat_regexp_unicode_escape_sequence(force_u_flag)? {
        cp = std::char::from_u32(self.last_int_value as u32).unwrap();
      } else if force_u_flag
        && is_lead_surrogate(cp as i64)
        && cp1.is_some()
        && is_trail_surrogate(cp1.unwrap() as i64)
      {
        cp = std::char::from_u32(combine_surrogate_pair(
          cp as i64,
          cp1.unwrap() as i64,
        ) as u32)
        .unwrap();
        self.advance();
      }

      if is_regexp_identifier_start(cp) {
        self.last_int_value = cp as i64;
        return Ok(true);
      }
    }

    if self.index() != start {
      self.rewind(start);
    }
    Ok(false)
  }

  /// Eat the next characters as a RegExp `RegExpIdentifierPart` production if
  /// possible.
  /// Set `self.last_int_value` if the identifier part existed.
  /// ```grammar
  /// RegExpIdentifierPart[U] ::
  ///      UnicodeIDContinue
  ///      `$`
  ///      `_`
  ///      `\` RegExpUnicodeEscapeSequence[+U]
  ///      [~U] UnicodeLeadSurrogate UnicodeTrailSurrogate
  ///      <ZWNJ>
  ///      <ZWJ>
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_regexp_identifier_part(&mut self) -> Result<bool, String> {
    let start = self.index();
    let force_u_flag = !self.u_flag && self.ecma_version >= EcmaVersion::Es2020;
    let mut cp = self.code_point_with_offset(0);
    self.advance();
    let cp1 = self.code_point_with_offset(0);

    if cp == Some('\\')
      && self.eat_regexp_unicode_escape_sequence(force_u_flag)?
    {
      // TODO: convert unicode code point to char
      cp = std::char::from_u32(self.last_int_value as u32);
    } else if force_u_flag
      && is_lead_surrogate(cp.unwrap() as i64)
      && is_trail_surrogate(cp1.unwrap() as i64)
    {
      cp = std::char::from_u32(combine_surrogate_pair(
        cp.unwrap() as i64,
        cp1.unwrap() as i64,
      ) as u32);
      self.advance();
    }

    if let Some(c) = cp {
      if is_regexp_identifier_part(c) {
        self.last_int_value = c as i64;
        return Ok(true);
      }
    }

    if self.index() != start {
      self.rewind(start);
    }
    Ok(false)
  }

  /// Eat the next characters as the follwoing alternatives if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  ///      `c` ControlLetter
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_c_control_letter(&mut self) -> bool {
    let start = self.index();
    if self.eat('c') {
      if self.eat_control_letter() {
        return true;
      }
      self.rewind(start);
    }
    false
  }

  /// Eat the next characters as the follwoing alternatives if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  ///      `0` [lookahead ∉ DecimalDigit]
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_zero(&mut self) -> bool {
    if self.code_point_with_offset(0) != Some('0') {
      return false;
    } else if let Some(cp) = self.code_point_with_offset(1) {
      if cp.is_ascii_digit() {
        return false;
      }
    }
    self.last_int_value = 0;
    self.advance();
    true
  }

  /// Eat the next characters as a RegExp `ControlEscape` production if
  /// possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// ControlEscape:: one of
  ///      f n r t v
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_control_escape(&mut self) -> bool {
    if self.eat('f') {
      self.last_int_value = 0x0c; // formfeed
      return true;
    }
    if self.eat('n') {
      self.last_int_value = 0x0a; // linefeed
      return true;
    }
    if self.eat('r') {
      self.last_int_value = 0x0d; // carriage return
      return true;
    }
    if self.eat('t') {
      self.last_int_value = 0x09; // character tabulation
      return true;
    }
    if self.eat('v') {
      self.last_int_value = 0x0b; // line tabulation
      return true;
    }
    false
  }

  /// Eat the next characters as a RegExp `ControlLetter` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// ControlLetter:: one of
  ///      a b c d e f g h i j k l m n o p q r s t u v w x y z
  ///      A B C D E F G H I J K L M N O P Q R S T U V W X Y Z
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_control_letter(&mut self) -> bool {
    if let Some(cp) = self.code_point_with_offset(0) {
      if cp.is_ascii_alphabetic() {
        self.advance();
        self.last_int_value = cp as i64 % 0x20;
        return true;
      }
    }
    false
  }

  /// Eat the next characters as a RegExp `RegExpUnicodeEscapeSequence`
  /// production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// RegExpUnicodeEscapeSequence[U]::
  ///      [+U] `u` LeadSurrogate `\u` TrailSurrogate
  ///      [+U] `u` LeadSurrogate
  ///      [+U] `u` TrailSurrogate
  ///      [+U] `u` NonSurrogate
  ///      [~U] `u` Hex4Digits
  ///      [+U] `u{` CodePoint `}`
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_regexp_unicode_escape_sequence(
    &mut self,
    force_u_flag: bool,
  ) -> Result<bool, &str> {
    let start = self.index();
    let u_flag = force_u_flag || self.u_flag;

    if self.eat('u') {
      if (u_flag && self.eat_regexp_unicode_surrogate_pair_escape())
        || self.eat_fixed_hex_digits(4)
        || (u_flag && self.eat_regexp_unicode_codepoint_escape())
      {
        return Ok(true);
      }
      if self.strict || u_flag {
        return Err("Invalid unicode escape");
      }
      self.rewind(start);
    }

    Ok(false)
  }

  /// Eat the next characters as the following alternatives if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  ///      LeadSurrogate `\u` TrailSurrogate
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_regexp_unicode_surrogate_pair_escape(&mut self) -> bool {
    let start = self.index();

    if self.eat_fixed_hex_digits(4) {
      let lead = self.last_int_value;
      if is_lead_surrogate(lead)
        && self.eat('\\')
        && self.eat('u')
        && self.eat_fixed_hex_digits(4)
      {
        let trail = self.last_int_value;
        if is_trail_surrogate(trail) {
          self.last_int_value = combine_surrogate_pair(lead, trail);
          return true;
        }
      }

      self.rewind(start);
    }

    false
  }

  /// Eat the next characters as the following alternatives if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  ///      `{` CodePoint `}`
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_regexp_unicode_codepoint_escape(&mut self) -> bool {
    let start = self.index();

    if self.eat('{')
      && self.eat_hex_digits()
      && self.eat('}')
      && is_valid_unicode(self.last_int_value)
    {
      true
    } else {
      self.rewind(start);
      false
    }
  }

  /// Eat the next characters as a RegExp `IdentityEscape` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// IdentityEscape[U, N]::
  ///      [+U] SyntaxCharacter
  ///      [+U] `/`
  ///      [strict][~U] SourceCharacter but not UnicodeIDContinue
  ///      [annexB][~U] SourceCharacterIdentityEscape[?N]
  /// SourceCharacterIdentityEscape[N]::
  ///      [~N] SourceCharacter but not c
  ///      [+N] SourceCharacter but not one of c k
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_identity_escape(&mut self) -> bool {
    if let Some(cp) = self.code_point_with_offset(0) {
      if self.is_valid_identity_escape(cp) {
        self.last_int_value = cp as i64;
        self.advance();
        return true;
      }
    }
    false
  }
  fn is_valid_identity_escape(&self, cp: char) -> bool {
    if self.u_flag {
      is_syntax_character(cp) || cp == '/'
    } else if self.strict {
      !is_id_continue(cp)
    } else if self.n_flag {
      !(cp == 'c' || cp == 'k')
    } else {
      cp != 'c'
    }
  }

  /// Eat the next characters as a RegExp `DecimalEscape` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// DecimalEscape::
  ///      NonZeroDigit DecimalDigits(opt) [lookahead ∉ DecimalDigit]
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_decimal_escape(&mut self) -> bool {
    self.last_int_value = 0;
    if let Some(cp) = self.code_point_with_offset(0) {
      if cp.is_ascii_digit() {
        self.last_int_value =
          10 * self.last_int_value + cp.to_digit(10).unwrap() as i64;
        self.advance();
        while let Some(cp) = self.code_point_with_offset(0) {
          if !cp.is_ascii_digit() {
            break;
          }
          self.last_int_value =
            10 * self.last_int_value + cp.to_digit(10).unwrap() as i64;
          self.advance();
        }
        return true;
      }
    }
    false
  }

  /// Eat the next characters as a RegExp `UnicodePropertyValueExpression` production if possible.
  /// Set `self.last_key_value` and `self.last_val_value` if it ate the next characters
  /// successfully.
  /// ```grammar
  /// UnicodePropertyValueExpression::
  ///      UnicodePropertyName `=` UnicodePropertyValue
  ///      LoneUnicodePropertyNameOrValue
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_unicode_property_value_expression(&mut self) -> Result<bool, &str> {
    let start = self.index();

    // UnicodePropertyName `=` UnicodePropertyValue
    if self.eat_unicode_property_name() && self.eat('=') {
      self.last_key_value.clone_from(&self.last_str_value);
      if self.eat_unicode_property_value() {
        self.last_val_value.clone_from(&self.last_str_value);
        if is_valid_unicode_property(
          self.ecma_version,
          &self.last_key_value,
          &self.last_val_value,
        ) {
          return Ok(true);
        }
        return Err("Invalid property name");
      }
    }
    self.rewind(start);

    // LoneUnicodePropertyNameOrValue
    if self.eat_lone_unicode_property_name_or_value() {
      let name_or_value = self.last_str_value.clone();
      if is_valid_unicode_property(
        self.ecma_version,
        "General_Category",
        &name_or_value,
      ) {
        self.last_key_value = "General_Category".to_string();
        self.last_val_value = name_or_value;
        return Ok(true);
      }
      if is_valid_lone_unicode_property(self.ecma_version, &name_or_value) {
        self.last_key_value = name_or_value;
        self.last_val_value = "".to_string();
        return Ok(true);
      }
      return Err("Invalid property name");
    }
    Ok(false)
  }

  /// Eat the next characters as a RegExp `UnicodePropertyName` production if possible.
  /// Set `self.last_str_value` if it ate the next characters successfully.
  /// ```grammar
  /// UnicodePropertyName::
  ///      UnicodePropertyNameCharacters
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_unicode_property_name(&mut self) -> bool {
    self.last_str_value = "".to_string();
    while let Some(cp) = self.code_point_with_offset(0) {
      if !is_unicode_property_name_character(cp) {
        break;
      }
      self.last_str_value.push(cp);
      self.advance();
    }
    !self.last_str_value.is_empty()
  }

  /// Eat the next characters as a RegExp `UnicodePropertyValue` production if possible.
  /// Set `self.last_str_value` if it ate the next characters successfully.
  /// ```grammar
  /// UnicodePropertyValue::
  ///      UnicodePropertyValueCharacters
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_unicode_property_value(&mut self) -> bool {
    self.last_str_value = "".to_string();
    while let Some(cp) = self.code_point_with_offset(0) {
      if !is_unicode_property_value_character(cp) {
        break;
      }
      self.last_str_value.push(cp);
      self.advance();
    }
    !self.last_str_value.is_empty()
  }

  /// Eat the next characters as a RegExp `UnicodePropertyValue` production if possible.
  /// Set `self.last_str_value` if it ate the next characters successfully.
  /// ```grammar
  /// LoneUnicodePropertyNameOrValue::
  ///      UnicodePropertyValueCharacters
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_lone_unicode_property_name_or_value(&mut self) -> bool {
    self.eat_unicode_property_value()
  }

  /// Eat the next characters as a `HexEscapeSequence` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// HexEscapeSequence::
  ///      `x` HexDigit HexDigit
  /// HexDigit:: one of
  ///      0 1 2 3 4 5 6 7 8 9 a b c d e f A B C D E F
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_hex_escape_sequence(&mut self) -> Result<bool, &str> {
    let start = self.index();
    if self.eat('x') {
      if self.eat_fixed_hex_digits(2) {
        return Ok(true);
      }
      if self.u_flag || self.strict {
        return Err("Invalid escape");
      }
      self.rewind(start);
    }
    Ok(false)
  }

  /// Eat the next characters as a `DecimalDigits` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// DecimalDigits::
  ///      DecimalDigit
  ///      DecimalDigits DecimalDigit
  /// DecimalDigit:: one of
  ///      0 1 2 3 4 5 6 7 8 9
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_decimal_digits(&mut self) -> bool {
    let start = self.index();

    self.last_int_value = 0;
    while let Some(cp) = self.code_point_with_offset(0) {
      if !cp.is_ascii_digit() {
        break;
      }
      self.last_int_value = 10 * self.last_int_value
        + self
          .code_point_with_offset(0)
          .unwrap()
          .to_digit(10)
          .unwrap() as i64;
      self.advance();
    }

    self.index() != start
  }

  /// Eat the next characters as a `HexDigits` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// HexDigits::
  ///      HexDigit
  ///      HexDigits HexDigit
  /// HexDigit:: one of
  ///      0 1 2 3 4 5 6 7 8 9 a b c d e f A B C D E F
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_hex_digits(&mut self) -> bool {
    let start = self.index();
    self.last_int_value = 0;
    while let Some(cp) = self.code_point_with_offset(0) {
      if !cp.is_ascii_hexdigit() {
        break;
      }
      self.last_int_value =
        16 * self.last_int_value + cp.to_digit(16).unwrap() as i64;
      self.advance();
    }
    self.index() != start
  }

  /// Eat the next characters as a `HexDigits` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// LegacyOctalEscapeSequence::
  ///      OctalDigit [lookahead ∉ OctalDigit]
  ///      ZeroToThree OctalDigit [lookahead ∉ OctalDigit]
  ///      FourToSeven OctalDigit
  ///      ZeroToThree OctalDigit OctalDigit
  /// OctalDigit:: one of
  ///      0 1 2 3 4 5 6 7
  /// ZeroToThree:: one of
  ///      0 1 2 3
  /// FourToSeven:: one of
  ///      4 5 6 7
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_legacy_octal_escape_sequence(&mut self) -> bool {
    if self.eat_octal_digit() {
      let n1 = self.last_int_value;
      if self.eat_octal_digit() {
        let n2 = self.last_int_value;
        if n1 <= 3 && self.eat_octal_digit() {
          self.last_int_value += n1 * 64 + n2 * 8;
        } else {
          self.last_int_value = n1 * 8 + n2;
        }
      } else {
        self.last_int_value = n1;
      }
      true
    } else {
      false
    }
  }

  /// Eat the next characters as a `OctalDigit` production if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// OctalDigit:: one of
  ///      0 1 2 3 4 5 6 7
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_octal_digit(&mut self) -> bool {
    if let Some(cp) = self.code_point_with_offset(0) {
      if cp.is_digit(8) {
        self.advance();
        self.last_int_value = cp.to_digit(8).unwrap() as i64;
        return true;
      }
    }
    self.last_int_value = 0;
    false
  }

  /// Eat the next characters as the given number of `HexDigit` productions if possible.
  /// Set `self.last_int_value` if it ate the next characters successfully.
  /// ```grammar
  /// HexDigit:: one of
  ///      0 1 2 3 4 5 6 7 8 9 a b c d e f A B C D E F
  /// ```
  /// Returns `true` if it ate the next characters successfully.
  fn eat_fixed_hex_digits(&mut self, length: i64) -> bool {
    let start = self.index();
    self.last_int_value = 0;
    for _ in 0..length {
      let cp = self.code_point_with_offset(0);
      if cp.is_none() || !cp.unwrap().is_ascii_hexdigit() {
        self.rewind(start);
        return false;
      }
      self.last_int_value =
        16 * self.last_int_value + cp.unwrap().to_digit(16).unwrap() as i64;
      self.advance();
    }
    true
  }

  fn count_capturing_parens(&mut self) -> u32 {
    let start = self.index();
    let mut in_class = false;
    let mut escaped = false;
    let mut count = 0;

    while let Some(cp) = self.code_point_with_offset(0) {
      if escaped {
        escaped = false;
      } else if cp == '\\' {
        escaped = true;
      } else if cp == '[' {
        in_class = true;
      } else if cp == ']' {
        in_class = false;
      } else if cp == '('
        && !in_class
        && (self.code_point_with_offset(1) != Some('?')
          || (self.code_point_with_offset(2) == Some('<')
            && self.code_point_with_offset(3) != Some('=')
            && self.code_point_with_offset(3) != Some('!')))
      {
        count += 1
      }
      self.advance();
    }

    self.rewind(start);
    count
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn count_capturing_parens_test() {
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    let source = "foo|(abc)de";
    validator.reset(source, 0, source.len(), false);
    assert_eq!(validator.count_capturing_parens(), 1);
    let source = "foo|(?:abc)de";
    validator.reset(source, 0, source.len(), false);
    assert_eq!(validator.count_capturing_parens(), 0);
    let source = "((foo)|(abc)de)";
    validator.reset(source, 0, source.len(), false);
    assert_eq!(validator.count_capturing_parens(), 3);
  }
}
