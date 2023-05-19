import "./style.scss";
import { useApp, useLocale, useProject } from "../../models/app.model";
import { Logo } from "./logo";
import { limitString, tryOrAlert } from "../../common/utils";
import { FolderOpen, Refresh } from "@icon-park/react";

export function ProjectDetails() {
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
                {locale.t("PROJECT_PATH")} :{" "}
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
              {locale.t("DEBUG")}
            </button>

            <button className="btn btn-primary" onClick={() => project.build()}>
              {locale.t("BUILD")}
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
            <FolderOpen /> {locale.t("OPEN")}
          </button>

          <button
            className="btn btn-md btn-info"
            onClick={async () => {
              tryOrAlert(app, project.refresh('refresh'));
            }}
          >
            <Refresh /> {locale.t("REFRESH")}
          </button>
        </div>
      </section>
      <section className="pd-more">
        <div className="fields-section">
          <h4>{locale.t("BASIC_INFO")}</h4>

          <div className="field-item">
            <span>{locale.t("ICON")}</span>
            <span>{state.config.icon || locale.t("NONE")}</span>
          </div>

          <div className="field-item">
            <span>{locale.t("CONFIG_FILE_PATH")}</span>
            <span>{state.configPath}</span>
          </div>
        </div>

        <div className="fields-section">
          <h4>{locale.t("DEBUG_INFO")}</h4>

          <div className="field-item">
            <span className="field-name">
              {locale.t("PROJECT_NAME")}
            </span>
            <span>
              {state.config.debug?.entry || locale.t("NONE")}
            </span>
          </div>

          <div className="field-item">
            <span className="field-name">
              {locale.t("RESOURCE_PATH")}
            </span>
            <span>
              {state.config.debug?.resource || locale.t("NONE")}
            </span>
          </div>
        </div>

        <div className="fields-section">
          <h4>{locale.t("BUILD_INFO")}</h4>
          <div className="field-item">
            <span className="field-name">
              {locale.t("RESOURCE_PATH")}
            </span>
            <span>
              {state.config.build?.resource || locale.t("DEFAULT")}
            </span>
          </div>
        </div>
      </section>
    </div>
  );
}
