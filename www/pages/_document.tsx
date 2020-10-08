import { h } from "../deps.ts";
import type { DocumentProps } from "../deps.ts";

function Document(props: DocumentProps) {
  return (
    <html lang="en">
      <head>
        <meta charSet="UTF-8" />
        <meta name="viewport" content="width=device-width, initial-scale=1.0" />
        <link rel="stylesheet" href="/style/main.css" />
        <link
          rel="stylesheet"
          href="//cdn.jsdelivr.net/gh/highlightjs/cdn-release@10.2.1/build/styles/atom-one-dark.min.css"
        />
        <script
          src="//cdn.jsdelivr.net/gh/highlightjs/cdn-release@10.2.1/build/highlight.min.js"
        >
        </script>
      </head>
      <body class="bg-gray-50">
        {props.children}
      </body>
      <script>
        hljs.initHighlightingOnLoad();
      </script>
    </html>
  );
}

export default Document;
