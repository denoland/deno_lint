// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

mod reader;
mod unicode;
mod validator;
use std::fmt;

pub use validator::{EcmaRegexValidator, EcmaVersion};

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy)]
pub struct UnicodeChar {
  value: u32,
}

impl fmt::Display for UnicodeChar {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    write!(f, "{}", self.value)
  }
}

macro_rules! delegate_if_char {
  ($($v: vis fn $fn: ident ($self: ident $(,)? $($param: ident : $param_ty : ty),*) $(-> $ret: ty)?);+ $(;)?) => {
    $(
      $v fn $fn($self $(, $param : $param_ty)*) $(-> $ret)? {
        if $self.is_scalar() {
          return unsafe { char::from_u32_unchecked($self.value) }.$fn($($param),*);
        }
        Default::default()
      }
    )+
  };
}

impl UnicodeChar {
  /// Returns true if the character is a Unicode scalar value.
  /// In other words, whether it could be validly represented as a `char` in Rust.
  pub fn is_scalar(self) -> bool {
    let v = self.value;
    (v ^ 0xD800).wrapping_sub(0x800) < 0x110000 - 0x800
  }
  delegate_if_char! {
    pub fn is_digit(self, radix: u32) -> bool;
    pub fn to_digit(self, radix: u32) -> Option<u32>;
    pub fn is_ascii_digit(self) -> bool;
    pub fn is_ascii_alphabetic(self) -> bool;
    pub fn is_ascii_hexdigit(self) -> bool;
  }
  pub fn to_char(self) -> Option<char> {
    char::from_u32(self.value)
  }
}

impl From<char> for UnicodeChar {
  fn from(value: char) -> Self {
    Self {
      value: value as u32,
    }
  }
}

impl From<u32> for UnicodeChar {
  fn from(value: u32) -> Self {
    Self { value }
  }
}

impl PartialEq<char> for UnicodeChar {
  fn eq(&self, other: &char) -> bool {
    self.value == *other as u32
  }
}

impl PartialEq<u32> for UnicodeChar {
  fn eq(&self, other: &u32) -> bool {
    self.value == *other
  }
}
impl PartialOrd<u32> for UnicodeChar {
  fn partial_cmp(&self, other: &u32) -> Option<std::cmp::Ordering> {
    self.value.partial_cmp(other)
  }
}

