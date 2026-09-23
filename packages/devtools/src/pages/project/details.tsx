import "./style.scss";
import { useApp, useLocale, useProject } from "../../models/app.model";
import { Logo } from "./logo";
import { tryOrAlert } from "../../common/utils";
import { FolderOpen, Refresh } from "@icon-park/react";

export function ProjectDetails() {
  const app = useApp();
  const locale = useLocale();
  const project = useProject();
  const { state } = project;

  return (
    <div className="project-detail">
      <header className="project-hero">
        <span className="project-hero-logo"><Logo src={state.icon} /></span>
        <div className="project-hero-copy">
          <div className="project-hero-name">
            <h2>{state.name}</h2>
            {state.config.version && <span className="version-tag">v{state.config.version}</span>}
          </div>
          <p title={state.path}>{state.path}</p>
        </div>
      </header>

      <div className="project-actions">
        <div className="primary-actions">
          <button className="btn" onClick={() => tryOrAlert(app, project.debug())}>
            {locale.t("DEBUG")}
          </button>
          <button className="btn btn-primary" onClick={() => tryOrAlert(app, project.build())}>
            {locale.t("BUILD")}
          </button>
        </div>
        <div className="utility-actions">
          <button className="btn btn-info" onClick={() => tryOrAlert(app, project.open())}>
            <FolderOpen size={15} />{locale.t("OPEN")}
          </button>
          <button className="btn btn-info" onClick={() => tryOrAlert(app, project.refresh())}>
            <Refresh size={15} />{locale.t("REFRESH")}
          </button>
        </div>
      </div>

      <div className="project-facts">
        <section className="fact-section">
          <h3>{locale.t("BASIC_INFO")}</h3>
          <dl>
            <div><dt>UUID</dt><dd className="mono">{state.uuid}</dd></div>
            <div><dt>{locale.t("ICON")}</dt><dd>{state.config.icon || locale.t("NONE")}</dd></div>
            <div><dt>{locale.t("CONFIG_FILE_PATH")}</dt><dd title={state.configPath}>{state.configPath}</dd></div>
          </dl>
        </section>
        <section className="fact-section">
          <h3>{locale.t("DEBUG_INFO")}</h3>
          <dl>
            <div><dt>{locale.t("ENTRY")}</dt><dd>{state.config.debug?.entry || locale.t("NONE")}</dd></div>
            <div><dt>{locale.t("RESOURCE_PATH")}</dt><dd>{state.config.debug?.resource || locale.t("NONE")}</dd></div>
          </dl>
        </section>
        <section className="fact-section">
          <h3>{locale.t("BUILD_INFO")}</h3>
          <dl>
            <div><dt>{locale.t("RESOURCE_PATH")}</dt><dd>{state.config.build?.resource || locale.t("DEFAULT")}</dd></div>
          </dl>
        </section>
      </div>
    </div>
  );
}
