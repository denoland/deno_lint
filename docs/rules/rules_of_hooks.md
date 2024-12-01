Ensure that (P)React hooks are only called inside component functions and not
called conditionally. A hook is a function that starts with `use`

### Invalid:

```tsx
// BAD: Called conditionally
function Component() {
  if (cond) {
    const [count, setCount] = useState(0);
  }
  // ...
}

// BAD: Called in a loop
function Component() {
  for (let i = 0; i < 10; i++) {
    const [count, setCount] = useState(0);
  }
  // ...
}

// BAD: Called after conditional return
function Component() {
  if (cond) {
    return;
  }

  const [count, setCount] = useState(0);
  // ...
}

// BAD: Called inside event handler
function Component() {
  function onClick() {
    const [count, setCount] = useState(0);
  }

  return <button onClick={onClick}>click me</button>;
}

// BAD: Called inside useMemo
function Component() {
  const value = useMemo(() => {
    const [count, setCount] = useState(0);
    return count;
  });
}

// BAD: Called inside try/catch
function Component() {
  try {
    const [count, setCount] = useState(0);
  } catch {
    const [count, setCount] = useState(0);
  }

  // ...
}
```

### Valid:

```tsx
function Component() {
  const [count, setCount] = useState(0);
  // ...
}

function useCustomHook() {
  const [count, setCount] = useState(0);
  // ...
}
```
