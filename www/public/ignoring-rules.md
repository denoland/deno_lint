## Ignore directives

### File level

To ignore a whole file use `// deno-lint-ignore-file` at the top of the file:

```ts
// deno-lint-ignore-file

function foo(): any {
  // ...
}
```

The ignore directive must be placed before the first statement or declaration:

```ts
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

/**
 * Some JS doc
 **/

// deno-lint-ignore-file

import { bar } from "./bar.js";

function foo(): any {
  // ...
}
```

You can also ignore certain diagnostics in the whole file:

```ts
// deno-lint-ignore-file no-explicit-any no-empty

function foo(): any {
  // ...
}
```

If there are multiple `// deno-lint-ignore-file` directives, all but the first
one are ignored:

```ts
// This is effective
// deno-lint-ignore-file no-explicit-any no-empty

// But this is NOT effective
// deno-lint-ignore-file no-debugger

function foo(): any {
  debugger; // not ignored!
}
```

### Line level

To ignore specific diagnostics use `// deno-lint-ignore <codes...>` on the preceding
line of the offending line.

```ts
// deno-lint-ignore no-explicit-any
function foo(): any {
  // ...
}

// deno-lint-ignore no-explicit-any explicit-function-return-type
function bar(a: any) {
  // ...
}
```

You must specify the names of the rules to be ignored.

## Ignore `ban-unused-ignore` itself

deno_lint provides `ban-unused-ignore` rule, which will detect ignore directives
that don't ever suppress certain diagnostics. This is useful when you want to discover
ignore directives that are no longer necessary after refactoring the code.

In some cases, however, you might want to ignore `ban-unused-ignore` rule itself.
One of the typical cases would be when working with auto-generated files; it makes
sense to add file-level ignore directives for some rules, and there's almost no
need for detecting unused directives via `ban-unused-ignore` in this case.

You can use `// deno-lint-ignore-file ban-unused-ignore` as always if you want to
suppress the rule for a whole file:

```ts
// deno-lint-ignore-file ban-unused-ignore no-explicit-any

// `no-explicit-any` isn't used but you'll get no diagnostics because of ignoring
// `ban-unused-ignore`
console.log(42);
```

Note that if you want to ignore unused directives per line, you must place `ban-unused-ignore`
on the same line as other rule codes. This is a bit different from how `// deno-lint-ignore`
works normally, in the sense that `// deno-lint-ignore ban-unused-ignore` works
for the _same_ line while the normal `// deno-lint-ignore <codes...>` works for
the _next_ line.

```ts
// deno-lint-ignore ban-unused-ignore no-explicit-any
console.log(42);
```
