import jsonData from "../static/docs.json" with { type: "json" };
import type { Handlers, PageProps } from "$fresh/server.ts";
import { renderMarkdown } from "../utils/render_markdown.ts";
import { Header } from "../components/Header.tsx";
import { Rule, RuleData } from "../components/Rule.tsx";

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
