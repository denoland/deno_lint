Disallows the use of the `window` object.

Using the `window` global is deprecated and scheduled for removal in Deno 2.0.
Deno does not have a window and `typeof window === "undefined"` is often used to
tell if the code is running in the browser.

### Invalid:

```typescript
const a = await window.fetch("https://deno.land");

const b = window.Deno.metrics();
console.log(window);
```

### Valid:

```typescript
const a1 = await fetch("https://deno.land");
const a2 = await globalThis.fetch("https://deno.land");
const a3 = await self.fetch("https://deno.land");

const b1 = Deno.metrics();
const b2 = globalThis.Deno.metrics();
const b3 = self.Deno.metrics();
console.log(globalThis);
```
