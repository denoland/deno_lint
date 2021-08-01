Disallows async functions that have no await expression

In general, the primary reason to use async functions is to use await
expressions inside. If an async function has no await expression, it is most
likely an unintentional mistake.

### Invalid:

```typescript
async function f1() {
  doSomething();
}

const f2 = async () => {
  doSomething();
};

const f3 = async () => doSomething();

const obj = {
  async method() {
    doSomething();
  },
};

class MyClass {
  async method() {
    doSomething();
  }
}
```

### Valid:

```typescript
await asyncFunction();

function normalFunction() {
  doSomething();
}

async function f1() {
  await asyncFunction();
}

const f2 = async () => {
  await asyncFunction();
};

const f3 = async () => await asyncFunction();

async function f4() {
  for await (const num of asyncIterable) {
    console.log(num);
  }
}

// empty functions are valid
async function emptyFunction() {}
const emptyArrowFunction = async () => {};

// generators are also valid
async function* gen() {
  console.log(42);
}
```
