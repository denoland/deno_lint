// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::time::Instant;

/// A struct to measure how long a function takes to execute.
///
/// When the struct is dropped, `debug!` is used to print the measurement.
pub struct PerformanceMark {
  name: String,
  start: Instant,
}

impl PerformanceMark {
  pub fn new(name: &str) -> Self {
    Self {
      name: name.to_string(),
      start: Instant::now(),
    }
  }
}

impl Drop for PerformanceMark {
  fn drop(&mut self) {
    let end = Instant::now();
    debug!("{} took {:#?}", self.name, end - self.start);
  }
}
