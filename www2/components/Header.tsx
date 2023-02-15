import { JSX } from "preact";

export function Header() {
  return (
    <section class="my-8">
      <h1 class="text-3xl font-bold">deno_lint docs</h1>
      <div class="flex flex-wrap justify-between w-full">
        <div class="mt-2">
          <a
            href="/"
            class="hover:underline"
          >
            Rule overview
          </a>
          <a
            href="/ignoring-rules"
            class="hover:underline ml-4"
          >
            Ignoring rules
          </a>
        </div>
        <div class="mt-2">
          <a
            href="https://github.com/denoland/deno_lint"
            class="hover:underline"
          >
            View on GitHub
          </a>
        </div>
      </div>
    </section>
  );
}
