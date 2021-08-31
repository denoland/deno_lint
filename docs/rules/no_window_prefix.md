Disallows the use of Web APIs via the `window` object.

In most situations, the global variable `window` works like `globalThis`. For
example, you could call the `fetch` API like `window.fetch(..)` instead of
`fetch(..)` or `globalThis.fetch(..)`. In Web Workers, however, `window` is not
available, but instead `self`, `globalThis`, or no prefix work fine. Therefore,
for compatibility between Web Workers and other contexts, it's highly
recommended to not access global properties via `window`.

Note that the following properties are allowed to call with `window`:

- `onload`
- `onunload`
- `closed`
- `alert`
- `confirm`
- `prompt`
- `localStorage`
- `sessionStorage`
- `window`
- `Navigator`

because these APIs are not supported in Workers. Additionally, `location` is
also allowed because what it points to in the Window context is different from
that in Web Workers.

### Invalid:

```typescript
const a = await window.fetch("https://deno.land");

const b = window.Deno.metrics();
```

### Valid:

```typescript
const a1 = await fetch("https://deno.land");
const a2 = await globalThis.fetch("https://deno.land");
const a3 = await self.fetch("https://deno.land");

const b1 = Deno.metrics();
const b2 = globalThis.Deno.metrics();
const b3 = self.Deno.metrics();

// `alert` is allowed to call with `window` because it's not supported in Workers
window.alert("🍣");

// `location` is also allowed
window.location.host;
```