impl UnicodeChar {
  pub fn to_u32(self) -> u32 {
    self.value
  }
  pub fn to_i64(self) -> i64 {
    self.value as i64
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use validator::UnicodeMode;

  #[test]
  fn valid_flags() {
    let validator = EcmaRegexValidator::new(EcmaVersion::Es2024);
    assert_eq!(validator.validate_flags(""), Ok(UnicodeMode::None));

    assert_eq!(validator.validate_flags("gimuys"), Ok(UnicodeMode::Unicode));
    assert_eq!(validator.validate_flags("gimuy"), Ok(UnicodeMode::Unicode));
    assert_eq!(validator.validate_flags("gim"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("g"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("i"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("m"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("s"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("u"), Ok(UnicodeMode::Unicode));
    assert_eq!(validator.validate_flags("v"), Ok(UnicodeMode::UnicodeSets));
    assert_eq!(validator.validate_flags("y"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("d"), Ok(UnicodeMode::None));

    assert_eq!(validator.validate_flags("gy"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("iy"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("my"), Ok(UnicodeMode::None));
    assert_eq!(validator.validate_flags("uy"), Ok(UnicodeMode::Unicode));
  }

  #[test]
  fn duplicate_flags() {
    let validator = EcmaRegexValidator::new(EcmaVersion::Es2024);
    assert_eq!(
      validator.validate_flags("gimgu"),
      Err("Duplicated flag g".to_string())
    );
    assert_eq!(
      validator.validate_flags("migg"),
      Err("Duplicated flag g".to_string())
    );
    assert_eq!(
      validator.validate_flags("igi"),
      Err("Duplicated flag i".to_string())
    );

    assert_eq!(
      validator.validate_flags("ii"),
      Err("Duplicated flag i".to_string())
    );
    assert_eq!(
      validator.validate_flags("mm"),
      Err("Duplicated flag m".to_string())
    );
    assert_eq!(
      validator.validate_flags("ss"),
      Err("Duplicated flag s".to_string())
    );
    assert_eq!(
      validator.validate_flags("uu"),
      Err("Duplicated flag u".to_string())
    );
    assert_eq!(
      validator.validate_flags("yy"),
      Err("Duplicated flag y".to_string())
    );
    assert_eq!(
      validator.validate_flags("uv"),
      Err("Cannot use u and v flags together".to_string())
    );
    assert_eq!(
      validator.validate_flags("dgimsuvy"),
      Err("Cannot use u and v flags together".to_string())
    );
  }

  #[test]
  fn invalid_flags() {
    let validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_eq!(
      validator.validate_flags("gimuf"),
      Err("Invalid flag f".to_string())
    );
    assert_eq!(
      validator.validate_flags("gI"),
      Err("Invalid flag I".to_string())
    );
    assert_eq!(
      validator.validate_flags("a"),
      Err("Invalid flag a".to_string())
    );
    assert_eq!(
      validator.validate_flags("1"),
      Err("Invalid flag 1".to_string())
    );
  }

  #[test]
  fn validate_pattern_test() {
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_eq!(validator.validate_pattern("", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("[abc]de|fg", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[abc]de|fg", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("^.$", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("^.$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("foo\\[bar", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("foo\\[bar", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\w+\\s", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(\\w+), (\\w+)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("\\/\\/.*|\\/\\*[^]*\\*\\/", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("(\\d{1,2})-(\\d{1,2})-(\\d{4})", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(?:\\d{3}|\\(\\d{3}\\))([-\\/\\.])\\d{3}\\1\\d{4}",
        UnicodeMode::None
      ),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("https?:\\/\\/(www\\.)?[-a-zA-Z0-9@:%._\\+~#=]{1,256}\\.[a-zA-Z0-9()]{1,6}\\b([-a-zA-Z0-9()@:%_\\+.~#?&//=]*)", UnicodeMode::None), Ok(()));

    assert_eq!(
      validator.validate_pattern("\\p{Script=Greek}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\p{Alphabetic}", UnicodeMode::Unicode),
      Ok(())
    );

    assert_ne!(validator.validate_pattern("\\", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("a**", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("++a", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("?a", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("a***", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("a++", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("a+++", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a???", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a????", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("*a", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("**a", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("+a", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("[{-z]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[a--z]", UnicodeMode::None),
      Ok(())
    );

    assert_ne!(
      validator.validate_pattern("0{2,1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("x{1}{1,}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("x{1,2}{1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("x{1,}{1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("x{0,1}{1,}", UnicodeMode::None),
      Ok(())
    );

    assert_ne!(
      validator.validate_pattern("\\1(\\P{P\0[}()/", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn character_range_order() {
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_ne!(
      validator.validate_pattern("^[z-a]$", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-ac-e]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[c-eb-a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[a-dc-b]", UnicodeMode::None),
      Ok(())
    );

    assert_ne!(
      validator.validate_pattern("[\\10b-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\ad-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\bd-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\Bd-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\db-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\Db-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\sb-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\Sb-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\wb-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\Wb-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\0b-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\td-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\nd-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\vd-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\fd-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\rd-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c0001d-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\x0061d-G]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0061d-G]", UnicodeMode::None),
      Ok(())
    );

    assert_ne!(
      validator.validate_pattern("[b-G\\10]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\b]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\B]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-G\\d]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-G\\D]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-G\\s]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-G\\S]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-G\\w]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-G\\W]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-G\\0]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\t]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\n]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\v]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\f]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\r]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\c0001]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\x0061]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[d-G\\u0061]", UnicodeMode::None),
      Ok(())
    );
  }

  #[test]
  fn unicode_quantifier_without_atom() {
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_ne!(
      validator.validate_pattern("*", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("+", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1,}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1,2}", UnicodeMode::Unicode),
      Ok(())
    );

    assert_ne!(
      validator.validate_pattern("*?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("+?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("??", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1,}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1,2}?", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn unicode_incomplete_quantifier() {
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_ne!(
      validator.validate_pattern("a{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1,", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1,2", UnicodeMode::Unicode),
      Ok(())
    );

    assert_ne!(
      validator.validate_pattern("{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1,", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{1,2", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn unicode_single_bracket() {
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_ne!(
      validator.validate_pattern("(", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(")", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("}", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn unicode_escapes() {
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_eq!(
      validator.validate_pattern("\\u{10ffff}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u{110000}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{110000}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("foo\\ud803\\ude6dbar", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("(\u{12345}|\u{23456}).\\1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\u{12345}{3}", UnicodeMode::Unicode),
      Ok(())
    );

    // unicode escapes in character classes
    assert_eq!(
      validator.validate_pattern("[\\u0062-\\u0066]oo", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0062-\\u0066]oo", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("[\\u{0062}-\\u{0066}]oo", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("[\\u{62}-\\u{00000066}]oo", UnicodeMode::Unicode),
      Ok(())
    );

    // invalid escapes
    assert_eq!(
      validator
        .validate_pattern("first\\u\\x\\z\\8\\9second", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u\\x\\z\\8\\9]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\u/u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\u12/u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\ufoo/u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\x/u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\xfoo/u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\z/u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\8/u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("/\\9/u", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn basic_valid() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/visitor/full.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_eq!(validator.validate_pattern("foo", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("foo|bar", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("||||", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^|$|\\b|\\B", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=foo)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?!)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?!foo)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a)*", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a)+", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a)?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a){", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a){}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a){a}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a){1}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a){1,}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=a){1,2}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("a*", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("a+", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("a?", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("a{", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("a{}", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("a{a}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("a{1", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("a{1,}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,2}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,2", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{2,1", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("a*?", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("a+?", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("a??", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("a{?", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("a{}?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{a}?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1}?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,}?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,2}?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,2?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{2,1?", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("👍🚀❇️", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("^", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("$", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern(".", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("]", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("{", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("}", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("|", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("${1,2", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("\\1", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("(a)\\1", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\1(a)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?:a)\\1", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(a)\\2", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?:a)\\2", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\\10",
        UnicodeMode::None
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\\11",
        UnicodeMode::None
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\\11",
        UnicodeMode::None
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?:a)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("\\d", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\D", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\s", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\S", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\w", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\W", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\f", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\n", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\r", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\t", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\v", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("\\cA", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\cz", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\c1", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("\\c", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\0", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\u", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("\\u1", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u12", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u123", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u1234", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u12345", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{z", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{a}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{20", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{20}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{10FFFF}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{110000}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{00000001}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\377", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\400", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("\\^", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\$", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\.", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\+", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\?", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\(", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\)", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\[", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\]", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\{", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\}", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\|", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\/", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("\\a", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("[]", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("[^-a-b-]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("[-]", UnicodeMode::None), Ok(()));
    assert_eq!(validator.validate_pattern("[a]", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("[--]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[-a]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[-a-]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[a-]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[a-b]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[-a-b-]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[---]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[a-b--/]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\b-\\n]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[b\\-a]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\d]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\D]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\s]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\S]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\w]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\W]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\f]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\n]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\r]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\t]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\v]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\cA]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\cz]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\c1]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\c]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\0]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\x]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\xz]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\x1]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\x12]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\x123]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u1]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u12]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u123]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u1234]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u12345]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{z]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{a}]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{20]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{20}]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{10FFFF}]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{110000}]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{00000001}]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\77]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\377]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\400]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\^]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\$]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\.]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\+]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\?]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\(]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\)]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\[]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\]]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\{]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\}]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\|]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\/]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\a]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\d-\\uFFFF]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\D-\\uFFFF]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\s-\\uFFFF]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\S-\\uFFFF]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\w-\\uFFFF]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\W-\\uFFFF]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\d]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\D]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\s]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\S]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\w]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\W]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\u0001]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{2-\\u{1}]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\a-\\z]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[0-9--/]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\c0-]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\c_]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9]+$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[a-zA-Z]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[a-zA-Z]+$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9a-zA-Z]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("^[a-zA-Z0-9!-/:-@\\[-`{-~]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^([a-zA-Z0-9]{8,})$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^([a-zA-Z0-9]{6,8})$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^([0-9]{0,8})$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9]{8}$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^https?:\\/\\/", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^\\d{3}-\\d{4}$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^\\d{1,3}(.\\d{1,3}){3}$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("^([1-9][0-9]*|0)(\\.[0-9]+)?$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("^-?([1-9][0-9]*|0)(\\.[0-9]+)?$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[ぁ-んー]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[ァ-ンヴー]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[ｧ-ﾝﾞﾟ\\-]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[^\\x20-\\x7e]*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\\.[a-zA-Z0-9-]+)*$",
        UnicodeMode::None
      ),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("^((4\\d{3})|(5[1-5]\\d{2})|(6011))([- ])?\\d{4}([- ])?\\d{4}([- ])?\\d{4}|3[4,7]\\d{13}$", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("^\\s*|\\s*$", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("[\\d][\\12-\\14]{1,}[^\\d]", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("([a ]\\b)*\\b", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("foo", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("foo|bar", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("||||", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^|$|\\b|\\B", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?=foo)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?!)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?!foo)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a*", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a+", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,2}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a*?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a+?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a??", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("a{1,2}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("👍🚀❇️", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(".", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("|", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(a)\\1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\1(a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\\10",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\\11",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?:a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\d", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\D", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\s", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\S", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\w", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\W", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\f", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\n", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\r", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\t", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\v", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\cA", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\cz", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\0", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u1234", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u12345", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{a}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{20}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{10FFFF}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\u{00000001}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\^", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\.", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\+", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\(", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\[", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\|", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\/", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[^-a-b-]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[-]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[a]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[--]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[-a]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[-a-]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[a-]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[a-b]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[-a-b-]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[---]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[a-b--/]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\b-\\n]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[b\\-a]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\d]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\D]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\s]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\S]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\w]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\W]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\f]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\n]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\r]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\t]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\v]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\cA]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\cz]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\0]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\x12]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\x123]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u1234]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u12345]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{a}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{20}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{10FFFF}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{00000001}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\^]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\$]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\.]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\+]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\?]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\(]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\)]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\[]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\]]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\{]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\|]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\/]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-\\u0001]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u{1}-\\u{2}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[0-9--/]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[🌷-🌸]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("[\\u0000-🌸-\\u0000]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("[\\u0000-\\u{1f338}-\\u0000]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "[\\u0000-\\ud83c\\udf38-\\u0000]",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "[\\uD834\\uDF06-\\uD834\\uDF08a-z]",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9]+$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[a-zA-Z]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[a-zA-Z]+$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9a-zA-Z]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("^[a-zA-Z0-9!-/:-@\\[-`{-~]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^([a-zA-Z0-9]{8,})$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^([a-zA-Z0-9]{6,8})$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^([0-9]{0,8})$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[0-9]{8}$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^https?:\\/\\/", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^\\d{3}-\\d{4}$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("^\\d{1,3}(.\\d{1,3}){3}$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "^([1-9][0-9]*|0)(\\.[0-9]+)?$",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "^-?([1-9][0-9]*|0)(\\.[0-9]+)?$",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[ぁ-んー]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[ァ-ンヴー]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[ｧ-ﾝﾞﾟ\\-]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("^[^\\x20-\\x7e]*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9-]+(?:\\.[a-zA-Z0-9-]+)*$",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("^((4\\d{3})|(5[1-5]\\d{2})|(6011))([- ])?\\d{4}([- ])?\\d{4}([- ])?\\d{4}|3[4,7]\\d{13}$", UnicodeMode::Unicode), Ok(()));
    assert_eq!(
      validator.validate_pattern("^\\s*|\\s*$", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<=a)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<=a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<!a)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<!a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<=(?<a>\\w){3})f", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("((?<=\\w{3}))f", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>(?<=\\w{3}))f", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<!(?<a>\\d){3})f", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<!(?<a>\\D){3})f|f", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>(?<!\\D{3}))f|f", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<=(?<a>\\w){3})f", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("((?<=\\w{3}))f", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>(?<=\\w{3}))f", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<!(?<a>\\d){3})f", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>(?<!\\D{3}))f|f", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("(?<=(?<fst>.)|(?<snd>.))", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("(a)", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("(?<a>)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("\\k", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("\\k<a>", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>a)\\k<a>", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>a)\\k<a>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>a)\\1", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>a)\\1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>a)\\2", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>a)(?<b>a)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a>a)(?<b>a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\k<a>(?<a>a)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\k<a>(?<a>a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\1(?<a>a)", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\1(?<a>a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<$abc>a)\\k<$abc>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<あ>a)\\k<あ>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("(?<𠮷>a)\\k<\\u{20bb7}>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(?<\\uD842\\uDFB7>a)\\k<\\u{20bb7}>",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(?<\\u{20bb7}>a)\\k<\\uD842\\uDFB7>",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(?<abc>a)\\k<\\u0061\\u0062\\u0063>",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(?<\\u0061\\u0062\\u0063>a)\\k<abc>",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "(?<\\u0061\\u0062\\u0063>a)\\k<\\u{61}\\u{62}\\u{63}>",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("(?<a1>a)\\k<a1>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(validator.validate_pattern("\\p", UnicodeMode::None), Ok(()));
    assert_eq!(
      validator.validate_pattern("\\p{", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\p{ASCII", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\p{ASCII}", UnicodeMode::None),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\p{ASCII}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\p{Emoji}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator
        .validate_pattern("\\p{General_Category=Letter}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\p{Script=Hiragana}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern(
        "[\\p{Script=Hiragana}\\-\\p{Script=Katakana}]",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_eq!(
      validator.validate_pattern("\\P{Letter}", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn basic_invalid() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/basic-invalid.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es5);
    assert_ne!(validator.validate_pattern("(", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("(?", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("(?=", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?=foo", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(?!", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?!foo", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(a{2,1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1}?", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(*)", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("+", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("?", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern(")", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("[", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("^*", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("$*", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("${1,2}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("${2,1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\2(a)(", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(?a", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(?:", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?:a", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(:a", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("[b-a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[a-b--+]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0001-\\u0000]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{1}-\\u{2}]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{2}-\\u{1}]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\z-\\a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[0-9--+]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c-a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[🌷-🌸]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[🌸-🌷]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(
        "[\\uD834\\uDF06-\\uD834\\uDF08a-z]",
        UnicodeMode::None
      ),
      Ok(())
    );
  }

  #[test]
  fn basic_invalid_2015() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/basic-invalid-2015.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2015);
    assert_ne!(validator.validate_pattern("(", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("(?", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("(?=", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?=foo", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(?!", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?!foo", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(a{2,1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1}?", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(*)", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("+", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("?", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern(")", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("[", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("^*", UnicodeMode::None), Ok(()));
    assert_ne!(validator.validate_pattern("$*", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("${1,2}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("${2,1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\2(a)(", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(?a", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(?:", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?:a", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(:a", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("[b-a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[a-b--+]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0001-\\u0000]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{1}-\\u{2}]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{2}-\\u{1}]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\z-\\a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[0-9--+]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c-a]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[🌷-🌸]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0000-🌸-\\u0000]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(
        "[\\u0000-\\ud83c\\udf38-\\u0000]",
        UnicodeMode::None
      ),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[🌸-🌷]", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(
        "[\\uD834\\uDF06-\\uD834\\uDF08a-z]",
        UnicodeMode::None
      ),
      Ok(())
    );
  }

  #[test]
  fn basic_invalid_2015_unicode() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/basic-invalid-2015-u.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2015);
    assert_ne!(
      validator.validate_pattern("(", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=foo", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?!", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?!foo", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a)*", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a)+", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a)?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a){", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a){}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a){a}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a){1}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a){1,}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?=a){1,2}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{a}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1,", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1,2", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(a{2,1}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{a}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1,?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{1,2?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1}?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("a{2,1?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(*)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("+", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(")", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("^*", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("$*", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("${1,2", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("${1,2}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("${2,1}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\2(a)(", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?:a)\\1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(a)\\2", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?:a)\\2", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(
        "(a)(a)(a)(a)(a)(a)(a)(a)(a)(a)\\11",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?a", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?:", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?:a", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(:a", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\c1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\c", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u1", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u12", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u123", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u{z", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u{20", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\u{110000}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\377", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\400", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\a", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[b-a]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[a-b--+]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c1]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\x]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\xz]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\x1]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u1]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u12]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u123]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{z]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{20]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{110000}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\77]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\377]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\400]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\a]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\d-\\uFFFF]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\D-\\uFFFF]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\s-\\uFFFF]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\S-\\uFFFF]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\w-\\uFFFF]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\W-\\uFFFF]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0000-\\d]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0000-\\D]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0000-\\s]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0000-\\S]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0000-\\w]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0000-\\W]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u0001-\\u0000]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{2}-\\u{1}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\u{2-\\u{1}]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\a-\\z]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\z-\\a]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[0-9--+]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c-a]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c0-]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[\\c_]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("[🌸-🌷]", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator
        .validate_pattern("[\\d][\\12-\\14]{1,}[^\\d]", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn lookbehind_assertion_invalid_2017() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/lookbehind-assertion-invalid-2017.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2017);
    assert_ne!(
      validator.validate_pattern("(?<a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn lookbehind_assertion_invalid_2018() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/lookbehind-assertion-invalid-2018.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_ne!(
      validator.validate_pattern("(?<a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)?", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)+", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)+", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)*", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a)*", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a){1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<=a){1}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)?", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)?", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)+", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)+", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)*", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a)*", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a){1}", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<!a){1}", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn named_capturing_group_invalid_2017() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/named-capturing-group-invalid-2017.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2017);
    assert_ne!(
      validator.validate_pattern("\\k", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\k<a>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<a", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<a", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<a>", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<a>", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn named_capturing_group_invalid_2018() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/named-capturing-group-invalid-2018.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_ne!(validator.validate_pattern("(?a", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(validator.validate_pattern("(?<", UnicodeMode::None), Ok(()));
    assert_ne!(
      validator.validate_pattern("(?<)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\k", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\k<a>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<a", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<a", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\2", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<b>", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)\\k<b>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)(?<a>a)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)(?<a>a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)(?<\\u{61}>a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<a>a)(?<\\u0061>a)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<☀>a)\\k<☀>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator
        .validate_pattern("(?<\\u0020>a)\\k<\\u0020>", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(
        "(?<\\u0061\\u0062\\u0063>a)\\k<abd>",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<11>a)\\k<11>", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn unicode_group_names_invalid_2020() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/unicode-group-names-invalid.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2020);
    assert_ne!(
      validator.validate_pattern("(?<\\ud83d\\ude80>.)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<\\ud83d\\ude80>.)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<\\u{1f680}>.)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<\\u{1f680}>.)", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<🚀>.)", UnicodeMode::None),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("(?<🚀>.)", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn unicode_property_escape_invalid_2017() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/unicode-property-escape-invalid-2017.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2017);
    assert_ne!(
      validator.validate_pattern("\\p", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{ASCII", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{ASCII}", UnicodeMode::Unicode),
      Ok(())
    );
  }

  #[test]
  fn unicode_property_escape_invalid_2018() {
    // source: https://github.com/mysticatea/regexpp/blob/master/test/fixtures/parser/literal/unicode-property-escape-invalid-2018.json
    let mut validator = EcmaRegexValidator::new(EcmaVersion::Es2018);
    assert_ne!(
      validator.validate_pattern("\\p", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{ASCII", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{General_Category}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator
        .validate_pattern("\\p{General_Category=}", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{General_Category", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern("\\p{General_Category=", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator
        .validate_pattern("\\p{General_Category=Letter", UnicodeMode::Unicode),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(
        "\\p{General_Category=Hiragana}",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
    assert_ne!(
      validator.validate_pattern(
        "[\\p{Script=Hiragana}-\\p{Script=Katakana}]",
        UnicodeMode::Unicode
      ),
      Ok(())
    );
  }
}
