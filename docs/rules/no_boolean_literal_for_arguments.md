Requires all functions called with more than one argument to
use a self-documenting constant if at least a single `boolean`
literal is provided.

Is common to define functions that can take `booleans` as
arguments. However, passing `boolean` literals as parameters
can lead to lack of context regarding the role of argument
inside the function in question. It is important to note
that a function that takes a single argument may actually
benefit from said pattern since the semantics may work as documentation
if proper naming conventions are used.

A simple fix for the points mentioned above is the use of
self documenting constants that will end up working as "named booleans"
that allow for a better understanding on what the parameters
mean in the context of the function call.

### Invalid
```typescript
function redraw(allViews: boolean, inline: boolean) {
  // redraw logic.
}
redraw(true, true);

function executeCommand(recursive: boolean, executionMode: EXECUTION_MODES) {
  // executeCommand logic.
}
executeCommand(true, EXECUTION_MODES.ONE);
```

### Valid
```typescript
function redraw(allViews: boolean, inline: boolean) {
  // redraw logic.
}
const ALL_VIEWS = true, INLINE = true;
redraw(ALL_VIEWS, INLINE);


function executeCommand(recursive: boolean, executionMode: EXECUTION_MODES) {
  // executeCommand logic.
}
const RECURSIVE = true;
executeCommand(RECURSIVE, EXECUTION_MODES.ONE);
```
