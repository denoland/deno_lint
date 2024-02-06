import { Head } from "$fresh/runtime.ts";

export function CommonHead() {
  return (
    <Head>
      <link
        rel="stylesheet"
        href="https://cdn.jsdelivr.net/gh/lucacasonato/manual@df7ae27/www/static/markdown.css"
        crossOrigin="anonymous"
      />
      <link rel="stylesheet" href="extension.css" />
    </Head>
  );
}
