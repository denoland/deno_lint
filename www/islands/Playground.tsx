import { useSignal } from "@preact/signals";
import MonacoEditor from "./MonacoEditor.tsx";
import Linter from "./Linter.tsx";
import { useMemo } from "preact/hooks";
import { IS_BROWSER } from "$fresh/runtime.ts";

export default function Playground() {
  const defaultSource = "let a = 42;";
  const source = useSignal(defaultSource);
  const isDarkMode = useMemo(() => {
    if (!IS_BROWSER) {
      return true;
    }

    const preferDark =
      window.matchMedia("(prefers-color-scheme: dark)").matches;
    return preferDark;
  }, []);

  return (
    <div class="flex flex-col gap-9 h-full">
      <MonacoEditor
        className="w-full h-2/3 border border-gray-300 dark:border-gray-700"
        defaultValue={defaultSource}
        language="typescript"
        source={source}
        isDarkMode={isDarkMode}
      />
      <div class="flex-1">
        <Linter source={source} />
      </div>
    </div>
  );
}
