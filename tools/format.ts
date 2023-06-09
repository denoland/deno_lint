#!/usr/bin/env -S deno run --allow-run
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
const check = Deno.args.includes("--check");
console.log("rustfmt");

const checkArgs = check ? ["--check"] : [];

const p1 = new Deno.Command("rustfmt", {
  args: [...checkArgs, "examples/dlint/main.rs"],
  stdin: "null",
}).spawn();

const result1 = await p1.status;

if (!result1.success) {
  throw new Error(
    `Failed: rustfmt ${check ? "--check" : ""}`,
  );
}

const p2 = new Deno.Command("rustfmt", {
  args: [...checkArgs, "src/lib.rs"],
  stdin: "null",
}).spawn();

const result2 = await p2.status;

if (!result2.success) {
  throw new Error(`Failed: rustfmt ${check ? "--check" : ""}`);
}

console.log("deno fmt");

const p3 = new Deno.Command("deno", {
  args: [
    "fmt",
    ...checkArgs,
    "tools",
    "benchmarks/benchmarks.ts",
    "www/pages",
    "docs/rules",
    "README.md",
  ],
  stdin: "null",
}).spawn();

const result3 = await p3.status;

if (!result3.success) {
  throw new Error(
    `Failed: deno fmt ${check ? "--check" : ""}`,
  );
}
