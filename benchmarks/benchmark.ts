import {
  BenchmarkTimer,
  bench,
  runBenchmarks,
} from "https://deno.land/std@0.54.0/testing/bench.ts";
import { expandGlobSync } from "https://deno.land/std@0.54.0/fs/expand_glob.ts";

const files = [
  ...expandGlobSync("**/*.ts", {
    root: "./benchmarks/oak",
  }),
].map((e) => e.path);

bench(async function dlint(b: BenchmarkTimer): Promise<void> {
  b.start();
  const proc = Deno.run({
    cmd: ["./target/release/dlint", ...files],
    stdout: "null",
    stderr: "null",
  });
  await proc.status();
  b.stop();
});

bench(async function eslint(b: BenchmarkTimer): Promise<void> {
  b.start();
  const proc = Deno.run({
    cmd: ["npx", "eslint", ...files],
    cwd: "./benchmarks",
    stdout: "null",
  });
  await proc.status();
  b.stop();
});

const r1 = await runBenchmarks({ silent: true });
const r2 = await runBenchmarks({ silent: true });
const r3 = await runBenchmarks({ silent: true });

const total = {
  results: {
    dlint:
      (r1.results[0].totalMs + r2.results[0].totalMs + r3.results[0].totalMs) /
      3,
    eslint:
      (r1.results[1].totalMs + r2.results[1].totalMs + r3.results[1].totalMs) /
      3,
  },
};
console.log(JSON.stringify(total));
