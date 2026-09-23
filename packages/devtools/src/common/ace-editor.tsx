import { useEffect, useRef } from "react";
import ace from "ace-builds";

import "ace-builds/src-noconflict/mode-json";
import "ace-builds/src-noconflict/theme-github";
import "ace-builds/src-noconflict/ext-language_tools";

import jsonWorkerUrl from "ace-builds/src-noconflict/worker-json?url";

ace.config.setModuleUrl("ace/mode/json_worker", jsonWorkerUrl as string);

interface AceEditorProps {
  mode?: string;
  theme?: string;
  name?: string;
  width?: string;
  height?: string;
  value?: string;
  onChange?: (value: string) => void;
  editorProps?: { $blockScrolling?: boolean };
}

/**
 * Minimal ace-builds wrapper (replaces the unmaintained react-ace).
 */
export function AceEditor(props: AceEditorProps) {
  const { mode, theme, value, onChange } = props;
  const containerRef = useRef<HTMLDivElement>(null);
  const editorRef = useRef<ace.Editor | null>(null);
  const onChangeRef = useRef(onChange);
  const syncingRef = useRef(false);
  onChangeRef.current = onChange;

  useEffect(() => {
    const container = containerRef.current;
    if (!container) {
      return;
    }
    const editor = ace.edit(container, {
      mode: mode ? `ace/mode/${mode}` : undefined,
      theme: theme ? `ace/theme/${theme}` : undefined,
    });
    if (props.editorProps?.$blockScrolling) {
      (editor as unknown as { $blockScrolling: boolean }).$blockScrolling = true;
    }
    editor.on("change", () => {
      if (!syncingRef.current) {
        onChangeRef.current?.(editor.getValue());
      }
    });
    editorRef.current = editor;
    return () => {
      editor.destroy();
      editorRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  useEffect(() => {
    const editor = editorRef.current;
    if (editor && value !== undefined && editor.getValue() !== value) {
      const position = editor.getCursorPosition();
      syncingRef.current = true;
      try {
        editor.setValue(value, -1);
        editor.moveCursorToPosition(position);
        editor.clearSelection();
      } finally {
        syncingRef.current = false;
      }
    }
  }, [value]);

  useEffect(() => {
    const editor = editorRef.current;
    if (editor && mode) {
      editor.session.setMode(`ace/mode/${mode}`);
    }
  }, [mode]);

  useEffect(() => {
    const editor = editorRef.current;
    if (editor && theme) {
      editor.setTheme(`ace/theme/${theme}`);
    }
  }, [theme]);

  return (
    <div
      id={props.name}
      ref={containerRef}
      style={{ width: props.width ?? "100%", height: props.height ?? "100%" }}
    />
  );
}
