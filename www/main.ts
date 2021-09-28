/// <reference no-default-lib="true" />
/// <reference lib="dom" />
/// <reference lib="deno.ns" />
/// <reference lib="deno.unstable" />

import { start } from "./server_deps.ts";
import routes from "./routes.gen.ts";

start(routes);
