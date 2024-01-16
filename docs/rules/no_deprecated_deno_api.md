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
// readAll
import { toArrayBuffer } from "https://deno.land/std/streams/to_array_buffer.ts";
const b = await toArrayBuffer(reader); // `b` is ArrayBuffer
const c = new Uint8Array(b); // You can convert ArrayBuffer to Uint8Array

// writeAll
// reader is `ReadableStream` and writer is `WritableStream`
const reader = ReadableStream.from([1, 2, 3]);
await reader.pipeTo(writer);

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

function foo(file: Deno.FsFile) {
  // ...
}
```
