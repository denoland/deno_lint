Ensure that inline dependency imports have a version specifier.

### Invalid:

```ts
import foo from "npm:chalk";
import foo from "jsr:@std/path";
```

### Valid:

```ts
import foo from "npm:chalk@5.3.0";
import foo from "npm:chalk@^5.3.0";
import foo from "jsr:@std/path@1.0.8";
import foo from "jsr:@std/path@^1.0.8";
```
