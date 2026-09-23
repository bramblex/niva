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
      <div className="import-content">
        <div className="welcome-copy">
          <img src="logo.png" alt="" />
          <h1>{locale.t("UPLOAD_TIPS")}</h1>
        </div>
        <div className={classNames("file-uploader", { active: isHover })}>
          <div className="file-uploader__tips">
            <span className="drop-icon"><Plus size={28} /></span>
          </div>
          <div className="file-uploader__btns">
            <button
              className="btn btn-bg btn-primary"
              onClick={() => tryOrAlert(app, app.openWithPicker())}
            >
              {locale.t("OPEN_PROJECT")}
            </button>
            <button
              className="btn btn-bg"
              onClick={() => tryOrAlert(app, app.create())}
            >
              {locale.t("NEW_PROJECT")}
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}
