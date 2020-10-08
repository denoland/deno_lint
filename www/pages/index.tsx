import { Fragment, h, MarkdownIt } from "../deps.ts";
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
  const { rule } = props;

  return <div style="padding: 12px; margin: 6px; border: black 2px solid;">
    <h2>{rule.code}</h2>
    <div dangerouslySetInnerHTML={{ __html: rule.docs }} />
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
