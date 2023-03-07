import { StateModel } from "@bramblex/state-model";
import { Project, ProjectData } from "./project";

interface LocalData {
  projects: ProjectData[];
}

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
    this.loadLocalData();
  }

  private async loadLocalData() {
    let localDataContent: LocalData | null = null;
    try {
      const { content } = await TauriLite.api.fs.read({
        path: this.localDataPath,
      });
      localDataContent = JSON.parse(content);
    } catch (err) {
      this.saveLocalData();
    }
    const { content } = await TauriLite.api.fs.read({
      path: this.localDataPath,
    });
    localDataContent = JSON.parse(content);

    this.setState({
      ...this.state,
      projects:
        localDataContent?.projects.map(
          (projectData) => new Project(projectData)
        ) || [],
    });
  }

  private async saveLocalData() {}
}

export const devtools = new DevtoolsModel();
