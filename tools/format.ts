console.log("rustfmt");

await Deno.run({
  cmd: ["rustfmt", "--check", "examples/dlint/main.rs"],
  stdin: "null",
  stdout: "null",
}).status();

await Deno.run({
  cmd: ["rustfmt", "--check", "src/lib.rs"],
  stdin: "null",
  stdout: "null",
}).status();

console.log("deno fmt");

await Deno.run({
  cmd: ["deno", "fmt", "--check", "benchmarks/benchmarks.ts"],
  stdin: "null",
  stdout: "null",
}).status();
