import { useState } from "react";
import { useApp, useHistory, useLocale } from "../../models/app.model";
import { Logo } from "./logo";
import classNames from "classnames";
import { tryOrAlert } from "../../common/utils";
import { HistoryItem } from "../../models/history.model";
import { FolderPlus, Plus } from "@icon-park/react";

function Highlighter({ text, highlight }: { text: string; highlight: string }) {
  const parts = highlight
    ? text.split(new RegExp(`(${highlight.toLowerCase()})`, "gi"))
    : [text];
  return (
    <>
      {parts.map((part, i) => (
        <span
          key={i}
          style={
            part.toLowerCase() === highlight.toLowerCase()
              // ? { color: "#0084FF" }
              ? { color: "#35dd8e" }
              : undefined
          }
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
            <i
              className="icon-sm icon-delete"
              style={{ cursor: "pointer" }}
              onClick={() => setKeyword("")}
            ></i>
          ) : (
            <i className="icon-sm icon-search"></i>
          )}
        </div>
        <div className="btn-containers">
          <div>
            <span
              className="text-btn"
              onClick={() => tryOrAlert(app, app.create())}
            >
              {/* <i className="icon-sm icon-plus-primary"></i> */}
              <Plus theme="outline" size="17"/>
              {locale.t("NEW_PROJECT")}
            </span>
          </div>
          <div>
            <span
              className="text-btn"
              onClick={() => tryOrAlert(app, app.openWithPicker())}
            >
              {/* <i className="icon-sm icon-folder-primary"></i> */}
              <FolderPlus theme="outline" size="17"/>
              {locale.t("OPEN_PROJECT")}
            </span>
          </div>
        </div>
      </div>
      <div className="history">
        <span
          className="text-btn clear-history"
          onClick={async () => {
            if (
              await modal.confirm(
                locale.t("TIPS"),
                locale.t("DELETE_CONFIRM")
              )
            ) {
              history.setState({ history: [] });
            }
          }}
        >
          {locale.t("CLEAR_HISTORY")}
          <i key={Math.random()} className="icon-sm icon-delete"></i>
        </span>
        {historyList.length > 0 ? (
          <div className="history-list">
            {historyList.map((item) => (
              <div
                className={classNames("history-item", {
                  active: item.uuid === project?.state.uuid,
                })}
                key={item.path}
                onClick={() => {
                  tryOrAlert(app, app.open(item.path));
                }}
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
                <i
                  className="icon-sm icon-delete"
                  onClick={async () => {
                    if (
                      await modal.confirm(
                        locale.t("TIPS"),
                        locale.t("DELETE_CONFIRM")
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
                ></i>
              </div>
            ))}
          </div>
        ) : null}
      </div>
    </div>
  );
}
