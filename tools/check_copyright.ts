#!/usr/bin/env -S deno run --allow-read
// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.

import { walk } from "https://deno.land/std@0.88.0/fs/walk.ts";
import { readLines } from "https://deno.land/std@0.88.0/io/mod.ts";

const skip = [
  /^target\//,
  /^benchmarks\//,
  /^www\//,
  /^.git\//,
  /^.idea\//,
];

const missing: string[] = [];

for await (const entry of walk(".", { skip })) {
  if (!entry.isFile) continue;
  if (!entry.path.endsWith(".rs")) continue;

  const f = await Deno.open(entry.path);
  for await (const line of readLines(f)) {
    const hasCopyright = line.startsWith("// Copyright ") &&
      line.endsWith(" the Deno authors. All rights reserved. MIT license.");

    if (!hasCopyright) {
      missing.push(entry.path);
    }

    // unnecessary to check for lines other than the first line
    break;
  }
}

if (missing.length > 0) {
  console.error(
    `Failed: the following files don't have the copyright:\n${
      missing.join("\n")
    }`,
  );
  Deno.exit(1);
}

console.log("Perfect! All Rust files in the repository have the copyright👌");
