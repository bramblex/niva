import { StateModel } from "@bramblex/state-model";
import { AppModel } from "./app.model";
import { dataDirWith, tryOrAlert } from "../common/utils";
import { ProjectModel } from "./project.model";
import { fromThrowable, fromThrowableAsync } from "../common/result";
import { maxBy } from "lodash";

export interface HistoryItem {
  name: string;
  uuid: string;
  path: string;
  icon: string | null;
  lastAccessed: number;
}

interface HistoryModelState {
  history: HistoryItem[];
}

const { fs } = Niva.api;

export class HistoryModel extends StateModel<HistoryModelState> {
  private historyFilePath!: string;

  constructor(public readonly app: AppModel) {
    super({
      history: [],
    });
  }

  async init() {
    await tryOrAlert(
      this.app,
      fromThrowableAsync(async () => {
        this.historyFilePath = dataDirWith("history.json");
        await fs.createDirAll(dataDirWith());
        if (!(await fs.exists(this.historyFilePath))) {
          await fs.write(this.historyFilePath, '{"history": []}');
        }
        const content = JSON.parse(await fs.read(this.historyFilePath));
        this.setState({
          ...this.state,
          ...content,
        });
      })
    );

    this.onStateChange(async () => {
      await fs.write(this.historyFilePath, JSON.stringify(this.state));
    });
  }

  async record(project: ProjectModel) {
    const { history } = this.state;

    let matched = false;
    const newHistory = history.map((p) => {
      if (p.uuid === project.state.uuid || p.path === project.state.path) {
        matched = true;
        return {
          name: project.state.name,
          uuid: project.state.uuid,
          path: project.state.path,
          icon: project.state.icon,
          lastAccessed: Date.now(),
        };
      }
      return p;
    });

    if (!matched) {
      newHistory.unshift({
        name: project.state.name,
        uuid: project.state.uuid,
        path: project.state.path,
        icon: project.state.icon,
        lastAccessed: Date.now(),
      });
    }

    this.setState({
      ...this.state,
      history: newHistory,
    });
  }

  async remove(path?: string, uuid?: string) {
    this.setState({
      ...this.state,
      history: this.state.history.filter(
        (p) => p.path !== path && p.uuid !== uuid
      ),
    });
  }

  recently(): string | null {
    return maxBy(this.state.history, "lastAccessed")?.path || null;
  }
}
