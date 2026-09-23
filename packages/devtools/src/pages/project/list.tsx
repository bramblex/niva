import { useState } from "react";
import { useApp, useHistory, useLocale } from "../../models/app.model";
import { Logo } from "./logo";
import classNames from "classnames";
import { tryOrAlert } from "../../common/utils";
import { FolderPlus, Plus } from "@icon-park/react";

function Highlighter({ text, highlight }: { text: string; highlight: string }) {
  const escapedHighlight = highlight.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const parts = highlight
    ? text.split(new RegExp(`(${escapedHighlight})`, "gi"))
    : [text];
  return (
    <>
      {parts.map((part, i) => (
        <span
          key={i}
          className={part.toLowerCase() === highlight.toLowerCase() ? "keyword-match" : undefined}
        >
          {part}
        </span>
      ))}
    </>
  );
}

export function ProjectList() {
  const app = useApp();
  const { project, modal } = app.state;

  const history = useHistory();
  const locale = useLocale();

  const [keyword, setKeyword] = useState("");

  const historyList = history.state.history.filter((p) =>
    p.name.toLowerCase().includes(keyword.toLowerCase())
  );

  return (
    <div className="file-uploader-dir">
      <div className="search-bar">
        <div className="search-input">
          <input
            placeholder={locale.t("SEARCH_PLACEHOLDER")}
            value={keyword}
            onChange={async (e) => setKeyword(e.target.value)}
          ></input>
          {keyword ? (
            <button
              type="button"
              className="icon-sm icon-delete"
              aria-label={locale.t("CLEAR_SEARCH")}
              onClick={() => setKeyword("")}
            ></button>
          ) : (
            <i className="icon-sm icon-search"></i>
          )}
        </div>
        <div className="btn-containers">
          <div>
            <button
              type="button"
              className="text-btn"
              onClick={() => tryOrAlert(app, app.create())}
            >
              {/* <i className="icon-sm icon-plus-primary"></i> */}
              <Plus theme="outline" size="17"/>
              {locale.t("NEW_PROJECT")}
            </button>
          </div>
          <div>
            <button
              type="button"
              className="text-btn"
              onClick={() => tryOrAlert(app, app.openWithPicker())}
            >
              {/* <i className="icon-sm icon-folder-primary"></i> */}
              <FolderPlus theme="outline" size="17"/>
              {locale.t("OPEN_PROJECT")}
            </button>
          </div>
        </div>
      </div>
      <div className="history">
        <div className="history-heading">
          <span>{locale.t("RECENT_PROJECTS")}</span>
          <button
          type="button"
          className="text-btn clear-history"
          onClick={async () => {
            if (
              await modal.confirm(
                locale.t("TIPS"),
                locale.t("CLEAR_HISTORY_CONFIRM")
              )
            ) {
              history.update({ history: [] });
            }
          }}
        >
          {locale.t("CLEAR_HISTORY")}
        </button>
        </div>
        {historyList.length > 0 ? (
          <div className="history-list">
            {historyList.map((item) => (
              <div className="history-item-container"
                key={item.path}
              >
                <button
                  type="button"
                  className={classNames("history-item", {
                    active: item.uuid === project?.state.uuid,
                  })}
                  onClick={() => tryOrAlert(app, app.open(item.path))}
                  aria-label={`${locale.t("OPEN_PROJECT")}: ${item.name}`}
                >
                  <div className="picon">
                    <Logo src={item.icon} />
                  </div>
                  <div className="pinfo">
                    <h4>
                      <Highlighter text={item.name} highlight={keyword} />
                    </h4>
                    <span>{item.path}</span>
                  </div>
                </button>
                <button
                  type="button"
                  className="icon-sm icon-delete"
                  aria-label={`${locale.t("REMOVE_HISTORY_ITEM")}: ${item.name}`}
                  onClick={async () => {
                    if (
                      await modal.confirm(
                        locale.t("TIPS"),
                        locale.t("REMOVE_HISTORY_CONFIRM")
                      )
                    ) {
                      if (
                        project?.state.path === item.path ||
                        project?.state.uuid === item.uuid
                      ) {
                        const promise = app.close();
                        await tryOrAlert(app, promise);
                        const result = await promise;
                        if (result.isOk()) {
                          history.remove(item.path, item.uuid);
                        }
                      } else {
                        history.remove(item.path, item.uuid);
                      }
                    }
                  }}
                ></button>
              </div>
            ))}
          </div>
        ) : keyword ? (
          <p className="history-no-match">{locale.t("NO_SEARCH_RESULTS")}</p>
        ) : null}
      </div>
    </div>
  );
}
