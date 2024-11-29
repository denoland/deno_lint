// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use std::fmt::Display;

#[derive(Debug, Hash, PartialEq)]
pub struct Tag(&'static str);

pub type Tags = &'static [Tag];

impl Display for Tag {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Tag {
    pub fn display(&self) -> &'static str {
        self.0
    }
}

pub const RECOMMENDED: Tag = Tag("recommended");
pub const FRESH: Tag = Tag("fresh");
pub const JSR: Tag = Tag("jsr");
pub const REACT: Tag = Tag("react");
pub const JSX: Tag = Tag("jsx");


pub const ALL_TAGS: Tags = &[
    RECOMMENDED,
    FRESH,
    JSR,
    REACT,
    JSX,
];
