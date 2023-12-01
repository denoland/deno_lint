// Copyright 2020-2023 the Deno authors. All rights reserved. MIT license.

#[cfg(feature = "wasm")]
mod noop_perf;
#[cfg(not(feature = "wasm"))]
mod perf;

#[cfg(feature = "wasm")]
pub use noop_perf::Perf;
#[cfg(not(feature = "wasm"))]
pub use perf::Perf;
