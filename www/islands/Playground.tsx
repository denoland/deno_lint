import { useSignal } from "@preact/signals";
import MonacoEditor, {
  type SupportedLanguages,
  supportedLanguages,
} from "./MonacoEditor.tsx";
import Linter from "./Linter.tsx";
import { useEffect } from "preact/hooks";
import { IS_BROWSER } from "$fresh/runtime.ts";

export default function Playground() {
  const defaultSource = "let a = 42;";
  const source = useSignal(defaultSource);
  const language = useSignal<SupportedLanguages>("TypeScript");
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
    <div class="flex flex-col w-full h-full md:px-9 md:gap-4">
      <div class="flex">
        <select
          class="dark:bg-[#1e1e1e] border border-gray-300 text-gray-900 text-sm rounded-lg focus:ring-blue-500 focus:border-blue-500 block p-2.5 dark:border-gray-700 dark:placeholder-gray-400 dark:text-white dark:focus:ring-blue-500 dark:focus:border-blue-500"
          defaultValue="typescript"
          value={language.value}
          onChange={(e) => {
            if (e.target !== null) {
              const target = e.target as HTMLSelectElement;
              language.value = target.value as SupportedLanguages;
            }
          }}
        >
          {supportedLanguages.map((language) => (
            <option value={language}>{language}</option>
          ))}
        </select>
      </div>
      <div class="flex w-full h-full md:gap-4">
        <MonacoEditor
          className="w-1/2 h-full border border-gray-300 dark:border-gray-700"
          defaultValue={defaultSource}
          language={language}
          source={source}
          isDarkMode={isDarkMode}
        />
        <div class="w-1/2 h-full">
          <Linter source={source} language={language} />
        </div>
      </div>
    </div>
  );
}
