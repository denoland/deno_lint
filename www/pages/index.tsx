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
  const [searchResults, setSearchResults] = useState([]);

  useEffect(() => {
    const results = props.data.rules
      .filter((rule) => rule.code.includes(search));
    setSearchResults(results);
  }, [search]);

  return (
    <div class="mx-auto max-w-screen-lg px-6 sm:px-6 md:px-8">
      <h1 class="text-3xl font-bold my-8">deno_lint docs</h1>
      <input class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline" type="text" placeholder="Search" value={search} onInput={e => setSearch(e.target.value)} />
      <div>{
        searchResults
          .map((rule) => <Rule rule={rule} />)
      }</div>
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

  return <div class="p-3 rounded-lg shadow my-3 bg-white">
    <h2 class="text-l font-medium">{rule.code}</h2>
    {rule.docs
      ? <>
        {expanded
          ? <button
            class="flex items-center text-gray-500 text-sm mt-2"
            onClick={() => setExpanded(false)}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              class="w-6 h-6 mr-2"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M5 15l7-7 7 7"
              />
            </svg>
            <span>Collapse</span>
          </button>
          : <button
            class="flex items-center text-gray-500 text-sm mt-2"
            onClick={() => setExpanded(true)}
          >
            <svg
              xmlns="http://www.w3.org/2000/svg"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
              class="w-6 h-6 mr-2"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={2}
                d="M19 9l-7 7-7-7"
              />
            </svg>
            <span>Expand</span>
          </button>}
        {expanded
          ? rule.docs.length > 0
            ? <div
              dangerouslySetInnerHTML={{ __html: rule.docs }}
              ref={ref}
              class="prose mt-2"
            />
            : <div class="text-gray-500 italic mt-2">no docs available</div>
          : undefined}
      </>
      : null}
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
