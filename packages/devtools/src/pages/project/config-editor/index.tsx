import { useEffect, useState } from "react";
// import { useTranslation } from 'react-i18next'
// import { DialogComponentProps } from "../modals";
// import { ProjectModel } from "./model";

import AceEditor from "react-ace";
import ace from "ace-builds";

import "ace-builds/src-noconflict/mode-json";
import "ace-builds/src-noconflict/theme-github";
import "ace-builds/src-noconflict/ext-language_tools";

// import { tryOrAlertAsync, withCtx, withCtxP } from '../common/utils';

import "./style.scss";
import { useApp, useLocale, useProject } from "../../../models/app.model";
import { useModel } from "@bramblex/state-model-react";
import { tryOrAlert, classNames } from "../../../common/utils";

// eslint-disable-next-line import/no-webpack-loader-syntax
const jsonWorkerUrl =
  // eslint-disable-next-line import/no-webpack-loader-syntax
  require("file-loader!ace-builds/src-noconflict/worker-json").default;
ace.config.setModuleUrl("ace/mode/json_worker", jsonWorkerUrl);

// interface OptionsEditorProps extends DialogComponentProps {
// 	project: ProjectModel;
// }

export function ConfigEditor() {
  const app = useApp();
  const locale = useLocale();
  const project = useProject();
  const editor = useModel(project.state.editor);

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "s" && (event.ctrlKey || event.metaKey)) {
      tryOrAlert(app, project.save());
    }
  };

  const saveBtnClassnames = classNames({
    'btn': true,
    'btn-md': true,
    'btn-primary': editor.state.isEdit,
    'btn-disabled': !editor.state.isEdit
  })

  return (
    <div className="options-editor" onKeyDown={handleKeyDown}>
      <div className="options-editor-body">
        <AceEditor
          mode="json"
          theme="github"
          name="options-editor"
          height="100%"
          width="100%"
          value={editor.state.content}
          onChange={(c) => editor.setContent(c)}
          editorProps={{
            $blockScrolling: true,
          }}
        />
      </div>

      <footer style={{ textAlign: "right" }}>
        <button
          className="btn btn-md"
          style={{ marginRight: "6px" }}
          onClick={() => {
            tryOrAlert(app, project.init());
          }}
        >
          {locale.t("RESET")}
        </button>
        <button className={saveBtnClassnames} disabled={!editor.state.isEdit} onClick={() => {
          tryOrAlert(app, project.save());
        }}>
          {locale.t("SAVE")}
        </button>
      </footer>
    </div>
  );
}
