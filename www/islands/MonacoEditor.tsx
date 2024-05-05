import { useEffect, useRef } from "preact/hooks";
import loader from "https://esm.sh/v130/@monaco-editor/loader@1.3.3";
import { type Signal } from "@preact/signals";

type Props = {
  defaultValue?: string;
  language: string;
  source: Signal<string>;
  className?: string;
  fontSize?: number;
  isDarkMode: boolean;
};

export default function MonacoEditor(props: Props) {
  const editorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    loader.init().then((monaco) => {
      monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: true,
        noSyntaxValidation: false,
      });

      monaco.editor.setTheme(props.isDarkMode ? "vs-dark" : "vs-light");

      const editor = monaco.editor.create(editorRef.current!, {
        value: props.defaultValue,
        language: props.language,
        fontSize: props.fontSize ?? 16,
        minimap: {
          enabled: false,
        },
        automaticLayout: true,
      });
      editor.onDidChangeModelContent((_e) => {
        props.source.value = editor.getValue();
      });
    });
  }, []);

  return <div ref={editorRef} class={props.className}></div>;
}
