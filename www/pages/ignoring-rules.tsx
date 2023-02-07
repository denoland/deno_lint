/** @jsx h */
import { h, Head, tw, useData } from "../deps.ts";
import { Header } from "../components/Header.tsx";
import { diskFetcher, renderMarkdown } from "../components/utils.ts";

function IgnoringRulesPage() {
  const md = useData("www/public/ignoring-rules.md", diskFetcher);
  const html = renderMarkdown(md);

  return (
    <div class={tw`dark:bg-[#0d1117] dark:text-white py-6 h-screen`}>
      <div
        class={tw`mx-auto max-w-screen-md px-6 sm:px-6 md:px-8 `}
      >
        <Head>
          <link
            rel="stylesheet"
            href="https://cdn.jsdelivr.net/gh/lucacasonato/manual@df7ae27/www/static/markdown.css"
            crossOrigin="anonymous"
          />
        </Head>
        <Header />
        <main
          dangerouslySetInnerHTML={{ __html: html }}
          class={tw`markdown-body my-8`}
        />
      </div>
    </div>
  );
}

export default IgnoringRulesPage;
