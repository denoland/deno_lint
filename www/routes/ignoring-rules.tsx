import type { Handlers, PageProps } from "$fresh/server.ts";
import { renderMarkdown } from "../utils/render_markdown.ts";
import { fromFileUrl, join } from "https://deno.land/std@0.177.0/path/posix.ts";

export const handler: Handlers<string> = {
  async GET(_req, ctx) {
    const mdPath = join(
      fromFileUrl(import.meta.url),
      "../../static/ignoring-rules.md",
    );
    const md = await Deno.readTextFile(mdPath);
    const html = renderMarkdown(md);
    return ctx.render(html);
  },
};

export default function IgnoringRulesPage(props: PageProps<string>) {
  return (
    <main
      dangerouslySetInnerHTML={{ __html: props.data }}
      class="markdown-body my-8"
    />
  );
}
