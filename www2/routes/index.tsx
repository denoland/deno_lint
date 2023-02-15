import { Head } from "$fresh/runtime.ts";
import jsonData from "../static/docs.json" assert { type: "json" };
import { Header } from "../components/Header.tsx";
import Prism from "prism";
import marked from "marked";
import { PageProps } from "$fresh/server.ts";

import "https://esm.sh/prismjs@1.25.0/components/prism-javascript.js?no-check&pin=v57";
import "https://esm.sh/prismjs@1.25.0/components/prism-typescript.js?no-check&pin=v57";

interface RuleData {
  code: string;
  snippet: string;
  docs: string;
  tags: string[];
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

export default function Home(props: PageProps) {
  const rules = jsonData.map<RuleData>((rule) => ({
    code: rule.code,
    snippet: renderMarkdown(rule.docs.split("\n")[0]),
    docs: renderMarkdown(rule.docs.split("\n").slice(1).join("\n")),
    tags: rule.tags,
  }));

  const search = props.url.searchParams.get("q") ?? "";
  const allRules = props.url.searchParams.has("all");

  const searchResults = rules
    .filter((rule: RuleData) => {
      if (allRules) {
        return true;
      } else {
        return rule.tags.includes("recommended");
      }
    })
    .filter((rule: RuleData) => rule.code.includes(search));

  return (
    <div class="dark:bg-[#0d1117] dark:text-white py-6 h-full h-screen">
      <div class="mx-auto max-w-screen-md px-6 sm:px-6 md:px-8">
        <Head>
          <link
            rel="stylesheet"
            href="https://cdn.jsdelivr.net/gh/lucacasonato/manual@df7ae27/www/static/markdown.css"
            crossOrigin="anonymous"
          />
          <link rel="stylesheet" href="extension.css" />
        </Head>
        <Header />
        <main class="my-8">
          <label for="search" class="sr-only">Search</label>
          <form id="search_form">
            <input
              type="text"
              name="q"
              class="w-full border h-10 border-gray-200 dark:border-gray-500 rounded rounded-r-none px-3 relative dark:bg-gray-800 "
              placeholder="Search"
              value={search}
            />
            <div class="mt-2">
              <input
                type="checkbox"
                id="all_rules"
                name="all"
                checked={allRules}
              />
              <label htmlFor="all_rules" class="ml-2">
                Show all rules
              </label>
            </div>
          </form>
          <script
            dangerouslySetInnerHTML={{
              __html:
                "document.getElementById('all_rules').oninput = () => document.getElementById('search_form').submit();",
            }}
          >
          </script>
          <div class="mt-6 text-gray-600 dark:text-gray-400">
            Showing {searchResults.length} out of {rules.length} rules
          </div>
          <div>
            {searchResults
              .map((rule: RuleData) => <Rule key={rule.code} rule={rule} />)}
          </div>
        </main>
      </div>
    </div>
  );
}

function Rule(props: { rule: RuleData }) {
  const { rule } = props;

  return (
    <section
      class="my-8 border-gray-200 dark:border-[#313235] border-2 rounded-lg overflow-hidden"
      id={rule.code}
    >
      <div
        class="p-3 border-b border-gray-200 flex justify-between flex-wrap gap-2 items-center bg-white dark:bg-[#0d1117] dark:border-[#313235]"
      >
        <h1 class="text-xl font-bold">
          <a href={`#${rule.code}`} class="hover:underline">
            {rule.code}
          </a>
        </h1>
        {rule.tags.includes("recommended") && (
          <span
            class="inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium leading-4 bg-blue-100 text-blue-800"
          >
            Recommended
          </span>
        )}
      </div>
      <div
        class="relative bg-gray-50 dark:bg-[#192029] dark:text-white p-3"
      >
        {rule.docs.length > 0
          ? (
            <>
              <details>
                <summary>
                  <div
                    dangerouslySetInnerHTML={{ __html: rule.snippet }}
                    class="markdown-body inline-block"
                  />
                </summary>
                <div
                  dangerouslySetInnerHTML={{ __html: rule.docs }}
                  class="markdown-body mt-4"
                />
              </details>
            </>
          )
          : <div class="text-gray-500 italic">no docs available</div>}
      </div>
    </section>
  );
}
