Disallows mixed spaces and tabs for indentation.

Most code conventions require either tabs or spaces be used for indentation.
Therefore, if a line of code is indented with both tabs and spaces, it's most
likely a mistake of a developer.

### Invalid:

```typescript
function add(x: number, y: number) {
  return x + y; // indented with a tab + two spaces
}
```

```typescript
let x = 5, // indented with a tab
  y = 7; // indented with a tab + four spaces
```

### Valid:

```typescript
function add(x: number, y: number) {
  return x + y;
}
```
