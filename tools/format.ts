console.log("rustfmt");

const p1 = await Deno.run({
  cmd: ["rustfmt", "--check", "examples/dlint/main.rs"],
  stdin: "null",
}).status();

if (p1.code !== 0) {
  throw new Error("Failed: rustfmt --check examples/dlint/main.rs");
}

const p2 = await Deno.run({
  cmd: ["rustfmt", "--check", "src/lib.rs"],
  stdin: "null",
}).status();

if (p2.code !== 0) {
  throw new Error("Failed: rustfmt --check src/lib.rs");
}

console.log("deno fmt");

const p3 = await Deno.run({
  cmd: ["deno", "fmt", "--check", "benchmarks/benchmarks.ts"],
  stdin: "null",
}).status();

if (p3.code !== 0) {
  throw new Error("Failed: deno fmt --check benchmarks/benchmarks.ts");
}
