Disallows ternary operators when simpler alternatives exist.

It's a common mistake to use a conditional expression to select between two
Boolean values instead of using ! to convert the test to a Boolean.

### Invalid:

```typescript
// You don't need a ternary when a simple `const foo = !condition` will do
const foo = condition ? false : true;
// use `const foo = condition` instead
const foo = condition ? true : false;
```

### Valid:

```typescript
const foo = x === 2 ? "Yes" : "No";
const foo = x === 2 ? "Yes" : false;
const foo = x === 2 ? true : "No";
```
