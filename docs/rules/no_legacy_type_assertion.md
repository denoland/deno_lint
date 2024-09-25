Disallows the use of legacy `<Type> value` type assertion syntax in TypeScript
code.

`<Type> value` casting syntax is considered to be outdated because it does not
work in JSX. Instead, you should use `value as Type`.

### Invalid:

```typescript
const foo = <Foo> bar;
```

### Valid:

```typescript
const foo = bar as Foo;
```
