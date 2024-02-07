import jsonData from "../../static/docs.json" with { type: "json" };

import { Handlers, PageProps } from "$fresh/server.ts";
import { Rule, RuleData } from "../../components/Rule.tsx";
import { renderMarkdown } from "../../utils/render_markdown.ts";
import { Header } from "../../components/Header.tsx";

export const handler: Handlers<RuleData> = {
  GET(_req, ctx) {
    const { name } = ctx.params;
    const rules = jsonData.map<RuleData>((rule) => ({
      code: rule.code,
      snippet: renderMarkdown(rule.docs.split("\n")[0]),
      docs: renderMarkdown(rule.docs.split("\n").slice(1).join("\n")),
      tags: rule.tags,
    }));

    const rule = rules.find((rule) => rule.code === name);

    if (!rule) {
      return ctx.renderNotFound();
    }
    return ctx.render(rule);
  },
};

export default function RulePage(props: PageProps) {
  const rule: RuleData = props.data;

  return (
    <div>
      <Header active={`/rules/${rule.code}`} />
      <main>
        <Rule rule={rule} open />
      </main>
    </div>
  );
}
