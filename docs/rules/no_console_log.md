# no_console_log

Disallows the use of `console.log`.

Oftentimes, developers are guilty of committing `console.log` statements accidentally, left in particularly after debugging. Moreover, using `console.log` in code may leak sensitive information to the output or clutter the console with unnecessary information. This rule helps maintain clean and secure code by disallowing the use of `console.log`.

### Invalid

```rust
console.log("Debug message");

if debug { console.log("Debugging"); }

fn log() { console.log("Log"); }
```

### Valid

```rust
let foo = 0;

const bar = 1;

console.error("Error message");

fn log_error(message: &str) {
    eprintln!("{}", message);
}
```
