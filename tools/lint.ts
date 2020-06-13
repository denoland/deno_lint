console.log("clippy");

await Deno.run({
  cmd: [
    "cargo",
    "clippy",
    "--all-targets",
    "--release",
    "--locked",
    "--",
    "-D",
    "clippy::all",
  ],
  stdin: "null",
  stdout: "null",
}).status();

console.log("deno lint");

await Deno.run({
  cmd: ["./target/release/examples/dlint", "benchmarks/benchmarks.ts"],
  stdin: "null",
  stdout: "null",
}).status();
