import {
  resolveMessage,
  translate,
  number,
  type DisplayMessage,
  type MessageKey,
} from './i18n/catalogue';
import { isLanguage, languages, type Language, type MessageParams } from './i18n/types';

export { languages, isLanguage } from './i18n/types';
export { msg, hasMessage } from './i18n/catalogue';
export type { DisplayMessage, MessageKey, Message } from './i18n/catalogue';
export type { Language } from './i18n/types';

export const LANGUAGE_STORAGE_KEY = 'superftp-language';

export function readLanguage(storage?: Pick<Storage, 'getItem'>): Language {
  try {
    const saved = (storage ?? globalThis.localStorage)?.getItem(LANGUAGE_STORAGE_KEY);
    if (isLanguage(saved)) return saved;
  } catch {
    /* A blocked/missing preference store must not stop the app. */
  }
  // An explicit saved choice always wins; otherwise use English.
  return 'en';
}

let currentLanguage = $state<Language>(readLanguage());

export const i18n = {
  get language() {
    return currentLanguage;
  },
  get locale() {
    return languages.find((language) => language.id === currentLanguage)!.locale;
  },
  setLanguage(value: unknown): boolean {
    if (!isLanguage(value)) return false;
    currentLanguage = value;
    try {
      globalThis.localStorage?.setItem(LANGUAGE_STORAGE_KEY, value);
    } catch {
      /* Optional persistence. */
    }
    return true;
  },
  t(key: MessageKey, params?: MessageParams) {
    return translate(currentLanguage, key, params);
  },
  text(value: DisplayMessage) {
    return resolveMessage(currentLanguage, value);
  },
  number(value: number) {
    return number(currentLanguage, value);
  },
  count(one: MessageKey, other: MessageKey, count: number) {
    return translate(currentLanguage, count === 1 ? one : other, { count });
  },
};
