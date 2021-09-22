/** @jsx h */
import { h, MarkdownIt, tw, useData, useEffect, useRef } from "../deps.ts";
import { Header } from "../components/Header.tsx";

function IgnoringRulesPage() {
  const raw = useData("www/public/ignoring-rules.md", fetcher);
  // @ts-ignore missing types
  const md = new MarkdownIt();
  const html = md.render(raw);

  // TODO: figure out.
  const ref = useRef<HTMLDivElement>();

  useEffect(() => {
    if (ref.current) {
      ref.current.querySelectorAll("pre code").forEach((block) => {
        // @ts-expect-error because typescript is not aware of hljs
        hljs.highlightBlock(block);
      });
    }
  }, [ref]);

  return (
    <div class={tw`mx-auto max-w-screen-md px-6 sm:px-6 md:px-8`}>
      <Header />
      <main
        dangerouslySetInnerHTML={{ __html: html }}
        ref={ref}
        class={tw`prose my-8`}
      />
    </div>
  );
}

async function fetcher(path: string) {
  return await Deno.readTextFile(path);
}

export default IgnoringRulesPage;
