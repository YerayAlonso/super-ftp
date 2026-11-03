import { common } from './common';
import { workspace } from './workspace';
import { files } from './files';
import { connections } from './connections';
import { transfers } from './transfers';
import { about } from './about';
import { errors } from './errors';
import { context } from './context';
import { conflicts } from './conflicts';
import { languages, type Language, type MessageParams } from './types';

export const messageGroups = [
  common,
  workspace,
  files,
  connections,
  transfers,
  about,
  errors,
  context,
  conflicts,
];
export const messages = {
  ...common,
  ...workspace,
  ...files,
  ...connections,
  ...transfers,
  ...about,
  ...errors,
  ...context,
  ...conflicts,
};
export type MessageKey = keyof typeof messages;
export interface Message {
  key: MessageKey;
  params: MessageParams;
}
export type DisplayMessage = string | Message;

export function hasMessage(value: unknown): value is MessageKey {
  return typeof value === 'string' && Object.prototype.hasOwnProperty.call(messages, value);
}

export function msg(key: MessageKey, params: MessageParams = {}): Message {
  return { key, params };
}

const numberFormats = new Map<Language, Intl.NumberFormat>();
export function number(language: Language, value: number): string {
  let formatter = numberFormats.get(language);
  if (!formatter) {
    formatter = new Intl.NumberFormat(languages.find((item) => item.id === language)!.locale);
    numberFormats.set(language, formatter);
  }
  return formatter.format(value);
}

export function translate(language: Language, key: MessageKey, params: MessageParams = {}): string {
  const template = language === 'en' ? key : messages[key][language];
  return template.replace(/\{(\w+)\}/g, (token, name: string) => {
    if (!Object.prototype.hasOwnProperty.call(params, name)) return token;
    const value = params[name];
    return typeof value === 'number' ? number(language, value) : value;
  });
}

export function resolveMessage(language: Language, value: DisplayMessage): string {
  if (typeof value !== 'string') return translate(language, value.key, value.params);
  // Raw messages from OS/SSH/SFTP libraries stay intact. They are details, not
  // translated UI; server names, paths, and file names are never translated.
  return hasMessage(value) ? translate(language, value) : value;
}
