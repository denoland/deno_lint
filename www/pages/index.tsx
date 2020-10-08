import { Fragment, h, MarkdownIt, useEffect, useRef } from "../deps.ts";
import type { GetStaticData, PageProps } from "../deps.ts";

interface Data {
  rules: Array<Rule>;
}

interface Rule {
  code: string;
  docs: string;
}

function IndexPage(props: PageProps<Data>) {
  return (
    <div class="mx-auto max-w-screen-lg px-4 sm:px-6 md:px-8">
      <h1 class="text-3xl font-bold my-8">deno_lint docs</h1>
      <div>{props.data.rules.map((rule) => <Rule rule={rule} />)}</div>
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

  return <div class="p-4 rounded-lg shadow my-4 bg-white">
    <h2 class="text-xl font-medium mb-4">{rule.code}</h2>
    {rule.docs.length > 0
      ? <div
        dangerouslySetInnerHTML={{ __html: rule.docs }}
        ref={ref}
        class="prose"
      />
      : <div class="text-gray-900">(no docs provided)</div>}
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
