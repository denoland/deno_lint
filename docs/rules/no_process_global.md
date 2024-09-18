Disallows the use of NodeJS process global.

NodeJS and Deno expose process global but they are hard to statically analyze by tools, so
code should not assume they are available. Instead, import process from "node:process" instead.

### Invalid:

```typescript
// foo.ts
const foo = process.env.FOO; // process is not a global object in deno
```

### Valid:

```typescript
// foo.ts
import process from "node:process";

const foo = process.env.FOO;
```
