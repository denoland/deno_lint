/// <reference no-default-lib="true" />
/// <reference lib="dom" />
/// <reference lib="deno.ns" />
/// <reference lib="deno.unstable" />

import { start } from "https://raw.githubusercontent.com/lucacasonato/fresh/a929b1022ab541e937f94592ecce05e7f4ffdaef/server.ts";
import routes from "./routes.gen.ts";

start(routes);
