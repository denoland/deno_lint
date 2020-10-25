Disallows the use of a constant expression in conditional test

Using a constant expression in a conditional test is often either a mistake or a
temporary situation introduced during development and is not ready for production.

### Invalid:
```typescript
if (true) {}
if (2) {}
for (;2;) {}  // infinite loop
while (typeof i) {}  // infinite loop
do {} while (x = 2);  // infinite loop
const x = 0 ? "a" : "b";
```

### Valid:
```typescript
if (x) {}
if (x === 0) {}
for (;;;) {}  // allowed infinite loop
while (typeof i === "string") {}
do {} while (x === 2);  // infinite loop
const x === 0 ? "a" : "b";
```