Disallow sync function inside async function

Using sync functions like `Deno.readTextFileSync` blocks the deno event loop so
its not recomonded to use it inside of an async function, because its stops
progress of all other async tasks.

### Invalid:

```javascript
async function foo() {
  Deno.readTextFileSync("");
}

const fooFn = async function foo() {
  Deno.readTextFileSync("");
};

const fooFn = async () => {
  Deno.readTextFileSync("");
};
```

### Valid:

```javascript
function foo() {
  Deno.readTextFileSync("");
}

const fooFn = function foo() {
  Deno.readTextFileSync("");
};

const fooFn = () => {
  Deno.readTextFileSync("");
};
```
