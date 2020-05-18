# Deno linter

**NOTE**
Very much work-in-progress

## Supported rules

- `banTsIgnore`
- `banUntaggedTodo`
- `eqeqeq`
- `explicitFunctionReturnType`
- `noAsyncPromiseExecutor`
- `noDebugger`
- `noDeleteVar`
- `noDupeArgs`
- `noDuplicateCase`
- `noEmptyFunction`
- `noEmptyInterface`
- `noEval`
- `noExplicitAny`
- `noSparseArray`
- `noVar`
- `singleVarDeclarator`
- `useIsNaN`

## Ignores

Only single line ignores are supported:

```ts
// deno-lint-ignore noExplicitAny
function foo(): any {
  // ...
}

// deno-lint-ignore noExplicitAny explicitFunctionReturnType
function bar(a: any) {
  // ...
}
```

## Example output

```shell
$ target/debug/dlint test.ts
error: `any` type is not allowed (noExplicitAny) at ./test.ts:6:14
error: `var` keyword is not allowed (noVar) at ./test.ts:12:0
error: Variables shouldn't be deleted (noDeleteVar) at ./test.ts:14:0
error: Multiple variable declarators are not allowed (singleVarDeclarator) at ./test.ts:17:0
error: Don't use `// @ts-ignore` (banTsIgnore) at ./test.ts:27:0
error: `debugger` statement is not allowed (noDebugger) at ./test.ts:30:4
error: `eval` call is not allowed (noEval) at ./test.ts:35:0
error: TODO should be tagged with (@username) or (#issue) (banUntaggedTodo) at ./test.ts:38:0
error: Missing return type on function (explicitFunctionReturnType) at ./test.ts:39:0
error: Empty interfaces are not allowed (noEmptyInterface) at ./test.ts:45:0
error: Use the isNaN function to compare with NaN (useIsNaN) at ./test.ts:48:0
error: switch(NaN)' can never match a case clause. Use Number.isNaN instead of the switch (useIsNaN) at ./test.ts:50:0
error: 'case NaN' can never match. Use Number.isNaN before the switch (useIsNaN) at ./test.ts:51:4
error: Sparse arrays are not allowed (noSparseArray) at ./test.ts:58:20
error: Duplicate values in `case` are not allowed (noDuplicateCase) at ./test.ts:67:9
error: Empty functions are not allowed (noEmptyFunction) at ./test.ts:75:0
error: Missing return type on function (explicitFunctionReturnType) at ./test.ts:78:0
error: Empty functions are not allowed (noEmptyFunction) at ./test.ts:78:0
error: Missing return type on function (explicitFunctionReturnType) at ./test.ts:81:12
error: Async promise executors are not allowed (noAsyncPromiseExecutor) at ./test.ts:81:0
error: Async promise executors are not allowed (noAsyncPromiseExecutor) at ./test.ts:85:0
error: Missing return type on function (explicitFunctionReturnType) at ./test.ts:90:0
error: Empty functions are not allowed (noEmptyFunction) at ./test.ts:90:0
error: Duplicate arguments not allowed (noDupeArgs) at ./test.ts:90:0
error: Duplicate arguments not allowed (noDupeArgs) at ./test.ts:94:18
Found 25 problems
```
