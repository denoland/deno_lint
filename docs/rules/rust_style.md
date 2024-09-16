Enforces the use of Rust-style naming conventions, see
https://rust-lang.github.io/api-guidelines/naming.html

Consistency in a code base is key for readability and maintainability. This rule
is useful for deno projects that call rust functions via FFI. It attempts to
unify naming conventions and enforces declarations and object property names
which you create to be\
in UpperCamelCase/PascalCase for classes, types, interfaces,\
in snake_case for functions, methods, variables\
and in SCREAMING_SNAKE_CASE for static class properties and constants.

Of note:

- `_` is allowed at the start or end of a variable
- All uppercase variable names (e.g. constants) may have `_` in their name
- If you have to use a camelCase key in an object for some reasons, wrap it in
  quotation mark
- This rule also applies to variables imported or exported via ES modules, but
  not to object properties of those variables

### Invalid:

```typescript
let firstName = "Ichigo";
const obj1 = { lastName: "Hoshimiya" };
const obj2 = { firstName };

function doSomething() {}
function foo({ camelCase = "default value" }) {}

class snake_case_class {}
class camelCaseClass {}
class Also_Not_Valid_Class {}

export * as camelCased from "mod.ts";

enum snake_case_enum {
  snake_case_variant,
}

enum camelCasedEnum {
  camelCasedVariant,
}

type snake_case_type = { some_property: number };

type camelCasedType = { someProperty: number };

interface snake_case_interface {
  some_property: number;
}

interface camelCasedInterface {
  someProperty: number;
}
```

### Valid:

```typescript
let first_name = "Ichigo";
const FIRST_NAME = "Ichigo";
const __my_private_variable = "Hoshimiya";
const my_private_variable_ = "Hoshimiya";
const obj1 = { "lastName": "Hoshimiya" }; // if an object key is wrapped in quotation mark, then it's valid
const obj2 = { "firstName": firstName };
const { lastName } = obj1; // valid, because one has no control over the identifier
const { lastName: last_name } = obj;

function do_something() {} // function declarations must be snake_case but...
doSomething(); // ...camel_case function calls are allowed
function foo({ camelCase: snake_case = "default value" }) {}

class PascalCaseClass {}

import { camelCased } from "external-module.js"; // valid, because one has no control over the identifier
import { camelCased as not_camel_cased } from "external-module.js";
export * as not_camel_cased from "mod.ts";

enum PascalCaseEnum {
  PascalCaseVariant,
}

type PascalCaseType = { some_property: number };

interface PascalCaseInterface {
  some_property: number;
}
```
