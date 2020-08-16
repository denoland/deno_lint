// Copyright 2020 the Deno authors. All rights reserved. MIT license.

use std::collections::VecDeque;

#[derive(Debug, Default)]
pub struct Reader {
  unicode: bool,
  src: String,
  index: usize,
  end: usize,
  cps: VecDeque<char>,
}

impl Reader {
  pub fn new() -> Self {
    Self {
      unicode: false,
      src: "".to_owned(),
      index: 0,
      end: 0,
      cps: VecDeque::with_capacity(4),
    }
  }

  pub fn source(&self) -> &str {
    &self.src
  }

  pub fn index(&self) -> usize {
    self.index
  }

  pub fn code_point_with_offset(&self, offset: usize) -> Option<char> {
    self.cps.get(offset).cloned()
  }

  pub fn reset(
    &mut self,
    source: &str,
    start: usize,
    end: usize,
    u_flag: bool,
  ) {
    self.unicode = u_flag;
    self.src = source.to_owned();
    self.end = end;
    self.rewind(start);
  }

  pub fn rewind(&mut self, index: usize) {
    self.index = index;
    self.cps.clear();
    for i in 0..4 {
      let w_sum: usize = i;
      if let Some(c) = self.at(index + w_sum) {
        self.cps.push_back(c);
      } else {
        break;
      }
    }
  }

  pub fn advance(&mut self) {
    if self.cps.get(0).is_some() {
      self.index += 1;
      self.cps.pop_front();
      if let Some(c) = self.at(self.index + self.cps.len()) {
        self.cps.push_back(c);
      }
    }
  }

  pub fn eat(&mut self, cp: char) -> bool {
    let opt = self.cps.get(0);
    if opt.is_some() && *opt.unwrap() == cp {
      self.advance();
      true
    } else {
      false
    }
  }

  pub fn eat2(&mut self, cp1: char, cp2: char) -> bool {
    let (opt1, opt2) = (self.cps.get(0), self.cps.get(1));
    if opt1.is_some()
      && opt2.is_some()
      && *opt1.unwrap() == cp1
      && *opt2.unwrap() == cp2
    {
      self.advance();
      self.advance();
      true
    } else {
      false
    }
  }

  pub fn eat3(&mut self, cp1: char, cp2: char, cp3: char) -> bool {
    let (opt1, opt2, opt3) =
      (self.cps.get(0), self.cps.get(1), self.cps.get(2));
    if opt1.is_some()
      && opt2.is_some()
      && opt3.is_some()
      && *opt1.unwrap() == cp1
      && *opt2.unwrap() == cp2
      && *opt3.unwrap() == cp3
    {
      self.advance();
      self.advance();
      self.advance();
      true
    } else {
      false
    }
  }

  fn at(&self, i: usize) -> Option<char> {
    if i >= self.end {
      None
    } else if self.unicode {
      let c = self.src.chars().nth(i).unwrap();
      Some(c)
    } else {
      // TODO: move the conversion out of this method and make it safe
      unsafe {
        let c: char = std::char::from_u32_unchecked(
          self.src.encode_utf16().nth(i).unwrap() as u32,
        );
        Some(c)
      }
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn eat_test() {
    let mut reader = Reader::new();
    reader.reset("abcdefghijk", 0, 11, true);
    assert_eq!(reader.eat('a'), true);
    assert_eq!(reader.eat3('b', 'd', 'd'), false);
    assert_eq!(reader.eat3('b', 'c', 'd'), true);
    assert_eq!(reader.eat2('e', 'f'), true);
    assert_eq!(reader.eat('h'), false);
    assert_eq!(reader.eat('g'), true);
    assert_eq!(reader.eat2('h', 'i'), true);
    assert_eq!(reader.eat3('j', 'k', 'a'), false);
  }

  #[test]
  fn rewind_test() {
    let mut reader = Reader::new();
    reader.reset("abcd", 0, 4, true);
    assert_eq!(reader.eat('a'), true);
    assert_eq!(reader.eat3('b', 'd', 'd'), false);
    assert_eq!(reader.eat3('b', 'c', 'd'), true);
    reader.rewind(0);
    assert_eq!(reader.eat('a'), true);
    assert_eq!(reader.eat3('b', 'd', 'd'), false);
    assert_eq!(reader.eat3('b', 'c', 'd'), true);
  }

  #[test]
  fn at_test_es_compliance() {
    let mut reader = Reader::new();
    // without unicode flag
    reader.reset("Hello", 0, 5, false);
    assert_eq!(reader.at(1).unwrap() as u32, 101);
    reader.reset("􀃃a🩢☃★♲", 0, 6, false);
    assert_eq!(reader.at(0).unwrap() as u32, 56256);
    reader.reset("􀃃ello", 0, 6, false);
    assert_eq!(reader.at(0).unwrap() as u32, 56256);
    reader.reset("􀃃ello", 0, 6, false);
    assert_eq!(reader.at(1).unwrap() as u32, 56515);
    // with unicode flag
    reader.reset("Hello", 0, 5, true);
    assert_eq!(reader.at(1).unwrap() as u32, 101);
    reader.reset("􀃃a🩢☃★♲", 0, 6, true);
    assert_eq!(reader.at(0).unwrap() as u32, 1048771);
    reader.reset("􀃃ello", 0, 6, true);
    assert_eq!(reader.at(0).unwrap() as u32, 1048771);
  }
}
