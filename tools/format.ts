#!/usr/bin/env -S deno run --allow-run
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
const check = Deno.args.includes("--check");
console.log("rustfmt");

const checkArgs = check ? ["--check"] : [];

const p1 = new Deno.Command("rustfmt", {
  args: [...checkArgs, "examples/dlint/main.rs"],
  stdin: "null",
});

const o1 = await p1.output();

if (o1.code !== 0) {
  throw new Error(
    `Failed: rustfmt ${check ? "--check" : ""} examples/dlint/main.rs`,
  );
}

const p2 = new Deno.Command("rustfmt", {
  args: [...checkArgs, "src/lib.rs"],
  stdin: "null",
});

const o2 = await p2.output();

if (o2.code !== 0) {
  throw new Error(`Failed: rustfmt ${check ? "--check" : ""} src/lib.rs`);
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
});

const o3 = await p3.output();

if (o3.code !== 0) {
  throw new Error(
    `Failed: deno fmt ${check ? "--check" : ""} benchmarks/benchmarks.ts`,
  );
}
