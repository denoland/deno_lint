import { useSignal } from "@preact/signals";
import MonacoEditor from "./MonacoEditor.tsx";

export default function Playground() {
  const defaultSource = "let a = 42;";
  const source = useSignal(defaultSource);

  return (
    <div>
      <MonacoEditor
        className="w-full h-[48rem]"
        defaultValue={defaultSource}
        language="typescript"
        source={source}
      />
      <p class="text-white">{source.value}</p>
    </div>
  );
}
