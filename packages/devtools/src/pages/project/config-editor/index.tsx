import { useMemo, useState } from "react";
import { AceEditor } from "../../../common/ace-editor";
import { useModel } from "../../../common/state";
import { tryOrAlert } from "../../../common/utils";
import { useApp, useLocale, useProject } from "../../../models/app.model";
import "./style.scss";

type ConfigObject = Record<string, any>;

export function ConfigEditor() {
  const app = useApp();
  const locale = useLocale();
  const project = useProject();
  const editor = project.state.editor;
  useModel(editor);

  const [view, setView] = useState<"fields" | "json">("fields");
  const config = useMemo<ConfigObject | null>(() => {
    try {
      const parsed = JSON.parse(editor.state.content);
      return parsed && typeof parsed === "object" && !Array.isArray(parsed) ? parsed : null;
    } catch {
      return null;
    }
  }, [editor.state.content]);

  const updateField = (section: string | null, key: string, value: string) => {
    if (!config) return;
    const next = JSON.parse(JSON.stringify(config)) as ConfigObject;
    if (section) {
      const currentSection = next[section];
      next[section] = {
        ...(currentSection && typeof currentSection === "object" && !Array.isArray(currentSection)
          ? currentSection
          : {}),
        [key]: value,
      };
    } else {
      next[key] = value;
    }
    editor.setContent(JSON.stringify(next, null, 2) + "\n");
  };

  const field = (section: string | null, key: string, label: string, placeholder = "") => (
    <label className="config-field" key={(section ?? "root") + "." + key}>
      <span>{label}</span>
      <input
        type="text"
        value={String(section ? config?.[section]?.[key] ?? "" : config?.[key] ?? "")}
        placeholder={placeholder}
        onChange={(event) => updateField(section, key, event.target.value)}
        disabled={!config}
      />
    </label>
  );

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (event.key.toLowerCase() === "s" && (event.ctrlKey || event.metaKey)) {
      event.preventDefault();
      tryOrAlert(app, project.save());
    }
  };

  return (
    <div className="options-editor" onKeyDown={handleKeyDown}>
      <div className="config-toolbar">
        <div className="config-toolbar-title">
          <h2>{locale.t("PROJECT_CONFIG")}</h2>
          <p>{locale.t("CONFIG_HELP")}</p>
        </div>
        <div className="view-switch" role="group" aria-label={locale.t("CONFIG_VIEW")}>
          <button type="button" aria-pressed={view === "fields"} onClick={() => setView("fields")}>
            {locale.t("CONFIG_FIELDS")}
          </button>
          <button type="button" aria-pressed={view === "json"} onClick={() => setView("json")}>
            JSON
          </button>
        </div>
      </div>

      {view === "fields" ? (
        <div className="config-form">
          {!config ? (
            <div className="config-parse-error" role="alert">
              <strong>{locale.t("CONFIG_FORMAT_ERROR")}</strong>
              <p>{locale.t("CONFIG_JSON_REPAIR")}</p>
              <button type="button" className="btn" onClick={() => setView("json")}>JSON</button>
            </div>
          ) : (
            <>
              <section>
                <h3>{locale.t("BASIC_INFO")}</h3>
                <div className="config-field-grid">
                  {field(null, "name", locale.t("PROJECT_NAME"))}
                  {field(null, "version", locale.t("VERSION"), "0.1.0")}
                </div>
              </section>
              <section>
                <h3>{locale.t("DEBUG_INFO")}</h3>
                <div className="config-field-grid">
                  {field("debug", "entry", locale.t("ENTRY"), "http://localhost:3000")}
                  {field("debug", "resource", locale.t("RESOURCE_PATH"), "public")}
                </div>
              </section>
              <section>
                <h3>{locale.t("BUILD_INFO")}</h3>
                <div className="config-field-grid">
                  {field("build", "resource", locale.t("RESOURCE_PATH"), "build")}
                  {field("window", "title", locale.t("WINDOW_TITLE"))}
                </div>
              </section>
            </>
          )}
        </div>
      ) : (
        <div className="options-editor-body">
          <AceEditor
            mode="json"
            theme="github"
            name="options-editor"
            height="100%"
            width="100%"
            value={editor.state.content}
            onChange={(content) => editor.setContent(content)}
            editorProps={{ $blockScrolling: true }}
          />
        </div>
      )}

      <footer>
        <span className="save-state" role="status">
          {editor.state.isEdit ? locale.t("UNSAVED_SHORT") : locale.t("SAVED_SHORT")}
        </span>
        <button
          type="button"
          className="btn btn-md"
          onClick={() => tryOrAlert(app, project.reset())}
        >
          {locale.t("RESET")}
        </button>
        <button
          type="button"
          className="btn btn-md btn-primary"
          disabled={!editor.state.isEdit}
          onClick={() => tryOrAlert(app, project.save())}
        >
          {locale.t("SAVE")}
        </button>
      </footer>
    </div>
  );
}
