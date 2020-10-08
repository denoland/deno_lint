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
    <>
      <h1>deno_lint</h1>
      {props.data.rules.map((rule) => <Rule rule={rule} />)}
    </>
  );
}

function Rule(props: { rule: Rule }) {
  const ref = useRef<HTMLDivElement>();
  const { rule } = props;

  useEffect(() => {
    ref.current.querySelectorAll("pre code").forEach((block) => {
      // @ts-expect-error because typescript is not aware of hljs
      hljs.highlightBlock(block);
    });
  }, [ref]);

  return <div style="padding: 12px; margin: 6px; border: black 2px solid;">
    <h2>{rule.code}</h2>
    {rule.docs.length > 0
      ? <div dangerouslySetInnerHTML={{ __html: rule.docs }} ref={ref} />
      : <div>(no docs provided)</div>}
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
