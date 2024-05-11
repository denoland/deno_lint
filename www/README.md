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
directory. To update these files, run `deno task wasmbuild` at the repository
root.
