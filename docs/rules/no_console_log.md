# no_console_log

Disallows the use of `console.log`.

Oftentimes, developers are guilty of committing `console.log` statements
accidentally, left in particularly after debugging. Moreover, using
`console.log` in code may leak sensitive information to the output or clutter
the console with unnecessary information. This rule helps maintain clean and
secure code by disallowing the use of `console.log`.

### Invalid

```typescript
console.log("Debug message");

if debug { console.log("Debugging"); }

function log() { console.log("Log"); }
```

### Valid

```typescript
console.error("Error message");

function log_error(message: string) {
  console.warn(message);
}
```
