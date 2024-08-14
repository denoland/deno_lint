Disallows the use of NodeJS global objects.

NodeJS exposes a set of global objects that differs from deno (and the web), so
code should not assume they are available. Instead, import the objects from
their defining modules as needed.

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
