#[macro_use]
extern crate lazy_static;

mod colors;
mod diagnostic;
mod linter;
mod rules;
mod scopes;
mod swc_util;

#[cfg(test)]
mod test_util;