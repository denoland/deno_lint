import { h, MarkdownIt, useEffect, useRef, useState } from "../deps.ts";
import type { GetStaticData, PageProps } from "../deps.ts";
import { Header } from "../components/Header.tsx";

interface Data {
  rules: Array<Rule>;
}

interface Rule {
  code: string;
  docs: string;
}

function IndexPage(props: PageProps<Data>) {
  const [search, setSearch] = useState("");
  const [searchResults, setSearchResults] = useState<Rule[]>([]);

  useEffect(() => {
    const results = props.data.rules
      .filter((rule) => rule.code.includes(search));
    setSearchResults(results);
  }, [search]);

  return (
    <div class="mx-auto max-w-screen-lg px-6 sm:px-6 md:px-8">
      <Header />
      <main class="my-8">
        <label for="search" class="sr-only">Search</label>
        <div class="flex flex-wrap items-stretch w-full relative">
          <input
            type="text"
            id="search"
            class="flex-shrink flex-grow leading-normal w-px flex-1 border h-10 border-grey-light rounded rounded-r-none px-3 relative"
            placeholder="Search"
            value={search}
            onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
          />
          <div class="flex -mr-px">
            <button
              class="flex items-center leading-normal bg-grey-lighter rounded rounded-l-none border border-l-0 border-grey-light px-3 whitespace-no-wrap text-grey-dark text-sm"
              onClick={() => setSearch("")}
            >
              Clear
            </button>
          </div>
        </div>
        <div class="mt-4">
          {searchResults
            .map((rule) => <Rule rule={rule} />)}
        </div>
      </main>
    </div>
  );
}

function Rule(props: { rule: Rule }) {
  const ref = useRef<HTMLDivElement | undefined>();
  const { rule } = props;

  useEffect(() => {
    if (ref.current) {
      ref.current.querySelectorAll("pre code").forEach((block) => {
        // @ts-expect-error because typescript is not aware of hljs
        hljs.highlightBlock(block);
      });
    }
  }, [ref]);

  return <section
    class="my-3 border-b border-t border-gray-200 border rounded-lg overflow-hidden"
    id={rule.code}
  >
    <div
      class="p-3 border-b border-gray-200 flex justify-between items-center bg-white"
    >
      <h1 class="text-xl font-bold">
        <a href={`/#${rule.code}`} class="hover:underline">
          {rule.code}
        </a>
      </h1>
    </div>
    <div class="relative bg-gray-50 p-3">
      {rule.docs.length > 0
        ? <div
          dangerouslySetInnerHTML={{ __html: rule.docs }}
          ref={ref}
          class="prose"
        />
        : <div class="text-gray-500 italic">no docs available</div>}
    </div>
  </section>;
}

export const getStaticData = async (): Promise<GetStaticData<Data>> => {
  const json = JSON.parse(await Deno.readTextFile("../docs.json"));

  const md = new MarkdownIt();

  const rules = json.map((rule: any) => ({
    code: rule.code,
    docs: md.render(rule.docs),
  }));

  return {
    data: {
      rules,
    },
  };
};

export default IndexPage;
