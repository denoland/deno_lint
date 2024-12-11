`@deprecated` tags must provide additional information, such as the reason for
deprecation or suggested alternatives.

### Invalid:

```typescript
/**
 * @deprecated
 */
export function oldFunction(): void {}
```

### Valid:

```typescript
/**
 * @deprecated since version 2.0. Use `newFunction` instead.
 */
export function oldFunction(): void {}
```
