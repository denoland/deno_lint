Warns the usage of the deprecated Deno APIs

The following APIs in `Deno` namespace are now marked as deprecated and will get
removed from the namespace in the future.

**IO APIs**

- `Deno.Buffer`
- `Deno.copy`
- `Deno.iter`
- `Deno.iterSync`
- `Deno.readAll`
- `Deno.readAllSync`
- `Deno.writeAll`
- `Deno.writeAllSync`

The IO APIs are already available in `std/io` or `std/streams`, so replace these
deprecated ones with alternatives from `std`. For more detail, see
[the tracking issue](https://github.com/denoland/deno/issues/9795).

**Sub Process API**

- `Deno.run`

`Deno.run` was deprecated in favor of `Deno.Command`. See
[deno#9435](https://github.com/denoland/deno/discussions/9435) for more details.

**Custom Inspector API**

- `Deno.customInspect`

`Deno.customInspect` was deprecated in favor of
`Symbol.for("Deno.customInspect")`. Replace the usages with this symbol
expression. See [deno#9294](https://github.com/denoland/deno/issues/9294) for
more details.

**File system API**

- `Deno.File`

`Deno.File` was deprecated in favor of `Deno.FsFile`. Replace the usages with
new class name.

### Invalid:

```typescript
// buffer
const a = Deno.Buffer();

// read
const b = await Deno.readAll(reader);
const c = Deno.readAllSync(reader);

// write
await Deno.writeAll(writer, data);
Deno.writeAllSync(writer, data);

// iter
for await (const x of Deno.iter(xs)) {}
for (const y of Deno.iterSync(ys)) {}

// copy
await Deno.copy(reader, writer);

// custom inspector
class A {
  [Deno.customInspect]() {
    return "This is A";
  }
}

function foo(file: Deno.File) {
  // ...
}
```

### Valid:

```typescript
// buffer
import { Buffer } from "https://deno.land/std/io/buffer.ts";
const a = new Buffer();

// read
import { readAll, readAllSync } from "https://deno.land/std/io/util.ts";
const b = await readAll(reader);
const c = readAllSync(reader);

// write
import { writeAll, writeAllSync } from "https://deno.land/std/io/util.ts";
await writeAll(writer, data);
writeAllSync(writer, data);

// iter
import { iter, iterSync } from "https://deno.land/std/io/util.ts";
for await (const x of iter(xs)) {}
for (const y of iterSync(ys)) {}

// copy
import { copy } from "https://deno.land/std/io/util.ts";
await copy(reader, writer);

// custom inspector
class A {
  [Symbol.for("Deno.customInspect")]() {
    return "This is A";
  }
}

function foo(file: Deno.FsFile) {
  // ...
}
```
