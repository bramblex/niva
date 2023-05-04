import { StateModel } from "@bramblex/state-model";
import { AppModel } from "./app.model";

interface HistoryModelState {
}

export class HistoryModel extends StateModel<HistoryModelState> {

	constructor(public readonly app: AppModel) {
		super({
		})
	}

	async init() {
	}

	async update() {
	}

	recently(): string | null {
		return null;
	}
}