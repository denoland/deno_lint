// Document https://fresh.deno.dev/docs/concepts/layouts

import { PageProps } from "$fresh/server.ts";
import { CommonHead } from "../components/CommonHead.tsx";
import { Footer } from "../components/Footer.tsx";

export default function Layout({ Component, state }: PageProps) {
  return (
    <div class="py-6">
      <div class="mx-auto max-w-screen-md px-6 sm:px-6 md:px-8">
        <CommonHead />
        <Component />
      </div>

      <Footer class="mt-8" />
    </div>
  );
}
