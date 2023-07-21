# `deno_lint`

[![](https://img.shields.io/crates/v/deno_lint.svg)](https://crates.io/crates/deno_lint)
[![Discord Chat](https://img.shields.io/discord/684898665143206084?logo=discord&style=social)](https://discord.gg/deno)

A Rust crate for writing fast JavaScript and TypeScript linters.

This crate powers [`deno lint`](https://deno.land/manual/tools/linter), but is
not Deno specific and can be used to write linters for Node as well.

_Supports `recommended` set of rules from ESLint and `@typescript-eslint` out of
the box with no config._

See [the roadmap](https://github.com/denoland/deno_lint/issues/176)

---

## Supported rules

Visit https://lint.deno.land for the list of available rules.

## Performance

Blazing fast, see comparison with ESLint:

```json
[
  {
    "name": "deno_lint",
    "totalMs": 105.3750100000002,
    "runsCount": 5,
    "measuredRunsAvgMs": 21.07500200000004,
    "measuredRunsMs": [
      24.79783199999997,
      19.563640000000078,
      20.759051999999883,
      19.99068000000011,
      20.26380600000016
    ]
  },
  {
    "name": "eslint",
    "totalMs": 11845.073306000002,
    "runsCount": 5,
    "measuredRunsAvgMs": 2369.0146612000003,
    "measuredRunsMs": [
      2686.1039550000005,
      2281.501061,
      2298.6185210000003,
      2279.5962849999996,
      2299.2534840000008
    ]
  }
]
```

_Benchmarks are run during CI on Ubuntu, using the same set of rules for both
linters. Test subject is [`oak` server](https://github.com/oakserver/oak)
consisting of about 50 files. See [`./benchmarks/`](./benchmarks/) directory for
more info._

## Node.js bindings

If you want to use `deno_lint` with Node, please refer to
[`@node-rs/deno-lint`](https://www.npmjs.com/package/@node-rs/deno-lint) package
which provides programmatic API as well as Webpack loader for `deno_lint`.

## Example

`examples/dlint/main.rs` provides a minimal standalone binary demonstrating how
`deno_lint` can be used as a crate.

```shell
# Build standalone binary
$ cargo build --example dlint

$ ./target/debug/examples/dlint --help

dlint

USAGE:
    dlint <SUBCOMMAND>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

SUBCOMMANDS:
    help     Prints this message or the help of the given subcommand(s)
    rules
    run

$ ./target/debug/examples/dlint run ../deno/std/http/server.ts ../deno/std/http/file_server.ts
(no-empty) Empty block statement
  --> ../deno/std/http/server.ts:93:14
   |
93 |       } catch {}
   |               ^^
   |
(no-empty) Empty block statement
   --> ../deno/std/http/server.ts:111:44
    |
111 |     while ((await body.read(buf)) !== null) {}
    |                                             ^^
    |
(no-empty) Empty block statement
   --> ../deno/std/http/server.ts:120:41
    |
120 |   constructor(public listener: Listener) {}
    |                                          ^^
    |
(ban-untagged-todo) TODO should be tagged with (@username) or (#issue)
 --> ../deno/std/http/file_server.ts:5:0
  |
5 | // TODO Stream responses instead of reading them into memory.
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
(ban-untagged-todo) TODO should be tagged with (@username) or (#issue)
 --> ../deno/std/http/file_server.ts:6:0
  |
6 | // TODO Add tests like these:
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
(ban-untagged-todo) TODO should be tagged with (@username) or (#issue)
   --> ../deno/std/http/file_server.ts:137:0
    |
137 | // TODO: simplify this after deno.stat and deno.readDir are fixed
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
(no-empty) Empty block statement
   --> ../deno/std/http/file_server.ts:155:16
    |
155 |     } catch (e) {}
    |                 ^^
    |
Found 7 problems
```

For more concrete implementation visit
[`deno`](https://github.com/denoland/deno/blob/main/cli/tools/lint.rs)

## Developing

Make sure to have latest stable version of Rust installed (1.56.0).

```shell
// check version
$ rustc --version
rustc 1.56.0 (09c42c458 2021-10-18)

// build all targets
$ cargo build --all-targets

// test it
$ cargo test
```

### Generating flamegraph (Linux)

Prerequisites:

- Install [`perf`](https://perf.wiki.kernel.org/index.php/Main_Page),
  [`stackcollapse-perf`](https://github.com/brendangregg/FlameGraph/blob/master/flamegraph.pl),
  [`c++filt`](https://sourceware.org/binutils/docs/binutils/c_002b_002bfilt.html)
  and
  [`flamegraph`](https://github.com/brendangregg/FlameGraph/blob/master/flamegraph.pl)

```shell
$ RUSTFLAGS='-g' cargo build --release --all-targets # build target
$ sudo perf record --call-graph dwarf ./target/release/examples/dlint benchmarks/oak/**.ts # create performance profile
$ perf script | stackcollapse-perf | c++filt | flamegraph > flame.svg # generate flamegraph
```

You can use
[rust-unmangle](https://github.com/Yamakaky/rust-unmangle/blob/master/rust-unmangle)
or [rustfilt](https://github.com/luser/rustfilt) instead of c++filt.

These commands can take a few minutes to run.

## Contributing

- If you are going to work on an issue, mention so in the issue comments
  _before_ you start working on the issue.

- Please be professional in the forums. We follow
  [Rust's code of conduct](https://www.rust-lang.org/policies/code-of-conduct)
  (CoC) Have a problem? Email ry@tinyclouds.org.

- Ask for help in the [community chat room](https://discord.gg/TGMHGv6).

## Submitting a Pull Request

Before submitting, please make sure the following is done:

1. That there is a related issue and it is referenced in the PR text.
2. There are tests that cover the changes.
3. Ensure `cargo test` passes.
4. Format your code with `deno run --allow-run tools/format.ts`
5. Make sure `deno run --allow-run --allow-env tools/lint.ts` passes.
6. If you've added a new rule:
   1. Run `cargo build --example dlint --all-features`
   2. Update docs by running the generated binary with these arguments
      `./target/debug/examples/dlint rules --json > www/static/docs.json`
