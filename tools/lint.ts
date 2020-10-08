const release = Deno.args.includes("--release");
console.log("clippy");

const mode = release ? ["--release"] : [];
const clippy = [
  "cargo",
  "clippy",
  "--all-targets",
  ...mode,
  "--locked",
  "--",
  "-D",
  "clippy::all",
];

let s1 = await Deno.run({
  cmd: clippy,
  stdin: "null",
}).status();

if (s1.code !== 0) {
  throw new Error(`Failed: ${clippy.join(" ")}`);
}

console.log("deno lint");

const dlint = `./target/${release ? "release" : "debug"}/examples/dlint`;
const s2 = await Deno.run({
  cmd: [
    dlint,
    "run",
    "benchmarks/benchmarks.ts",
  ],
  stdin: "null",
}).status();

if (s2.code !== 0) {
  throw new Error(`Failed: ${dlint} benchmarks/benchmarks.ts`);
}
