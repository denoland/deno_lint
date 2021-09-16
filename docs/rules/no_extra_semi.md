Disallows the use of unnecessary semi-colons

Extra (and unnecessary) semi-colons can cause confusion when reading the code as
well as making the code less clean.

### Invalid:

```typescript
const x = 5;
;
function foo() {};
```

### Valid:

```typescript
const x = 5;

function foo() {}
```
