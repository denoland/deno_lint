Disallows the declaration of an empty interface

An interface with no members serves no purpose.  Either the interface extends
another interface, in which case the supertype can be used, or it does not
extend a supertype in which case it is the equivalent to an empty object.  This
rule will capture these situations as either unnecessary code or a mistaken
empty implementation.

### Invalid:

```typescript
interface Foo {}
interface Foo extends Bar {}
```

### Valid:

```typescript
interface Foo {
  name: string;
}

interface Bar {
  age: number;
}

// Using an empty interface as a union type is allowed
interface Baz extends Foo, Bar {}
```
