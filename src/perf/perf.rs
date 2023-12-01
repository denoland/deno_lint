// Copyright 2020-2023 the Deno authors. All rights reserved. MIT license.

use std::time::Instant;

pub struct Perf {
  origin: Instant,
  entries: Vec<Entry>,
}

struct Entry {
  label: String,
  duration: Duration,
}

impl Perf {
  pub fn start() -> Self {
    Self {
      origin: Instant::now(),
      entries: Vec::new(),
    }
  }

  pub fn mark(&mut self, label: impl Into<String>) {
    let duration = self.origin.elapsed();
    self.entries.push(Entry {
      label: label.into(),
      duration,
    });
  }
}

impl Drop for Perf {
  fn drop(&mut self) {
    for entry in &self.entries {
      debug!("{} took {:#?}", entry.label, entry.duration);
    }
  }
}
