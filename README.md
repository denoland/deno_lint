# `deno_lint`

A Rust crate for writing fast JavaScript and TypeScript linters.

This crate powers [`deno lint`](https://deno.land/manual/tools/linter), but is not Deno specific
and can be used to write linters for Node as well.

_Supports `recommended` set of rules from ESLint and `@typescript-eslint` out of the box
with no config._

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
    "totalMs": 247.20262200000025,
    "runsCount": 5,
    "measuredRunsAvgMs": 49.44052440000005,
    "measuredRunsMs": [
      49.016501999999946,
      49.56810500000006,
      49.68610600000011,
      48.97360200000003,
      49.958307000000104
    ]
  },
  {
    "name": "eslint",
    "totalMs": 12214.295835,
    "runsCount": 5,
    "measuredRunsAvgMs": 2442.859167,
    "measuredRunsMs": [
      2703.5126729999997,
      2380.431925,
      2369.1452910000007,
      2362.1451909999996,
      2399.0607550000004
    ]
  }
]
```

_Benchmarks are run during CI on Ubuntu, using the same set of rules for both linters.
Test subject is [`oak` server](https://github.com/oakserver/oak) consisting of about 50 files.
See [`./benchmarks/`](./benchmarks/) directory for more info._

## Example

`examples/dlint/main.rs` provides a minimal standalone binary demonstrating
how `deno_lint` can be used as a crate.

```shell
$ ▶ target/debug/examples/dlint ../deno/std/http/server.ts ../deno/std/http/file_server.ts
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

For more concrete implementation visit [`deno`](https://github.com/denoland/deno/blob/master/cli/lint.rs)

## Developing

Make sure to have latest stable version of Rust installed (1.47.0).

```shell
// check version
$ rustc --version
rustc 1.47.0 (18bf6b4f0 2020-10-07)

// build all targets
$ cargo build --all-targets

// test it
$ cargo test
```

### Generating flamegraph (Linux)

Prerequisites:

- Install [`perf`](https://perf.wiki.kernel.org/index.php/Main_Page), [`stackcollapse-perf`](https://github.com/brendangregg/FlameGraph/blob/master/flamegraph.pl), [`rust-unmangle`](https://github.com/Yamakaky/rust-unmangle/blob/master/rust-unmangle) and [`flamegraph`](https://github.com/brendangregg/FlameGraph/blob/master/flamegraph.pl)

```shell
$ RUSTFLAGS='-g' cargo build --release --all-targets # build target
$ sudo perf record --call-graph dwarf ./target/release/examples/dlint benchmarks/oak/**.ts # create performance profile
$ perf script | stackcollapse-perf | rust-unmangle | flamegraph > flame.svg # generate flamegraph
```

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
5. Make sure `deno run --allow-run tools/lint.ts` passes.
