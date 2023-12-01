// Copyright 2020-2023 the Deno authors. All rights reserved. MIT license.

pub struct Perf;

impl Perf {
  pub fn start() -> Self {
    Self
  }

  pub fn mark(&mut self, _label: impl Into<String>) {
    // noop
  }
}
