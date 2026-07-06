// Copyright 2018-2024 the Deno authors. All rights reserved. MIT license.

use deno_ast::oxc::allocator::Allocator;
use deno_ast::MediaType;
use deno_ast::ModuleSpecifier;
use deno_ast::ParseDiagnostic;
use deno_ast::ParsedSource;

pub(crate) fn parse_program<'a>(
  allocator: &'a Allocator,
  specifier: ModuleSpecifier,
  media_type: MediaType,
  source_code: String,
) -> Result<ParsedSource<'a>, ParseDiagnostic> {
  let source_type = deno_ast::get_source_type(media_type);
  deno_ast::parse_program(
    allocator,
    deno_ast::ParseParams {
      specifier,
      media_type,
      text: source_code.into(),
      capture_tokens: true,
      maybe_source_type: Some(source_type),
      scope_analysis: true,
    },
  )
}
