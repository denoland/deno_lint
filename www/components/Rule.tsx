import { Badge } from "./Badge.tsx";

export interface RuleData {
  code: string;
  snippet: string;
  docs: string;
  tags: string[];
}

export interface RuleProps {
  rule: RuleData;
  open?: boolean;
}

export function Rule(props: RuleProps) {
  const { rule } = props;

  return (
    <section
      class="my-8 border-gray-200 dark:border-[#313235] border-2 rounded-lg overflow-hidden"
      id={rule.code}
    >
      <div class="p-3 border-b border-gray-200 flex justify-between flex-wrap gap-2 items-center bg-white dark:bg-[#0d1117] dark:border-[#313235]">
        <h1 class="text-xl font-bold">
          <a href={`/rules/${rule.code}`} class="hover:underline">
            {rule.code}
          </a>
        </h1>
        <div>
          {rule.tags.includes("recommended") &&
            <Badge color="blue">Recommended</Badge>}
          {rule.tags.includes("fresh") &&
            <Badge color="green">Fresh</Badge>}
        </div>
      </div>
      <div class="relative bg-gray-50 dark:bg-[#192029] dark:text-white p-3">
        {rule.docs.length > 0
          ? (
            <>
              <details open={props.open}>
                <summary>
                  <div
                    dangerouslySetInnerHTML={{ __html: rule.snippet }}
                    class="markdown-body inline-block"
                  />
                </summary>
                <div
                  dangerouslySetInnerHTML={{ __html: rule.docs }}
                  class="markdown-body mt-4"
                />
              </details>
            </>
          )
          : <div class="text-gray-500 italic">no docs available</div>}
      </div>
    </section>
  );
}
