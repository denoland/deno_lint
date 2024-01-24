Warns the usage of the deprecated Deno APIs

The following APIs in the `Deno` namespace are now marked as deprecated and will
get removed from the namespace in Deno 2.0.

**IO APIs**

- `Deno.Buffer`
- `Deno.copy`
- `Deno.iter`
- `Deno.iterSync`
- `Deno.readAll`
- `Deno.readAllSync`
- `Deno.writeAll`
- `Deno.writeAllSync`

Most of these APIs have been moved to [`std/io`](https://deno.land/std/io) of
the Deno Standard Library. For more detail, see
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
const a = new Deno.Buffer();

// read all
const b = await Deno.readAll(reader);
const c = Deno.readAllSync(reader);

// write all
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

// file
function foo(file: Deno.File) {
  // ...
}
```

### Valid:

```typescript
// buffer
import { Buffer } from "https://deno.land/std/io/buffer.ts";
const a = new Buffer();

// read all
import { readAll, readAllSync } from "https://deno.land/std/io/read_all.ts";
const b = await readAll(reader);
const c = readAllSync(reader);

// write all
import { writeAll, writeAllSync } from "https://deno.land/std/io/write_all.ts";
await writeAll(writer, data);
writeAllSync(writer, data);

// iter
// reader is `ReadableStream`
for await (const chunk of reader) {
  // do something
}

// copy
// reader is `ReadableStream` and writer is `WritableStream`
await reader.pipeTo(writer);

// custom inspector
class A {
  [Symbol.for("Deno.customInspect")]() {
    return "This is A";
  }
}

// file
function foo(file: Deno.FsFile) {
  // ...
}
```
