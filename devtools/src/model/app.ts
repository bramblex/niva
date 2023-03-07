import { StateModel } from "@bramblex/state-model";
import { Project } from "./project";

interface DevtoolsState {
  projects: Project[];
}

class DevtoolsModel extends StateModel<DevtoolsState> {
  private localDataPath!: string;

  constructor() {
    super({
      projects: [],
    });
  }

  async init() {
    const { sep } = await TauriLite.api.os.sep();
    const { dataDir } = await TauriLite.api.process.tl_env();
    this.localDataPath = [dataDir, "projects.json"].join(sep);
  }

  private async loadLocalData() {}

  private async saveLocalData() {}
}

export const devtools = new DevtoolsModel();
