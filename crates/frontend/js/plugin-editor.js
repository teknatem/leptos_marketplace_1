import { basicSetup, EditorView } from "codemirror";
import { css } from "@codemirror/lang-css";
import { javascript } from "@codemirror/lang-javascript";
import { SQLite, sql } from "@codemirror/lang-sql";
import { oneDark } from "@codemirror/theme-one-dark";

function languageExtension(language) {
  switch (language) {
    case "css":
      return css();
    case "sql":
      return sql({ dialect: SQLite });
    default:
      return javascript({ jsx: false, typescript: false });
  }
}

window.PluginCodeEditor = Object.freeze({
  create(parent, language, value, onChange) {
    return new EditorView({
      parent,
      doc: value || "",
      extensions: [
        basicSetup,
        languageExtension(language),
        oneDark,
        EditorView.lineWrapping,
        EditorView.updateListener.of(update => {
          if (update.docChanged) {
            onChange(update.state.doc.toString());
          }
        })
      ]
    });
  },

  setValue(editor, value) {
    const next = value || "";
    const current = editor.state.doc.toString();
    if (current === next) return;
    editor.dispatch({
      changes: { from: 0, to: current.length, insert: next }
    });
  },

  destroy(editor) {
    editor.destroy();
  }
});
