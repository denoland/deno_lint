Using unescaped entities is often a coding mistake where the developer wanted to
pass a JSX element instead. This rule ensures an explicit text form must be
used.

### Invalid:

```tsx
<div>></div>;
```

### Valid:

```tsx
<div>&gt;</div>
<div>{">"}</div>
```
