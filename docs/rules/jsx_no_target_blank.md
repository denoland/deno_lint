Older browsers don't set the `rel="noopener"` attribute wich can lead to
security issues, see
[MDN rel="noopener"](https://developer.mozilla.org/en-US/docs/Web/HTML/Attributes/rel/noopener).

### Invalid:

```tsx
<a target="_blank" />
<a target="_blank" rel="foo" />
<a target="_blank" rel="noreferrer" />
```

### Valid:

```tsx
<a target="_blank" rel="noopener" />
<a target={"_blank"} rel={"noopener"} />
<a target={`_blank`} rel={`noopener`} />
<a target={foo && "_blank"} rel={foo && "noopener"} />
<a target={foo ? "_blank" : null} rel={foo ? "noopener": null} />
```
