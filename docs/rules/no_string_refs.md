String `refs` are deprecated and should not be used. Use callback-based refs or
`useRef` instead.

### Invalid:

```tsx
<div ref="foo" />
<App ref="foo" />
```

### Valid:

```tsx
<div ref={(c) => {}} />;

function Foo() {
  const ref = useRef();
  return <Bar ref={ref} />;
}
```
