Disallows ternary operators when simpler alternatives exist.

It's a common mistake to use a conditional expression to select between two
Boolean values instead of using ! to convert the test to a Boolean.

### Invalid:

```typescript
const foo = condition ? true : false;
```

### Valid:

```typescript
const foo = condition ? x : y;
const foo = condition ? x : false;
```
