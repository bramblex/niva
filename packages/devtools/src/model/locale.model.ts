import { StateModel } from "@bramblex/state-model";
import { AppModel } from "./app.model";
import { importAll } from "../common/utils";
import { mapKeys } from "lodash";

const resources = mapKeys(
	importAll((require as any).context('../i18n', false, /\.json$/)),
	(value, key) => key.replace(/\.\//, '').replace(/\.json$/, '')
) as Record<string, Record<string, string>>;

interface LocaleModelState {
	current: string;
	translation: Record<string, string>;
}

export class LocaleModel extends StateModel<LocaleModelState> {

	constructor(readonly app: AppModel, readonly defaultLocale: string = 'en_US') {
		super({} as LocaleModelState);
		this.setLocale(defaultLocale);
	}

	setLocale(locale: string) {
		console.log(resources);
		const translation = {
			...resources[this.defaultLocale],
			...resources[locale],
		};
		this.setState({
			current: locale,
			translation
		});
	}

	getText(key: string) {
		return this.state.translation[key] || `[${key.toUpperCase()}]`;
	}
}