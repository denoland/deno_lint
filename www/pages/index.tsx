import {
  Fragment,
  h,
  MarkdownIt,
  useEffect,
  useRef,
  useState,
} from "../deps.ts";
import type { GetStaticData, PageProps } from "../deps.ts";

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
      <h1 class="text-3xl font-bold my-8">deno_lint docs</h1>
      <input
        class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline"
        type="text"
        placeholder="Search"
        value={search}
        onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
      />
      <div>
        {searchResults
          .map((rule) => <Rule rule={rule} />)}
      </div>
    </div>
  );
}

function Rule(props: { rule: Rule }) {
  const [expanded, setExpanded] = useState(false);

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

  return <div
    class="my-3 border-b border-t border-gray-200 sm:border sm:rounded-lg overflow-hidden"
  >
    <div
      class="p-3 border-b border-gray-200 flex justify-between items-center bg-white"
    >
      <h1 class="text-xl font-bold">{rule.code}</h1>
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
  </div>;
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
