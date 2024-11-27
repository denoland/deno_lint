Prevent comment strings being accidentally passed as text in JSX.

### Invalid:

```tsx
const foo = <div>// comment</div>;
const foo = <div>/* comment */</div>;
```

### Valid:

```tsx
const foo = <div>{/* comment */}</div>;
```
