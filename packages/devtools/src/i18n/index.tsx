import { en_US } from "./en_US";
import { zh_CN } from "./zh_CN";

export type Translations = typeof en_US;
export type TranslateKey = keyof Translations;

const _resources = {
  en_US,
  zh_CN,
};

export type Locale = keyof typeof _resources;

export const resources: Record<Locale, Translations> = _resources;
