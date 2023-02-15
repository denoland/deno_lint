import Prism from "prism";
import marked from "marked";

import "https://esm.sh/prismjs@1.25.0/components/prism-javascript.js?no-check&pin=v57";
import "https://esm.sh/prismjs@1.25.0/components/prism-typescript.js?no-check&pin=v57";

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
