import { useSignal } from "@preact/signals";
import MonacoEditor from "./MonacoEditor.tsx";
import Linter from "./Linter.tsx";
import { useEffect, useMemo } from "preact/hooks";
import { IS_BROWSER } from "$fresh/runtime.ts";

export default function Playground() {
  const defaultSource = "let a = 42;";
  const source = useSignal(defaultSource);
  const isDarkMode = useSignal(false);

  useEffect(() => {
    if (!IS_BROWSER) {
      return;
    }

    const preferDarkMode = globalThis.matchMedia(
      "(prefers-color-scheme: dark)",
    );
    isDarkMode.value = preferDarkMode.matches;

    const handler = () => {
      isDarkMode.value = preferDarkMode.matches;
    };

    preferDarkMode.addEventListener("change", handler);

    return () => {
      preferDarkMode.removeEventListener("change", handler);
    };
  }, []);

  return (
    <div class="flex w-full h-full md:px-9 md:gap-4">
      <MonacoEditor
        className="w-1/2 h-full border border-gray-300 dark:border-gray-700"
        defaultValue={defaultSource}
        language="typescript"
        source={source}
        isDarkMode={isDarkMode}
      />
      <div class="w-1/2 h-full">
        <Linter source={source} />
      </div>
    </div>
  );
}
