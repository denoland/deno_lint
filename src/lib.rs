// Copyright 2020 the Deno authors. All rights reserved. MIT license.

#![deny(warnings)]

#[macro_use]
extern crate lazy_static;

pub mod colors;
pub mod diagnostic;
pub mod linter;
pub mod rules;

mod scopes;
mod swc_util;

#[cfg(test)]
mod test_util;
