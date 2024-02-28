Enforces type imports to be declared as type imports.

This rule ensures that the code works when the `verbatimModuleSyntax`
is enabled. This is useful in library distributing TypeScript as sources
to work in more scenarios. 

### Invalid:

```typescript
import { TypeOnly } from "./a.ts";

const value: TypeOnly = getValue();
console.log(value);
```

```typescript
import { TypeOnly, alterValue } from "./a.ts";

const value: TypeOnly = getValue();
console.log(alterValue(value));
```

### Valid:

```typescript
import type { TypeOnly } from "./a.ts";

const value: TypeOnly = getValue();
console.log(value);
```

```typescript
import { type TypeOnly, alterValue } from "./a.ts";

const value: TypeOnly = getValue();
console.log(alterValue(value));
```
