import classNames from "classnames";
import { useState, useEffect } from "react";
import "./style.scss";
import { Plus } from "@icon-park/react";
import { useApp, useLocale } from "../../models/app.model";
import { tryOrAlert } from "../../common/utils";

export function ImportPage() {
  const app = useApp();
  const locale = useLocale();

  const [isHover, setHover] = useState(false);

  useEffect(() => {
    const handleDropped = (_: string, { paths }: { paths: string[] }) => {
      setHover(false);
      const path = paths[0];
      if (path) {
        tryOrAlert(app, app.open(path));
      }
    };

    const handleHovered = (_: string, { paths }: { paths: string[] }) => paths.length > 0 ? setHover(true) : void 0;
    const handleCancelled = () => setHover(false);

    Niva.addEventListener("fileDrop.dropped", handleDropped);
    Niva.addEventListener("fileDrop.hovered", handleHovered);
    Niva.addEventListener("fileDrop.cancelled", handleCancelled);
    return () => {
      Niva.removeEventListener("fileDrop.dropped", handleDropped);
      Niva.removeEventListener("fileDrop.hovered", handleHovered);
      Niva.removeEventListener("fileDrop.cancelled", handleCancelled);
    };
  }, []);

  return (
    <div className="import-page">
      <div className={classNames("file-uploader", { active: isHover })}>
        <div className="file-uploader__tips">
          <Plus size={36} />
          {locale.t("UPLOAD_TIPS")}
        </div>
        <div className="file-uploader__btns">
          <button
            className="btn btn-bg btn-primary"
            onClick={() => tryOrAlert(app, app.openWithPicker())}
          >
            <i className="icon-sm icon-folder"></i>
            {locale.t("OPEN_PROJECT")}
          </button>

          <button
            className="btn btn-bg"
            style={{ marginLeft: "6px" }}
            onClick={() => tryOrAlert(app, app.create())}
          >
            <i className="icon-sm icon-plus-black"></i>
            {locale.t("NEW_PROJECT")}
          </button>
        </div>
      </div>
    </div>
  );
}
