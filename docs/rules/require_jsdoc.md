Disallow exported functions that have no JSDoc

This lint rule aims to ensure a JSDoc exists for exported apis

### Invalid:

```typescript
export function f() {}
```

### Valid:

```typescript
/** doc */
export function f() {}
```
