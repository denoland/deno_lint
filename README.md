# `deno_lint`

[![](https://img.shields.io/crates/v/deno_lint.svg)](https://crates.io/crates/deno_lint)
[![Discord Chat](https://img.shields.io/discord/684898665143206084?logo=discord&style=social)](https://discord.gg/deno)

A Rust crate for writing JavaScript and TypeScript linters.

This crate powers
[`deno lint`](https://docs.deno.com/runtime/reference/cli/lint/), but is not
Deno specific and can be used as a standalone crate.

It ships with a set of built-in rules, including a `recommended` set that is
enabled by default and requires no configuration.

## Supported rules

The list of available rules and their documentation is hosted at
[docs.deno.com/lint/rules](https://docs.deno.com/lint/rules/).

## Example

To see how `deno_lint` is used as a crate, look at how it is integrated in
[`deno`](https://github.com/denoland/deno/blob/main/cli/tools/lint/mod.rs).

## Developing

Make sure to have the latest stable version of Rust installed, see
[rust-toolchain.toml](./rust-toolchain.toml).

```shell
# check version
$ rustc --version

# build all targets
$ cargo build --all-targets

# test it
$ cargo test
```

## Contributing

- If you are going to work on an issue, mention so in the issue comments
  _before_ you start working on the issue.

- Please be professional in the forums. We follow
  [Rust's code of conduct](https://www.rust-lang.org/policies/code-of-conduct)
  (CoC).

- Ask for help in the [community chat room](https://discord.gg/deno).

## Submitting a Pull Request

Before submitting, please make sure the following is done:

1. That there is a related issue and it is referenced in the PR text.
2. There are tests that cover the changes.
3. Ensure `cargo test` passes.
4. Format your code with `deno run --allow-run tools/format.ts`.
5. Make sure `deno run --allow-run --allow-env tools/lint.ts` passes.
6. If you've added a new rule, open a PR to
   [denoland/docs](https://github.com/denoland/docs) with documentation for the
   rule.
