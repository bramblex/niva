import { StateModel } from "@bramblex/state-model";

interface HistoryModelState {
}

export class HistoryModel extends StateModel<HistoryModelState> {

	constructor() {
		super({})
	}
}