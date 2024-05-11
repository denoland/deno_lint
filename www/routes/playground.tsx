import { Header } from "../components/Header.tsx";
import { CommonHead } from "../components/CommonHead.tsx";
import Playground from "../islands/Playground.tsx";
import type { LayoutConfig } from "$fresh/server.ts";

export const config: LayoutConfig = {
  skipInheritedLayouts: true,
};

export default function PlaygroundPage() {
  return (
    <div class="flex flex-col py-6 h-screen">
      <div class="flex flex-col mx-auto max-w-screen-md px-6 sm:px-6 md:px-8 w-full">
        <CommonHead />
        <Header active="/playground" />
      </div>
      <div class="flex-1">
        <Playground />
      </div>
    </div>
  );
}
