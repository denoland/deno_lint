import jsonData from "../static/docs.json" assert { type: "json" };
import type { Handlers, PageProps } from "$fresh/server.ts";
import { renderMarkdown } from "../utils/render_markdown.ts";
import { ComponentChildren } from "preact";
import { Header } from "../components/Header.tsx";

interface RuleData {
  code: string;
  snippet: string;
  docs: string;
  tags: string[];
}

export const handler: Handlers<RuleData[]> = {
  GET(_req, ctx) {
    const rules = jsonData.map<RuleData>((rule) => ({
      code: rule.code,
      snippet: renderMarkdown(rule.docs.split("\n")[0]),
      docs: renderMarkdown(rule.docs.split("\n").slice(1).join("\n")),
      tags: rule.tags,
    }));
    return ctx.render(rules);
  },
};

export default function Home(props: PageProps<RuleData[]>) {
  const rules = props.data;

  const search = props.url.searchParams.get("q") ?? "";
  const allRules = props.url.searchParams.has("all");

  const searchResults = rules
    .filter((rule: RuleData) => {
      if (allRules) {
        return true;
      } else {
        return rule.tags.includes("recommended") || rule.tags.includes("fresh");
      }
    })
    .filter((rule: RuleData) => rule.code.includes(search));

  return (
    <div>
      <Header active="/" />

      <main class="my-8">
        <label for="search" class="sr-only">Search</label>
        <form id="search_form">
          <input
            type="text"
            name="q"
            class="w-full border h-10 border-gray-200 dark:border-gray-500 rounded rounded-r-none px-3 relative dark:bg-gray-800 "
            id="search"
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
  );
}

function Rule(props: { rule: RuleData }) {
  const { rule } = props;

  return (
    <section
      class="my-8 border-gray-200 dark:border-[#313235] border-2 rounded-lg overflow-hidden"
      id={rule.code}
    >
      <div class="p-3 border-b border-gray-200 flex justify-between flex-wrap gap-2 items-center bg-white dark:bg-[#0d1117] dark:border-[#313235]">
        <h1 class="text-xl font-bold">
          <a href={`#${rule.code}`} class="hover:underline">
            {rule.code}
          </a>
        </h1>
        <div>
          {rule.tags.includes("recommended") &&
            <Badge color="blue">Recommended</Badge>}
          {rule.tags.includes("fresh") &&
            <Badge color="green">Fresh</Badge>}
        </div>
      </div>
      <div class="relative bg-gray-50 dark:bg-[#192029] dark:text-white p-3">
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

function Badge(
  { children, color }: { children: ComponentChildren; color: string },
) {
  return (
    <span
      class={`inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium leading-4 bg-${color}-100 text-${color}-800`}
    >
      {children}
    </span>
  );
}
