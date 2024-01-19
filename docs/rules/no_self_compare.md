Disallows comparisons where both sides are exactly the same.

Comparing a variable against itself is usually an error, either a typo or
refactoring error. It is confusing to the reader and may potentially introduce a
runtime error.

### Invalid:

```typescript
if (x === x) {
}
if (a.b === a.b) {
}
if (a["b"] === a["b"]) {
}
```

### Valid:

```typescript
if (x === y) {
}
if (a.b === a.c) {
}
if (a["b"] === a["c"]) {
}
```
