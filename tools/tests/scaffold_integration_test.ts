import { exists } from "https://deno.land/std@0.106.0/fs/mod.ts";
import { assertEquals } from "https://deno.land/std@0.106.0/testing/asserts.ts";

Deno.test(
  "Check if the files created by tools/scaffold.ts pass `cargo check`",
  async () => {
    const name = "dummy-lint-rule-for-testing";
    const filename = name.replaceAll("-", "_");
    const rulesPath = "./src/rules.rs";

    // Preserve the original content of src/rules.rs
    const rulesRs = await Deno.readTextFile(rulesPath);

    try {
      console.log(`Run the scaffold script to create ${name} rule`);
      const p1 = new Deno.Command("deno", {
        args: [
          "run",
          "--allow-write=.",
          "--allow-read=.",
          "./tools/scaffold.ts",
          name,
        ],
      });
      const o1 = await p1.output();

      assertEquals(o1.code, 0);
      console.log("Scaffold succeeded");

      // Check if `cargo check` passes
      console.log("Run `cargo check`");
      const args = ["check", "--all-targets", "--all-features", "--locked"];
      if (Deno.env.get("GH_ACTIONS") === "1") {
        // do a release build on GitHub actions since the other
        // cargo builds are also release
        args.push("--release");
      }
      const p2 = new Deno.Command("cargo", {
        args,
      });
      const o2 = await p2.output();

      assertEquals(o2.code, 0);
      console.log("`cargo check` succeeded");
    } finally {
      console.log("Start cleanup...");
      console.log("Restoring src/rules.rs...");
      await Deno.writeTextFile(rulesPath, rulesRs);

      console.log(`Deleting src/rules/${filename}.rs...`);
      const rsPath = `./src/rules/${filename}.rs`;
      if (await exists(rsPath)) {
        await Deno.remove(rsPath);
      }

      console.log(`Deleting docs/rules/${filename}.md...`);
      const mdPath = `./docs/rules/${filename}.md`;
      if (await exists(mdPath)) {
        await Deno.remove(mdPath);
      }

      console.log("Cleanup finished");
    }
  },
);
