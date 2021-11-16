Disallow exported functions that have no JSDoc

This lint rule aims to check for presence of jsdoc comments on public apis, such
as class declarations, interface declarations and functions.

### Invalid:

```typescript
export function f() {} // missing jsdoc comment on function f

export class A { // missing jsdoc comment on class A
  constructor() {} // missing jsdoc comment on constructor

  method() {} // missing jsdoc comment on method
}

export interface B { // missing jsdoc comment on interface B
  bar(): void; // missing jsdoc comment on method bar
}

export default () => {}; // missing jsdoc comment on this function
```

### Valid:

```typescript
/** doc */
export function f() {}

/** doc */
export class A {
  /** doc */
  constructor() {}

  /** doc */
  method() {}
}

/** doc */
export interface A {
  /** doc */
  method(): void;
}
```
