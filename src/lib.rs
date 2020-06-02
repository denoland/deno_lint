#[macro_use]
extern crate lazy_static;

pub mod diagnostic;
pub mod linter;
pub mod rules;

mod colors;
mod scopes;
mod swc_util;

#[cfg(test)]
mod test_util;
