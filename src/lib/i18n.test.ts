import { beforeEach, describe, expect, it, vi } from 'vitest';
import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { parse } from 'svelte/compiler';
import {
  i18n,
  isLanguage,
  languages,
  LANGUAGE_STORAGE_KEY,
  readLanguage,
  msg,
} from './i18n.svelte';
import { hasMessage, messageGroups, messages, translate } from './i18n/catalogue';
import { errorMessage } from './api';
import { formatBytes, formatDate } from './format';

beforeEach(() => {
  vi.unstubAllGlobals();
  i18n.setLanguage('ca');
});

describe('offline translation catalogue', () => {
  it('flags third-party notices when dependency lockfiles change', () => {
    const notices = readFileSync(resolve('src/lib/third-party-notices.txt'), 'utf8');
    const inventory = JSON.parse(readFileSync(resolve('third-party/rust-windows-inventory.json'), 'utf8'));
    for (const [label, file] of [['Cargo.lock', 'src-tauri/Cargo.lock'], ['package-lock.json', 'package-lock.json']]) {
      const digest = createHash('sha256').update(readFileSync(resolve(file), 'utf8').replace(/\r\n/g, '\n')).digest('hex');
      expect(notices).toContain(`${label} SHA-256: ${digest}`);
      if (label === 'Cargo.lock') expect(inventory.cargoLockSha256).toBe(digest);
    }
  });
  it('has eight explicit language choices and no duplicate source messages', () => {
    expect(languages.map((language) => language.id)).toEqual([
      'ca',
      'es',
      'en',
      'fr',
      'de',
      'it',
      'pt',
      'nl',
    ]);
    const keys = messageGroups.flatMap((group) => Object.keys(group));
    expect(new Set(keys).size).toBe(keys.length);
  });

  it('provides complete translations with exactly the same placeholders in every language', () => {
    const placeholders = (text: string) =>
      [...text.matchAll(/\{(\w+)\}/g)].map((match) => match[1]).sort();
    for (const [source, translations] of Object.entries(messages)) {
      for (const language of languages) {
        if (language.id === 'en') continue;
        const text = translations[language.id];
        expect(text.trim(), `${language.id}: ${source}`).not.toBe('');
        expect(placeholders(text), `${language.id}: ${source}`).toEqual(placeholders(source));
      }
    }
  });

  it('covers every app-defined static Rust error and parameterized template', () => {
    for (const name of readdirSync(resolve('src-tauri/src')).filter((name) =>
      name.endsWith('.rs'),
    )) {
      const source = readFileSync(resolve('src-tauri/src', name), 'utf8');
      for (const match of source.matchAll(
        /(?:AppError|Self)::(?:new|parameterized)\(\s*"[^"]+",\s*"((?:[^"\\]|\\.)*)"/g,
      )) {
        const message = JSON.parse(`"${match[1]}"`) as string;
        expect(hasMessage(message), `${name}: ${message}`).toBe(true);
      }
    }
  });

  it('keeps visible UI copy and accessibility labels out of hardcoded component text', () => {
    const files = [
      resolve('src/App.svelte'),
      ...readdirSync(resolve('src/components'))
        .filter((name) => name.endsWith('.svelte'))
        .map((name) => resolve('src/components', name)),
    ];
    const technical =
      /^(?:super|ftp|v[\d.]+|SFTP|SSH \/ SFTP|npm run desktop|Ctrl \/ ⌘ [A-Z]|[NK]|A[−+]|\/s|F5|sftp\.exemple\.cat|deploy|~\/\.ssh\/id_ed25519)$/;
    for (const file of files) {
      const seen = new WeakSet<object>();
      function visit(value: unknown, parentType = '') {
        if (Array.isArray(value)) {
          value.forEach((child) => visit(child, parentType));
          return;
        }
        if (!value || typeof value !== 'object' || seen.has(value)) return;
        seen.add(value);
        const node = value as Record<string, unknown>;
        if (node.type === 'Text' && parentType !== 'Attribute') {
          const text = String(node.data).trim().replace(/\s+/g, ' ');
          if (/[a-zÀ-ÿ]/i.test(text)) expect(technical.test(text), `${file}: ${text}`).toBe(true);
        }
        if (
          node.type === 'Attribute' &&
          ['aria-label', 'title', 'placeholder', 'eyebrow'].includes(String(node.name)) &&
          Array.isArray(node.value)
        ) {
          for (const part of node.value) {
            if (part.type !== 'Text') continue;
            const text = String(part.data).trim();
            if (/[a-zÀ-ÿ]/i.test(text))
              expect(technical.test(text), `${file}: ${node.name}="${text}"`).toBe(true);
          }
        }
        for (const child of Object.values(node)) visit(child, String(node.type ?? parentType));
      }
      visit(parse(readFileSync(file, 'utf8'), { modern: true }));
    }
  });
});

describe('language state and safe interpolation', () => {
  it('defaults to English and ignores invalid or unavailable stored settings', () => {
    expect(readLanguage({ getItem: () => null })).toBe('en');
    expect(readLanguage({ getItem: () => 'fr' })).toBe('fr');
    expect(readLanguage({ getItem: () => '__proto__' })).toBe('en');
    expect(
      readLanguage({
        getItem: () => {
          throw new Error('blocked');
        },
      }),
    ).toBe('en');
    expect(isLanguage('pt')).toBe(true);
    expect(i18n.setLanguage('invalid')).toBe(false);
  });

  it('persists explicit choices and still works when storage is blocked', () => {
    const setItem = vi.fn();
    vi.stubGlobal('localStorage', { setItem, getItem: () => 'en' });
    expect(i18n.setLanguage('de')).toBe(true);
    expect(setItem).toHaveBeenCalledWith(LANGUAGE_STORAGE_KEY, 'de');
    expect(i18n.t('New connection')).toBe('Neue Verbindung');
    vi.stubGlobal('localStorage', {
      setItem() {
        throw new Error('blocked');
      },
    });
    expect(i18n.setLanguage('it')).toBe(true);
    expect(i18n.t('New connection')).toBe('Nuova connessione');
  });

  it('resolves existing descriptors in the newly selected language without altering parameters', () => {
    const notice = msg('Securely connected to {name}.', { name: '<server> {name}' });
    i18n.setLanguage('en');
    expect(i18n.text(notice)).toBe('Securely connected to <server> {name}.');
    i18n.setLanguage('ca');
    expect(i18n.text(notice)).toBe('Connexió segura amb <server> {name}.');
    i18n.setLanguage('es');
    expect(i18n.text(notice)).toBe('Conexión segura con <server> {name}.');
    expect(hasMessage('__proto__')).toBe(false);
    expect(i18n.text('remote-library-detail')).toBe('remote-library-detail');
  });

  it('translates backend error metadata without guessing paths or parsing interpolated text', () => {
    const error = errorMessage({
      code: 'EXISTS',
      message: 'Readable fallback',
      translation: {
        key: '“{path}” already exists. It was not overwritten.',
        params: { path: '/home/test/{path}.txt' },
      },
    });
    i18n.setLanguage('en');
    expect(i18n.text(error)).toBe(
      '“/home/test/{path}.txt” already exists. It was not overwritten.',
    );
    i18n.setLanguage('fr');
    expect(i18n.text(error)).toContain('existe déjà');
    expect(
      errorMessage({
        code: 'EXISTS',
        message: 'fallback',
        translation: { key: '__proto__', params: {} },
      }),
    ).toBe('fallback');
  });

  it('uses singular/plural forms and localized dates, sizes and counts', () => {
    i18n.setLanguage('en');
    expect(i18n.count('{count} item', '{count} items', 1)).toBe('1 item');
    expect(i18n.count('{count} item', '{count} items', 1000)).toBe('1,000 items');
    expect(formatBytes(1536, 'en-GB')).toBe('1.5 KB');
    expect(formatBytes(1536, 'de-DE')).toBe('1,5 KB');
    expect(formatDate(1780000000, 'en-GB')).not.toBe(formatDate(1780000000, 'ca-ES'));
    for (const language of languages)
      expect(translate(language.id, '{count} items', { count: 0 })).not.toContain('{count}');
  });
});
