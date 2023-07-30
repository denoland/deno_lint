import { Header } from "../components/Header.tsx";
import { CommonHead } from "../components/CommonHead.tsx";
import Playground from "../islands/Playground.tsx";

export default function PlaygroundPage() {
  return (
    <div class="py-6">
      <div class="mx-auto max-w-screen-md px-6 sm:px-6 md:px-8">
        <CommonHead />
        <Header />
        <Playground />
      </div>
    </div>
  );
}
