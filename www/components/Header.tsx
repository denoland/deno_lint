/** @jsx h */
import { h, tw } from "../deps.ts";

export function Header() {
  return (
    <section class={tw`my-8`}>
      <h1 class={tw`text-3xl font-bold`}>deno_lint docs</h1>
      <div class={tw`flex flex-wrap justify-between w-full`}>
        <div class={tw`mt-2`}>
          <a
            href="/"
            class={tw`hover:underline`}
          >
            Rule overview
          </a>
          <a
            href="/ignoring-rules"
            class={tw`hover:underline ml-4`}
          >
            Ignoring rules
          </a>
        </div>
        <div class={tw`mt-2`}>
          <a
            href="https://github.com/denoland/deno_lint"
            class={tw`hover:underline`}
          >
            View on GitHub
          </a>
        </div>
      </div>
    </section>
  );
}
