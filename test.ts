/** duplicateImportSource */
import { test1 } from "./test1.ts";
import { test2 } from "./test1.ts";

/** noExplicitAny */
function foo(): any {
    // nothing going on here
}

/** noVar */
var someVar = "someString";
/** noDeleteVar */
delete someVar;

/** singleVarDeclarator */
const a = "a", b = "b", c = "c";

/** noDuplicateKeys */
const obj = {
    a: 10,
    a: 20,
    b: 30,
};

/** noDebugger, banTsIgnore */
// @ts-ignore
function asdf(): number {
    console.log("asdf");
    debugger;
    return 1;
}

/** explicitFunctionReturnType, banUntaggedTodo */
// TODO:
function missingType() {
    console.log("asdf");

    return 1;
}
/** noEmptyInterface */
interface EmptyInterface {}