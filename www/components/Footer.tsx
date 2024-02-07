import { JSX } from "preact";

export function Footer(props: JSX.HTMLAttributes<HTMLDivElement>) {
  return (
    <div {...props}>
      <div class="mx-auto max-w-screen-md border-gray-200 border-t-1">
        <div class="flex justify-center items-center gap-4 py-6">
          <a
            href="/"
            class="hover:underline"
          >
            Home
          </a>
          <a
            href="https://github.com/denoland/deno_lint"
            class="hover:underline"
          >
            GitHub
          </a>
        </div>
      </div>
    </div>
  );
}
