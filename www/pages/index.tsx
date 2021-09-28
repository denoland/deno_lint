/** @jsx h */
import {
  h,
  MarkdownIt,
  PageConfig,
  tw,
  useData,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "../deps.ts";
import { Header } from "../components/Header.tsx";

interface RuleData {
  code: string;
  snippet: string;
  docs: string;
  tags: string[];
}

function IndexPage() {
  const jsonData = JSON.parse(useData("www/public/docs.json", fetcher));
  // @ts-ignore missing types
  const md = new MarkdownIt();

  const rules = jsonData.map((rule: any) => ({
    code: rule.code,
    snippet: md.render(rule.docs.split("\n")[0]),
    docs: md.render(rule.docs),
    tags: rule.tags,
  }));

  const [search, setSearch] = useState("");
  const [filterRecommended, setFilterRecommended] = useState(true);

  const searchResults = useMemo(() => {
    return rules
      .filter((rule: RuleData) => {
        if (filterRecommended) {
          return rule.tags.includes("recommended");
        } else {
          return true;
        }
      })
      .filter((rule: RuleData) => rule.code.includes(search));
  }, [search, filterRecommended]);

  return (
    <div class={tw`mx-auto max-w-screen-md px-6 sm:px-6 md:px-8`}>
      <Header />
      <main class={tw`my-8`}>
        <label for="search" class={tw`sr-only`}>Search</label>
        <div class={tw`flex flex-wrap items-stretch w-full relative`}>
          <input
            type="text"
            id="search"
            class={tw
              `flex-shrink flex-grow leading-normal w-px flex-1 border h-10 border-grey-light rounded rounded-r-none px-3 relative`}
            placeholder="Search"
            value={search}
            onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
          />
          <div class={tw`flex -mr-px`}>
            <button
              class={tw
                `flex items-center leading-normal bg-grey-lighter rounded rounded-l-none border border-l-0 border-grey-light px-3 whitespace-no-wrap text-grey-dark text-sm`}
              onClick={() => setSearch("")}
            >
              Clear
            </button>
          </div>
        </div>
        <div class={tw`mt-2`}>
          <input
            type="checkbox"
            id="only_recommended"
            checked={filterRecommended}
            onInput={(e) => {
              setFilterRecommended(e.currentTarget.checked);
            }}
          />
          <label htmlFor="only_recommended" class={tw`ml-2`}>
            Show only recommended rules
          </label>
        </div>
        <div class={tw`mt-6 text-gray-600`}>
          Showing {searchResults.length} out of {rules.length} rules
        </div>
        <div>
          {searchResults
            .map((rule: RuleData) => <Rule rule={rule} />)}
        </div>
      </main>
    </div>
  );
}

function Rule(props: { rule: RuleData }) {
  const [extended, setExtended] = useState(false);

  const ref = useRef<HTMLDivElement>();
  const { rule } = props;

  useEffect(() => {
    if (ref.current) {
      ref.current.querySelectorAll("pre code").forEach((block) => {
        // @ts-expect-error because typescript is not aware of hljs
        hljs.highlightBlock(block);
      });
    }
  }, [ref, extended]);

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
          <a href={`/#${rule.code}`} class={tw`hover:underline`}>
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
      <div class={tw`relative bg-gray-50 p-3`}>
        {rule.docs.length > 0
          ? (
            extended
              ? (
                <div>
                  <div
                    dangerouslySetInnerHTML={{ __html: rule.docs }}
                    ref={ref}
                    class={tw`prose max-w-none`}
                  />
                  <a
                    class={tw
                      `mt-4 block cursor-pointer text-blue-600 hover:underline`}
                    onClick={() => setExtended(false)}
                  >
                    View Less
                  </a>
                </div>
              )
              : (
                <div>
                  <div
                    dangerouslySetInnerHTML={{ __html: rule.snippet }}
                    ref={ref}
                    class={tw`prose max-w-none`}
                  />
                  <a
                    class={tw
                      `mt-4 block cursor-pointer text-blue-600 hover:underline`}
                    onClick={() => setExtended(true)}
                  >
                    View More
                  </a>
                </div>
              )
          )
          : <div class={tw`text-gray-500 italic`}>no docs available</div>}
      </div>
    </section>
  );
}

async function fetcher(path: string) {
  return await Deno.readTextFile(path);
}

export const config: PageConfig = { runtimeJS: true };

export default IndexPage;
