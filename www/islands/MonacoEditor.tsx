import { useEffect, useRef } from "preact/hooks";
import loader from "https://esm.sh/v130/@monaco-editor/loader@1.3.3";
import type { Signal } from "@preact/signals";

type Props = {
  defaultValue?: string;
  language: string;
  source: Signal<string>;
  className?: string;
};

export default function MonacoEditor(props: Props) {
  const editorRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    if (editorRef.current === null) {
      return;
    }

    loader.init().then((monaco) => {
      const properties = {
        value: 'function hello() {\n\talert("Hello world!");\n}',
        language: "javascript",
      };
      monaco.editor.setTheme("vs-dark");
      const editor = monaco.editor.create(editorRef.current, {
        value: props.defaultValue,
        language: props.language,
        minimap: {
          enabled: false,
        },
      });
      editor.onDidChangeModelContent((_e) => {
        props.source.value = editor.getValue();
      });
    });
  }, []);

  return <div ref={editorRef} class={props.className}></div>;
}
