export const languages = [
  { id: 'ca', name: 'Català', locale: 'ca-ES' },
  { id: 'es', name: 'Español', locale: 'es-ES' },
  { id: 'en', name: 'English', locale: 'en-GB' },
  { id: 'fr', name: 'Français', locale: 'fr-FR' },
  { id: 'de', name: 'Deutsch', locale: 'de-DE' },
  { id: 'it', name: 'Italiano', locale: 'it-IT' },
  { id: 'pt', name: 'Português', locale: 'pt-PT' },
  { id: 'nl', name: 'Nederlands', locale: 'nl-NL' },
] as const;

export type Language = (typeof languages)[number]['id'];
export type Translations = Record<Exclude<Language, 'en'>, string>;
export type MessageParams = Record<string, string | number>;

export function isLanguage(value: unknown): value is Language {
  return typeof value === 'string' && languages.some((language) => language.id === value);
}
