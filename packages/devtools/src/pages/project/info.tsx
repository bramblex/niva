import { useModel } from "@bramblex/state-model-react";
import { useLocale, useProject } from "../../models/app.model";
import { useState } from "react";
import { ProjectDetails } from "./details";
import { ConfigEditor } from "./config-editor";

export function ProjectInfo() {
  const project = useProject();
  const locale = useLocale();
  const { state } = project;

  const {
    state: { isEdit },
  } = useModel(state.editor);

  const [tab, setTab] = useState(0);

  return (
    <section className="tabs">
      <menu
        className="tabs-menu"
        role="tablist"
        aria-label="Project Tabs"
        onMouseDownCapture={(ev) => {
          const t = ev.target as HTMLElement;
          if (t.tagName !== "BUTTON") {
            Niva.api.window.dragWindow();
          }
        }}
      >
        <button
          role="tab"
          aria-controls="detail-tab"
          aria-selected={tab === 0}
          onClick={() => setTab(0)}
        >
          {locale.t("PROJECT_INFO")}
        </button>
        <button
          role="tab"
          aria-controls="config-tab"
          aria-selected={tab === 1}
          onClick={() => setTab(1)}
        >
          {isEdit ? (
            <span style={{ color: "#F44336", fontWeight: "bold" }}>
              {locale.t("PROJECT_CONFIG")}*
            </span>
          ) : (
            locale.t("PROJECT_CONFIG")
          )}
        </button>
      </menu>
      <article
        className="tabs-panel"
        role="tabpanel"
        id="detail-tab"
        hidden={tab !== 0}
      >
        <ProjectDetails />
      </article>
      <article
        className="tabs-panel"
        role="tabpanel"
        id="config-tab"
        hidden={tab !== 1}
      >
        <ConfigEditor />
      </article>
    </section>
  );
}
