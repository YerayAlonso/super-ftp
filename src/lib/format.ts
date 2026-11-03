import type { FileEntry, Transfer } from './types';

const byteFormats = new Map<string, Intl.NumberFormat>();
const dateFormats = new Map<string, Intl.DateTimeFormat>();
const collators = new Map<string, Intl.Collator>();

export function formatBytes(bytes: number, locale = 'ca'): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), 4);
  const value = bytes / 1024 ** index;
  const cacheKey = `${locale}:${index === 0 ? 0 : 1}`;
  let formatter = byteFormats.get(cacheKey);
  if (!formatter) {
    formatter = new Intl.NumberFormat(locale, { maximumFractionDigits: index === 0 ? 0 : 1 });
    byteFormats.set(cacheKey, formatter);
  }
  return `${formatter.format(value)} ${['B', 'KB', 'MB', 'GB', 'TB'][index]}`;
}

export function formatDate(seconds: number | null, locale = 'ca'): string {
  if (seconds === null) return '—';
  let formatter = dateFormats.get(locale);
  if (!formatter) {
    formatter = new Intl.DateTimeFormat(locale, {
      day: '2-digit',
      month: 'short',
      year: 'numeric',
    });
    dateFormats.set(locale, formatter);
  }
  return formatter.format(new Date(seconds * 1000));
}

export function formatPermissions(mode: number | null): string {
  return mode === null ? '—' : (mode & 0o777).toString(8).padStart(3, '0');
}

export type SortKey = 'name' | 'size' | 'modified';

export function sortEntries(
  entries: FileEntry[],
  query: string,
  showHidden: boolean,
  sort: SortKey = 'name',
  ascending = true,
  locale = 'ca',
): FileEntry[] {
  const needle = query.trim().toLocaleLowerCase(locale);
  let collator = collators.get(locale);
  if (!collator) {
    collator = new Intl.Collator(locale, { numeric: true, sensitivity: 'base' });
    collators.set(locale, collator);
  }
  return entries
    .filter(
      (entry) =>
        (showHidden || !entry.hidden) &&
        (!needle || entry.name.toLocaleLowerCase(locale).includes(needle)),
    )
    .sort((a, b) => {
      if (a.kind === 'directory' && b.kind !== 'directory') return -1;
      if (b.kind === 'directory' && a.kind !== 'directory') return 1;
      const order =
        sort === 'name'
          ? collator.compare(a.name, b.name)
          : sort === 'size'
            ? a.size - b.size
            : (a.modified ?? 0) - (b.modified ?? 0);
      return ascending ? order : -order;
    });
}

export function isActive(transfer: Transfer): boolean {
  return ['queued', 'preparing', 'transferring', 'awaitingDecision'].includes(transfer.status);
}

export function progress(transfer: Transfer): number {
  if (transfer.status === 'completed') return 100;
  if (!transfer.totalBytes) return 0;
  return Math.min(100, Math.max(0, (transfer.bytesTransferred / transfer.totalBytes) * 100));
}

export function shortPath(path: string): string {
  return path.replace(/^\\\\\?\\/, '').replace(/\\/g, '/');
}
