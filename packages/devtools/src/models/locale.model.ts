import { StateModel } from "@bramblex/state-model";
import { AppModel } from "./app.model";
import { Locale, TranslateKey, Translations, resources } from "../i18n";

interface LocaleModelState {
  current: Locale;
  translations: Translations;
}

export class LocaleModel extends StateModel<LocaleModelState> {
  constructor(
    readonly app: AppModel,
    readonly defaultLocale: Locale = "en_US"
  ) {
    super({} as LocaleModelState);
    this.setLocale(defaultLocale);
  }

  async init() {
    const locale: string = await Niva.api.os.locale();
    if (locale.endsWith("CN")) {
      this.setLocale("zh_CN");
    }
  }

  setLocale(locale: Locale) {
    const translation = {
      ...resources[this.defaultLocale],
      ...resources[locale],
    };
    this.setState({
      current: locale,
      translations: translation,
    });
  }

  t(key: TranslateKey, option?: { [key: string]: string }) {
    if (option) {
      const translate = this.state.translations[key];

      if (!translate) {
        return `[${key.toUpperCase()}]`
      };

      let result: string = translate;
      Object.entries(option).forEach(([_key, _value]) => {
        result = result.replaceAll(`{{${_key}}}`, _value);
        console.log('###res', _key, _value, result)
      });
      return result
    } else {
      return this.state.translations[key] || `[${key.toUpperCase()}]`;
    }
  }
}
