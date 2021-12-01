import { marked, Prism } from "../deps.ts";

export async function diskFetcher(path: string): Promise<string> {
  return await Deno.readTextFile(path);
}

export function renderMarkdown(markdown: string): string {
  const html = marked(markdown, {
    highlight(code, lang) {
      return Prism.highlight(
        code,
        Prism.languages.ts,
        lang,
      );
    },
  });
  return html;
}
