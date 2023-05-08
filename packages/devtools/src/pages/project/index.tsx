import { useApp, useLocale, useProject } from "../../models/app.model";
import { limitString, tryOrAlert } from "../../common/utils";

import defaultLogo from "../../assets/logo-default.png";

import "./style.scss";
import { useState } from "react";
import { FolderOpen, Refresh } from "@icon-park/react";
import { ConfigEditor } from "./config-editor";

function Logo(props: { src: string | null }) {
  const [src, setSrc] = useState(props.src);

  return (
    <img
      style={{ height: "100%", width: "100%" }}
      alt="logo"
      src={src || defaultLogo}
      onError={() => {
        if (src !== defaultLogo) {
          setSrc(defaultLogo);
        }
      }}
    />
  );
}

function ProjectDetails() {
  const app = useApp();
  const locale = useLocale();
  const project = useProject();
  const { state } = project;

  return (
    <div className="project-detail">
      <section className="pd-base">
        <div className="pd-lf">
          <div className="pd-lf__info-container">
            <span>
              <Logo src={state.icon} />
            </span>
            <div className="info-container">
              <h3>{state.name}</h3>
              <p title={state.path}>
                {locale.getTranslation("PROJECT_PATH")} :{" "}
                {limitString(state.path, 50)}
              </p>
              <p>UUID: {state.uuid}</p>
            </div>
          </div>
          <div>
            <button
              className="btn"
              onClick={async () => {
                project.debug();
              }}
            >
              {locale.getTranslation("DEBUG")}
            </button>

            <button className="btn btn-primary" onClick={() => project.build()}>
              {locale.getTranslation("BUILD")}
            </button>
          </div>
        </div>
        <div className="pd-rt">
          <button
            className="btn btn-md btn-info"
            onClick={async () => {
              tryOrAlert(app, project.open());
            }}
          >
            <FolderOpen /> {locale.getTranslation("OPEN")}
          </button>

          <button
            className="btn btn-md btn-info"
            onClick={async () => {
              tryOrAlert(app, project.init());
            }}
          >
            <Refresh /> {locale.getTranslation("REFRESH")}
          </button>
        </div>
      </section>
      <section className="pd-more">
        <div className="fields-section">
          <h4>{locale.getTranslation("BASIC_INFO")}</h4>

          <div className="field-item">
            <span>{locale.getTranslation("ICON")}</span>
            <span>{state.config.icon}</span>
          </div>

          <div className="field-item">
            <span>{locale.getTranslation("CONFIG_FILE_PATH")}</span>
            <span>{state.configPath}</span>
          </div>
        </div>

        <div className="fields-section">
          <h4>{locale.getTranslation("DEBUG_INFO")}</h4>

          <div className="field-item">
            <span className="field-name">
              {locale.getTranslation("PROJECT_NAME")}
            </span>
            <span>
              {state.config.debug?.entry || locale.getTranslation("DEFAULT")}
            </span>
          </div>

          <div className="field-item">
            <span className="field-name">
              {locale.getTranslation("RESOURCE_PATH")}
            </span>
            <span>
              {state.config.debug?.resource || locale.getTranslation("DEFAULT")}
            </span>
          </div>
        </div>

        <div className="fields-section">
          <h4>{locale.getTranslation("BUILD_INFO")}</h4>
          <div className="field-item">
            <span className="field-name">
              {locale.getTranslation("RESOURCE_PATH")}
            </span>
            <span>
              {state.config.build?.resource || locale.getTranslation("DEFAULT")}
            </span>
          </div>
        </div>
      </section>
    </div>
  );
}

export function ProjectConfig() {
  return <ConfigEditor />;
}

export function Directory() {
  return <div>{/* <ImportLoader type="directory"></ImportLoader> */}</div>;
}

export function ProjectPage() {
  const app = useApp();
  const locale = useLocale();
  const project = useProject();
  const { state } = project;

  const [tab, setTab] = useState(0);

  if (!state) {
    return null;
  }

  return (
    <div className="project-page">
      <div className="directory">
        {/* <button onClick={() => {console.log('test xxx click')}}>xxxxxx</button> */}
        <Directory></Directory>
      </div>
      <div className="project-info">
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
              {locale.getTranslation("PROJECT_INFO")}
            </button>
            <button
              role="tab"
              aria-controls="config-tab"
              aria-selected={tab === 1}
              onClick={() => setTab(1)}
            >
              {locale.getTranslation("PROJECT_CONFIG")}
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
            <ProjectConfig />
          </article>
        </section>
      </div>
    </div>
  );
}
