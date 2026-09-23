import {useModel} from "../../common/state";
import { useLocale, useProject } from "../../models/app.model";
import { useRef, useState } from "react";
import { ProjectDetails } from "./details";
import { ConfigEditor } from "./config-editor";

export function ProjectInfo() {
  const project = useProject();
  const locale = useLocale();
  const { state } = project;

  const editor = state.editor;
  useModel(editor);
  const {
    state: { isEdit },
  } = editor;

  const [tab, setTab] = useState(0);
  const tabRefs = [useRef<HTMLButtonElement>(null), useRef<HTMLButtonElement>(null)];
  const handleTabKeyDown = (event: React.KeyboardEvent<HTMLButtonElement>, next: number) => {
    if (event.key === "ArrowLeft" || event.key === "ArrowRight") {
      event.preventDefault();
      setTab(next);
      tabRefs[next].current?.focus();
    }
  };

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
          ref={tabRefs[0]}
          id="detail-tab-trigger"
          role="tab"
          aria-controls="detail-tab"
          aria-selected={tab === 0}
          tabIndex={tab === 0 ? 0 : -1}
          onKeyDown={(event) => handleTabKeyDown(event, 1)}
          onClick={() => setTab(0)}
        >
          {locale.t("PROJECT_INFO")}
        </button>
        <button
          ref={tabRefs[1]}
          id="config-tab-trigger"
          role="tab"
          aria-controls="config-tab"
          aria-selected={tab === 1}
          tabIndex={tab === 1 ? 0 : -1}
          onKeyDown={(event) => handleTabKeyDown(event, 0)}
          onClick={() => setTab(1)}
        >
          {isEdit ? (
            <span className="unsaved-tab-label">
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
        aria-labelledby="detail-tab-trigger"
        hidden={tab !== 0}
      >
        <ProjectDetails />
      </article>
      <article
        className="tabs-panel"
        role="tabpanel"
        id="config-tab"
        aria-labelledby="config-tab-trigger"
        hidden={tab !== 1}
      >
        <ConfigEditor />
      </article>
    </section>
  );
}
