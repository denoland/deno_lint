#!/usr/bin/env -S deno run --allow-run
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
const check = Deno.args.includes("--check");
console.log("rustfmt");

const checkArgs = check ? ["--check"] : [];

const p1 = await Deno.run({
  cmd: ["rustfmt", ...checkArgs, "examples/dlint/main.rs"],
  stdin: "null",
}).status();

if (p1.code !== 0) {
  throw new Error(
    `Failed: rustfmt ${check ? "--check" : ""} examples/dlint/main.rs`,
  );
}

const p2 = await Deno.run({
  cmd: ["rustfmt", ...checkArgs, "src/lib.rs"],
  stdin: "null",
}).status();

if (p2.code !== 0) {
  throw new Error(`Failed: rustfmt ${check ? "--check" : ""} src/lib.rs`);
}

console.log("deno fmt");

const p3 = await Deno.run({
  cmd: [
    "deno",
    "fmt",
    ...checkArgs,
    "tools",
    "benchmarks/benchmarks.ts",
    "www/pages",
    "docs/rules",
    "README.md",
    ".devcontainer",
  ],
  stdin: "null",
}).status();

if (p3.code !== 0) {
  throw new Error(
    `Failed: deno fmt ${check ? "--check" : ""} benchmarks/benchmarks.ts`,
  );
}
