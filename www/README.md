# lint.deno.land

### Usage

Start the project:

```
deno task start
```

This will watch the project directory and restart as necessary.

### Playground

There is a playground available at https://lint.deno.land/playground where we
can write random source code and see the diagnostics right away.

#### How to update wasm files used in playground

This feature is realized using wasm and its glue code put in the `static`
directory. To update these files, follow the steps below. (Note: These steps
assume that you are in the root directory of this repository)

1. Make sure you have `wasm-bindgen-cli` v0.2.92 installed:

```shell
cargo install -f wasm-bindgen-cli --version 0.2.92
```

2. Build deno_lint for the wasm target.

```shell
cargo build --target wasm32-unknown-unknown --features wasm --release
```

3. Run `wasm-bindgen-cli` to generate the glue code and to put everything needed
   into the `static` directory:

```shell
wasm-bindgen --out-dir=www/static --target=web ./target/wasm32-unknown-unknown/release/deno_lint.wasm
```

4. (Optional, but preferable) Reduce the size of the wasm binary using
   [the binaryen toolkit](https://github.com/WebAssembly/binaryen):

```shell
wasm-opt -Os -o www/static/deno_lint_bg.wasm www/static/deno_lint_bg.wasm
```
