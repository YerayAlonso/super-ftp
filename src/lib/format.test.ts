import { describe, expect, it } from 'vitest';
import {
  formatBytes,
  formatDate,
  formatPermissions,
  progress,
  shortPath,
  sortEntries,
} from './format';
import { asError } from './api';
import type { FileEntry, Transfer } from './types';

const file = (name: string, kind: FileEntry['kind'] = 'file', size = 0): FileEntry => ({
  name,
  path: `/${name}`,
  kind,
  size,
  modified: null,
  permissions: null,
  hidden: name.startsWith('.'),
});

describe('file listing', () => {
  it('keeps directories first and uses numeric name ordering', () => {
    expect(
      sortEntries([file('file10'), file('file2'), file('z', 'directory')], '', true).map(
        (f) => f.name,
      ),
    ).toEqual(['z', 'file2', 'file10']);
  });
  it('filters hidden files and ignores case without mutating the input', () => {
    const source = [file('.private'), file('PHOTO.jpg'), file('notes.txt')];
    expect(sortEntries(source, 'photo', false).map((f) => f.name)).toEqual(['PHOTO.jpg']);
    expect(source[0].name).toBe('.private');
  });
  it('retains directories first when descending by size', () => {
    expect(
      sortEntries(
        [file('a', 'file', 1), file('dir', 'directory'), file('b', 'file', 20)],
        '',
        true,
        'size',
        false,
      )[0].name,
    ).toBe('dir');
  });
});

describe('safe display formatting', () => {
  it('handles zero, invalid values, and binary units', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(NaN)).toBe('0 B');
    expect(formatBytes(1024)).toBe('1 KB');
    expect(formatDate(null)).toBe('—');
    expect(formatPermissions(0o100644)).toBe('644');
  });
  it('formats Windows extended paths for display only', () => {
    expect(shortPath('\\\\?\\C:\\Users\\test')).toBe('C:/Users/test');
  });
  it('clamps transfer progress and completes empty files', () => {
    expect(progress({ status: 'completed', totalBytes: 0 } as Transfer)).toBe(100);
    expect(
      progress({ status: 'transferring', totalBytes: 10, bytesTransferred: 20 } as Transfer),
    ).toBe(100);
  });
  it('preserves structured host-key errors', () => {
    const error = { code: 'HOST_KEY_CHANGED', message: 'Blocked', hostKey: { changed: true } };
    expect(asError(error).hostKey?.changed).toBe(true);
    expect(asError('offline').message).toBe('offline');
  });
});
