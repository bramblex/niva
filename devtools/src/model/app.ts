import { StateModel } from "@bramblex/state-model";

interface AppState {}

export class App extends StateModel<AppState> {
  constructor() {
    super({});
  }
}
