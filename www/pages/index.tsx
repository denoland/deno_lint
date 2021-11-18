/** @jsx h */
/** @jsxFrag Fragment */
import { Fragment, h, Head, PageProps, tw, useData } from "../deps.ts";
import { Header } from "../components/Header.tsx";
import { diskFetcher, renderMarkdown } from "../components/utils.ts";

interface RuleData {
  code: string;
  snippet: string;
  docs: string;
  tags: string[];
}

function IndexPage(props: PageProps) {
  const jsonData: any[] = JSON.parse(
    useData("www/public/docs.json", diskFetcher),
  );

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
    <div class={tw`mx-auto max-w-screen-md px-6 sm:px-6 md:px-8`}>
      <Head>
        <link
          rel="stylesheet"
          href="https://cdn.jsdelivr.net/gh/lucacasonato/manual@www/www/static/markdown.css"
          crossOrigin="anonymous"
        />
      </Head>
      <Header />
      <main class={tw`my-8`}>
        <label for="search" class={tw`sr-only`}>Search</label>
        <form id="search_form">
          <input
            type="text"
            name="q"
            class={tw
              `w-full border h-10 border-gray-200 rounded rounded-r-none px-3 relative`}
            placeholder="Search"
            value={search}
          />
          <div class={tw`mt-2`}>
            <input
              type="checkbox"
              id="all_rules"
              name="all"
              checked={allRules}
            />
            <label htmlFor="all_rules" class={tw`ml-2`}>
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
        <div class={tw`mt-6 text-gray-600`}>
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
      class={tw`my-8 border-gray-200 border-2 rounded-lg overflow-hidden`}
      id={rule.code}
    >
      <div
        class={tw
          `p-3 border-b border-gray-200 flex justify-between flex-wrap gap-2 items-center bg-white`}
      >
        <h1 class={tw`text-xl font-bold`}>
          <a href={`#${rule.code}`} class={tw`hover:underline`}>
            {rule.code}
          </a>
        </h1>
        {rule.tags.includes("recommended") && (
          <span
            class={tw
              `inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium leading-4 bg-blue-100 text-blue-800`}
          >
            Recommended
          </span>
        )}
      </div>
      <div
        class={tw`relative bg-gray-50 dark:bg-[#0d1117] dark:text-white p-3`}
      >
        {rule.docs.length > 0
          ? (
            <>
              <details>
                <summary>
                  <div
                    dangerouslySetInnerHTML={{ __html: rule.snippet }}
                    class="markdown-body"
                  />
                </summary>
                <div
                  dangerouslySetInnerHTML={{ __html: rule.docs }}
                  class={"markdown-body " + tw`mt-4`}
                />
              </details>
            </>
          )
          : <div class={tw`text-gray-500 italic`}>no docs available</div>}
      </div>
    </section>
  );
}

export default IndexPage;
