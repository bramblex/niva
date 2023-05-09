import { useApp, useLocale, useProject } from "../../models/app.model";
import { limitString, tryOrAlert } from "../../common/utils";

import "./style.scss";
import { useEffect, useState } from "react";
import { Plus } from "@icon-park/react";
import { ProjectList } from "./list";
import { ProjectInfo } from "./info";

function EmptyPage() {
  return (
    <div className="tabs">
      <div
        className="tabs-menu"
        onMouseDownCapture={() => Niva.api.window.dragWindow()}
      ></div>
    </div>
  );
}

export function ProjectPage() {
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

    const handleHovered = () => setHover(true);
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
    <div className="project-page">
      <div className="directory">
        <ProjectList />
      </div>
      <div className="project-info">
        {app.state.project ? <ProjectInfo /> : <EmptyPage />}
      </div>
      {isHover ? (
        <div className="project-page-uploader">
          <div className="project-page-uploader-content">
            <Plus size={48} />
            {locale.t("UPLOAD_TIPS")}
          </div>
        </div>
      ) : null}
    </div>
  );
}
