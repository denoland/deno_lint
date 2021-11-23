Disallows the use of `true` and `false` as arguments.

### Invalid:

```typescript
function foo(a: boolean) {
  console.log(a);
}

foo(false);
```

### Valid:

```typescript
function foo(a: boolean) {
  console.log(a);
}

const bar = false;
foo(bar);
```
