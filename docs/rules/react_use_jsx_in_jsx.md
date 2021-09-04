Encourages React users to use the same contextual rendering mechanism.

If a user is expressing components in JSX-style React, prompt the user to apply
consistency in rendering components via JSX, which uses `React.createElement`,
versus plain javascript, which is still valid React when consuming function
components.

### Invalid:

```typescript
<div>{MyComponent()}</div>;
```

### Valid:

```typescript
<div>
  <MyComponent />
</div>;
```

```typescript
React.createElement("div", {}, MyComponent());
```
