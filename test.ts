// an interface with any number of members
interface Foo {
  name: string;
}

// same as above
interface Bar {
  age: number;
}

// an interface with more than one supertype
// in this case the interface can be used as a replacement of a union type.
interface Baz extends Foo, Bar {}

const first_name = "";
function foo_bar() {}
