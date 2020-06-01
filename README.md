# Deno linter

**NOTE**
Very much work-in-progress

## Supported rules

- `banTsIgnore`
- `banUntaggedTodo`
- `constructorSuper`
- `defaultParamLast`
- `eqeqeq`
- `explicitFunctionReturnType`
- `forDirection`
- `getterReturn`
- `noAsyncPromiseExecutor`
- `noCaseDeclarations`
- `noCompareNegZero`
- `noCondAssign`
- `noDebugger`
- `noDeleteVar`
- `noDupeArgs`
- `noDupeKeys`
- `noDuplicateCase`
- `noEmpty`
- `noEmptyFunction`
- `noEmptyInterface`
- `noEval`
- `noExplicitAny`
- `noNewSymbol`
- `noPrototypeBuiltins`
- `noSetterReturn`
- `noSparseArray`
- `noThrowLiteral`
- `noUnsafeFinally`
- `noVar`
- `noWith`
- `requireYield`
- `singleVarDeclarator`
- `useIsNaN`
- `validTypeof`

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
$ ▶ target/debug/dlint ../deno/std/http/server.ts ../deno/std/http/file_server.ts
(noEmpty) Empty block statement
 --> ../deno/std/http/server.ts:93:14
   |
93 |       } catch {}
   |               ^^
   |
(noEmpty) Empty block statement
 --> ../deno/std/http/server.ts:111:44
    |
111 |     while ((await body.read(buf)) !== null) {}
    |                                             ^^
    |
(noEmpty) Empty block statement
 --> ../deno/std/http/server.ts:120:41
    |
120 |   constructor(public listener: Listener) {}
    |                                          ^^
    |
(banUntaggedTodo) TODO should be tagged with (@username) or (#issue)
 --> ../deno/std/http/file_server.ts:5:0
  |
5 | // TODO Stream responses instead of reading them into memory.
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
(banUntaggedTodo) TODO should be tagged with (@username) or (#issue)
 --> ../deno/std/http/file_server.ts:6:0
  |
6 | // TODO Add tests like these:
  | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
(banUntaggedTodo) TODO should be tagged with (@username) or (#issue)
 --> ../deno/std/http/file_server.ts:137:0
    |
137 | // TODO: simplify this after deno.stat and deno.readDir are fixed
    | ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
    |
(noEmpty) Empty block statement
 --> ../deno/std/http/file_server.ts:155:16
    |
155 |     } catch (e) {}
    |                 ^^
    |
Found 7 problems
```
