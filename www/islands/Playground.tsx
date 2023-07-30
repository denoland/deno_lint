import { useSignal } from "@preact/signals";
import MonacoEditor from "./MonacoEditor.tsx";
import Linter from "./Linter.tsx";

export default function Playground() {
  const defaultSource = "let a = 42;";
  const source = useSignal(defaultSource);

  return (
    <div class="flex flex-col gap-9 h-full">
      <MonacoEditor
        className="w-full h-2/3"
        defaultValue={defaultSource}
        language="typescript"
        source={source}
      />
      <div class="flex-1">
        <Linter source={source} />
      </div>
    </div>
  );
}
