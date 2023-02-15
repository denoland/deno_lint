import { Head } from "$fresh/runtime.ts";
import { Header } from "../components/Header.tsx";
import { CommonHead } from "../components/CommonHead.tsx";
import type { Handlers, PageProps } from "$fresh/server.ts";
import { renderMarkdown } from "../utils/render_markdown.ts";

export const handler: Handlers<string> = {
  async GET(_req, ctx) {
    const md = await Deno.readTextFile("static/ignoring-rules.md");
    const html = renderMarkdown(md);
    return ctx.render(html);
  },
};

export default function IgnoringRulesPage(props: PageProps<string>) {
  return (
    <div class="py-6">
      <div
        class="mx-auto max-w-screen-md px-6 sm:px-6 md:px-8"
      >
        <CommonHead />
        <Header />
        <main
          dangerouslySetInnerHTML={{ __html: props.data }}
          class="markdown-body my-8"
        />
      </div>
    </div>
  );
}
