import { useEffect, useRef } from "preact/hooks";
import { type Signal, useSignal } from "@preact/signals";
import loader, { type Monaco } from "npm:@monaco-editor/loader@1.3.3";

export const supportedLanguages = [
  "TypeScript",
  "JavaScript",
  "TSX",
  "JSX",
] as const;

export type SupportedLanguages = typeof supportedLanguages[number];

type Props = {
  defaultValue?: string;
  language: Signal<SupportedLanguages>;
  source: Signal<string>;
  className?: string;
  fontSize?: number;
  isDarkMode: Signal<boolean>;
};

export default function MonacoEditor(props: Props) {
  const editorRef = useRef<HTMLDivElement>(null);
  const monacoEditor = useSignal<Monaco | null>(null);

  useEffect(() => {
    loader.init().then((monaco) => {
      monacoEditor.value = monaco;

      monaco.languages.typescript.typescriptDefaults.setDiagnosticsOptions({
        noSemanticValidation: true,
        noSyntaxValidation: false,
      });

      monaco.languages.typescript.typescriptDefaults.setCompilerOptions({
        jsx: props.language.value === "TSX" || props.language.value === "JSX"
          ? monaco.languages.typescript.JsxEmit.React
          : monaco.languages.typescript.JsxEmit.None,
      });

      monaco.editor.setTheme(props.isDarkMode.value ? "vs-dark" : "vs-light");

      const editor = monaco.editor.create(editorRef.current!, {
        value: props.defaultValue,
        language: "typescript",
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

  useEffect(() => {
    if (monacoEditor.value) {
      monacoEditor.value.editor.setTheme(
        props.isDarkMode.value ? "vs-dark" : "vs-light",
      );
    }
  }, [props.isDarkMode.value]);

  useEffect(() => {
    if (monacoEditor.value) {
      monacoEditor.value.languages.typescript.typescriptDefaults
        .setCompilerOptions(
          {
            jsx:
              props.language.value === "TSX" || props.language.value === "JSX"
                ? monacoEditor.value.languages.typescript.JsxEmit.React
                : monacoEditor.value.languages.typescript.JsxEmit.None,
          },
        );
    }
  }, [props.language.value]);

  return <div ref={editorRef} class={props.className}></div>;
}
