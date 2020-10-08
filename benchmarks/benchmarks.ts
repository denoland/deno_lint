import {
  bench,
  BenchmarkTimer,
  runBenchmarks,
} from "https://deno.land/std@0.67.0/testing/bench.ts";
import { expandGlobSync } from "https://deno.land/std@0.67.0/fs/expand_glob.ts";

const RUN_COUNT = 5;

const files = [
  ...expandGlobSync("**/*.ts", {
    root: "./benchmarks/oak",
  }),
].map((e) => e.path);

bench({
  name: "deno_lint",
  runs: RUN_COUNT,
  async func(b: BenchmarkTimer): Promise<void> {
    b.start();
    const proc = Deno.run({
      cmd: ["./target/release/examples/dlint", ...files],
      stdout: "null",
      stderr: "null",
    });

    // No assert on success, cause dlint returns exit
    // code 1 if there's any problem.
    await proc.status();
    b.stop();
  },
});

bench({
  name: "eslint",
  runs: RUN_COUNT,
  async func(b: BenchmarkTimer): Promise<void> {
    b.start();
    const proc = Deno.run({
      cmd: ["npm", "run", "eslint", ...files],
      cwd: Deno.build.os === "windows" ? ".\\benchmarks" : "./benchmarks",
      stdout: "null",
      stderr: "null",
    });
    const { success } = await proc.status();
    if (!success) {
      // await Deno.copy(proc.stdout!, Deno.stdout);
      // await Deno.copy(proc.stderr!, Deno.stderr);
      throw Error("Failed to run eslint");
    }
    b.stop();
  },
});

const data = await runBenchmarks({ silent: true });

console.log(JSON.stringify(data.results));
