// Copyright 2020-2021 the Deno authors. All rights reserved. MIT license.
import { doc } from "https://deno.land/x/deno_doc@v0.13.0/mod.ts";

const windowDoc = await doc(
  "https://raw.githubusercontent.com/denoland/deno/main/cli/dts/lib.dom.d.ts",
);
const workerDoc = await doc(
  "https://raw.githubusercontent.com/denoland/deno/main/cli/dts/lib.webworker.d.ts",
);

const windowItems = new Set(windowDoc.map((item) => item.name));
const workerItems = new Set(workerDoc.map((item) => item.name));

const intersection = new Set(
  [...windowItems].filter((x) => workerItems.has(x)),
);
intersection.add("Deno");

// window's `location` and worker's `location` are not the same
// https://github.com/denoland/deno_lint/pull/824#issuecomment-908820143
intersection.delete("location");

console.log(JSON.stringify([...intersection], null, 2));
