Require `for-in` loops to include an `if` statement

Looping over objects with a `for-in` loop will include properties that are
inherited through the prototype chain. This behavior can lead to unexpected
items in your for loop.

### Invalid:

```typescript
for (key in obj) {
  foo(obj, key);
}
```

### Valid:

```typescript
for (key in obj) {
  if (!Object.hasOwn(obj, key)) {
    continue;
  }
  foo(obj, key);
}
```
