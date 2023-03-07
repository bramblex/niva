import { StateModel } from "@bramblex/state-model";

interface ProjectState {}

export class Project extends StateModel<ProjectState> {
  constructor() {
    super({});
  }
}
