Disallows the use of external imports

### Invalid:

```typescript
import { assertEquals } from "https://deno.land/std@0.126.0/testing/asserts.ts";
```

### Valid:

```typescript
import { assertEquals } from "deps.ts";
```
