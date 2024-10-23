import { assertEquals } from "jsr:@std/assert@1.0.6";

const check = Deno.args.includes("--check");

const rulesOutput = await new Deno.Command("cargo", {
  args: ["run", "--features=docs", "--example", "dlint", "rules", "--json"],
}).output();
if (!rulesOutput.success) {
  throw new Error("Command failed: dlint rules --json");
}
const rulesOutputText = new TextDecoder().decode(rulesOutput.stdout);
const ruleEntries = JSON.parse(rulesOutputText);
const rules = new Set();
const tags = new Set();
for (const rule of ruleEntries) {
  rules.add(rule.code);
  for (const tag of rule.tags) {
    tags.add(tag);
  }
}
// These rules are implemented in CLI.
rules.add("no-sloppy-imports");
rules.add("no-slow-types");
const rulesSchema = {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "enum": [...rules].sort(),
};
const tagsSchema = {
  "$schema": "http://json-schema.org/draft-07/schema#",
  "enum": [...tags].sort(),
};

const rulesUrl = new URL("../schemas/rules.v1.json", import.meta.url);
const tagsUrl = new URL("../schemas/tags.v1.json", import.meta.url);
if (check) {
  const existingRulesSchema = JSON.parse(await Deno.readTextFile(rulesUrl));
  const existingTagsSchema = JSON.parse(await Deno.readTextFile(tagsUrl));
  assertEquals(existingRulesSchema, rulesSchema);
  assertEquals(existingTagsSchema, tagsSchema);
} else {
  await Deno.writeTextFile(rulesUrl, JSON.stringify(rulesSchema));
  await Deno.writeTextFile(tagsUrl, JSON.stringify(tagsSchema));
  const fmtOutput = await new Deno.Command("deno", {
    args: ["fmt", rulesUrl.toString(), tagsUrl.toString()],
  }).output();
  if (!fmtOutput.success) {
    throw new Error("Command failed: deno fmt");
  }
}
