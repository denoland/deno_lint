import { h } from "../deps.ts";
import type { DocumentProps } from "../deps.ts";

function Document(props: DocumentProps) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <title>deno_lint</title>
        <meta property="og:type" content="website" />
        <meta property="og:site_name" content="deno_lint" />
        <meta property="og:url" content="https://lint.deno.land" />
        <meta property="og:title" content="deno_lint docs" />
        <meta
          property="og:description"
          content="deno_lint documentation for every lint rule"
        />
        <meta property="og:image" content="https://deno.land/images/hashrock_simple.png" />
        <meta name="twitter:card" content="summary" />
        <link rel="stylesheet" href="/style/main.css" />
        <link
          rel="stylesheet"
          href="//cdn.jsdelivr.net/gh/highlightjs/cdn-release@10.2.1/build/styles/monokai-sublime.min.css"
        />
        <script
          src="//cdn.jsdelivr.net/gh/highlightjs/cdn-release@10.2.1/build/highlight.min.js"
        >
        </script>
      </head>
      <body class="bg-white">
        {props.children}
      </body>
      <script>
        hljs.initHighlightingOnLoad();
      </script>
    </html>
  );
}

export default Document;
