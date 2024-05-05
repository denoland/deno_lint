// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::time::Instant;

/// A struct to measure how long a function takes to execute.
///
/// When the struct is dropped, `debug!` is used to print the measurement.
pub struct PerformanceMark {
  name: &'static str,
  start: Option<Instant>,
}

impl PerformanceMark {
  pub fn new(name: &'static str) -> Self {
    Self {
      name,
      start: if log::log_enabled!(log::Level::Debug) {
        Some(Instant::now())
      } else {
        None
      },
    }
  }
}

impl Drop for PerformanceMark {
  fn drop(&mut self) {
    if let Some(start) = self.start {
      let end = Instant::now();
      debug!("{} took {:#?}", self.name, end - start);
    }
  }
}
