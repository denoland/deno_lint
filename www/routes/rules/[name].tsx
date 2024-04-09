import IconBrandGithub from "https://deno.land/x/tabler_icons_tsx@0.0.5/tsx/brand-github.tsx";

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
  const ruleSlug = rule.code.replaceAll("-", "_");
  const sourceLink =
    `https://github.com/denoland/deno_lint/blob/main/src/rules/${ruleSlug}.rs`;

  return (
    <div>
      <Header active={`/rules/${rule.code}`} />
      <div class="text-sm inline-flex gap-2">
        <a href="/">
          <span class="text-blue-500 hover:text-blue-600">All rules</span>
        </a>
        <span class="text-gray-500">
          /
        </span>
        <span class="text-gray-500">{rule.code}</span>
      </div>
      <main>
        <Rule rule={rule} open />
      </main>
      <footer>
        <a class="hover:underline" href={sourceLink}>
          <IconBrandGithub
            title="View rule source on GitHub"
            class="w-5 h-5 mr-1 inline"
          />
          Rule source
        </a>
      </footer>
    </div>
  );
}
