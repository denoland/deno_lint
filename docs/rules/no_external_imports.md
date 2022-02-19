Disallows the use of external imports

- what's the motivation of this lint rule?
  - this rule emits warnings if external modules are imported via URL. "mod.ts"
    and import maps are exception.
- why is linted code considered bad?
  - importing external modules just works fine, but it will take time and effort
    when you want to upgrade those modules if they are imported in multiple
    places in your project.
- who should use it?
  - to avoid it you could use "mod.ts convention" or
    [import maps](https://deno.land/manual/linking_to_external_code/import_maps),
    where you import all external modules and then re-export them or assign
    aliases to them.
  - so if you'd like to follow the "mod.ts convention" or to use import maps,
    this rule is for you.

### Invalid:

```typescript
import { assertEquals } from "https://deno.land/std@0.126.0/testing/asserts.ts";
```

### Valid:

```typescript
import { assertEquals } from "deps.ts";
```

`mod.ts`:

```
{
   "imports": {
        "fmt/": "https://deno.land/std@0.126.0/fmt/"
   }
}
```

- in "mod.ts", it's totally okay to import an external module via URL
- module maps can also be used to specify an external module URL
