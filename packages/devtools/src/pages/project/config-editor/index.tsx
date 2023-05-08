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

// eslint-disable-next-line import/no-webpack-loader-syntax
const jsonWorkerUrl =
  // eslint-disable-next-line import/no-webpack-loader-syntax
  require("file-loader!ace-builds/src-noconflict/worker-json").default;
ace.config.setModuleUrl("ace/mode/json_worker", jsonWorkerUrl);

const { fs } = Niva.api;

// interface OptionsEditorProps extends DialogComponentProps {
// 	project: ProjectModel;
// }

export function ConfigEditor() {
  const locale = useLocale();
  const project = useProject();

  const editor = useModel(project.state.editor);
  const { content, isEdit } = editor.state;

  // const project = useProject();
  // const { t } = useTranslation()
  // const [value, setValue] = useState("");

  // useEffect(() => {
  // 	tryOrAlertAsync(async () => {
  // 		const configPath = project.state!.configPath;
  // 		const content = await withCtxP(fs.read(configPath), t('failreadcfg'));
  // 		setValue(content as string);
  // 	}).catch(close);
  // }, []);

  // const saveDoc = () => {
  // 	tryOrAlertAsync(async () => {
  // 		withCtx(() => JSON.parse(value), t('errorformat'));
  // 		await withCtxP(fs.write(project.state!.configPath, value), t('failsave'));
  // 		close();
  // 		project.init(project.state!.path);
  // 	})
  // }

  const handleKeyDown = (event: React.KeyboardEvent) => {
    if (event.key === "s" && (event.ctrlKey || event.metaKey)) {
      project.save();
    }
  };

  return (
    <div className="options-editor" onKeyDown={handleKeyDown}>
      <div className="options-editor-body">
        <AceEditor
          mode="json"
          theme="github"
          name="options-editor"
          height="100%"
          width="100%"
          value={content}
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
            // setValue(JSON.stringify({ name: project.state!.name, uuid: project.state?.config?.uuid }))
          }}
        >
          {locale.getTranslation("RESET")}
        </button>
        <button className="btn btn-md btn-primary">
          {locale.getTranslation("SAVE")}
        </button>
      </footer>
    </div>
  );
}
