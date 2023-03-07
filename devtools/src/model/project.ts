import { StateModel } from "@bramblex/state-model";

export interface ProjectData {
}

interface ProjectState {}

export class Project extends StateModel<ProjectState> {
  constructor(projectData: ProjectData) {
    super({});
  }

	toProjectData(): ProjectData {
		return {};
	}
}
