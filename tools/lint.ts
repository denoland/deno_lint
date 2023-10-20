#!/usr/bin/env -S deno run --allow-run
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
const release = Deno.args.includes("--release");
console.log("clippy");

const mode = release ? ["--release"] : [];
const clippy = [
  "clippy",
  "--all-targets",
  "--all-features",
  ...mode,
  "--locked",
  "--",
  "-D",
  "clippy::all",
];

const p1 = new Deno.Command("cargo", {
  args: clippy,
  stdin: "null",
});

const o1 = await p1.output();

if (o1.code !== 0) {
  throw new Error(`Failed: ${clippy.join(" ")}`);
}

console.log("deno lint");

const cargoTargetDir = Deno.env.get("CARGO_TARGET_DIR") || "./target";
const dlint = `${cargoTargetDir}/${
  release ? "release" : "debug"
}/examples/dlint`;
const p2 = new Deno.Command(dlint, {
  args: ["run", "benchmarks/benchmarks.ts"],
  stdin: "null",
});

const o2 = await p2.output();

if (o2.code !== 0) {
  throw new Error(`Failed: ${dlint} benchmarks/benchmarks.ts`);
}
