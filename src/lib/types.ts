import type { DisplayMessage } from './i18n/catalogue';

export type AuthMethod = 'password' | 'keyFile' | 'agent';
export interface Connection {
  id: string;
  name: string;
  host: string;
  port: number;
  username: string;
  authMethod: AuthMethod;
  keyPath: string | null;
  rememberSecret: boolean;
  remotePath: string;
  localPath: string | null;
  color: string;
}

export interface HostKeyChallenge {
  id: string;
  host: string;
  port: number;
  algorithm: string;
  fingerprint: string;
  previousFingerprint: string | null;
  changed: boolean;
}

export interface AppError {
  code: string;
  message: string;
  hostKey?: HostKeyChallenge;
  translation?: { key: string; params: Record<string, string | number> };
}

export type EntryKind = 'directory' | 'file' | 'symlink' | 'other';
export interface FileEntry {
  name: string;
  path: string;
  kind: EntryKind;
  size: number;
  modified: number | null;
  permissions: number | null;
  hidden: boolean;
}

export interface DirectoryListing {
  path: string;
  parent: string | null;
  entries: FileEntry[];
}

export interface SessionInfo {
  id: string;
  connectionId: string;
  homePath: string;
  fingerprint: string;
}

export type Direction = 'upload' | 'download';
export type TransferStatus =
  'queued' | 'preparing' | 'transferring' | 'awaitingDecision' | 'completed' | 'failed' | 'cancelled';
export interface TransferConflict {
  id: string; source: string; destination: string; name: string; direction: Direction;
  sourceSize: number; destinationSize: number | null; destinationModified: number | null;
}
export type ConflictDecision = { action: 'overwrite' | 'skip' | 'cancel' } | { action: 'rename'; name: string };
export interface Transfer {
  id: string;
  sessionId: string;
  name: string;
  direction: Direction;
  source: string;
  destination: string;
  status: TransferStatus;
  bytesTransferred: number;
  totalBytes: number;
  filesTotal: number;
  filesDone: number;
  skipped: number;
  bytesPerSecond: number;
  error: string | null;
  errorTranslation?: AppError['translation'] | null;
  conflict?: TransferConflict | null;
}

export interface PaneState {
  listing: DirectoryListing | null;
  loading: boolean;
  error: DisplayMessage | null;
  selected: FileEntry[];
  history: string[];
  historyIndex: number;
  request: number;
}

export interface SessionTab {
  id: string;
  connection: Connection;
  info: SessionInfo;
  local: PaneState;
  remote: PaneState;
}

export const COLORS = ['#b5e878', '#7cb7ff', '#c4a3ff', '#ffbc7c', '#ff8ca1', '#7cddd1'];

export function createPane(): PaneState {
  return {
    listing: null,
    loading: false,
    error: null,
    selected: [],
    history: [],
    historyIndex: -1,
    request: 0,
  };
}

export function newConnection(): Connection {
  return {
    id: '',
    name: '',
    host: '',
    port: 22,
    username: '',
    authMethod: 'password',
    keyPath: null,
    rememberSecret: false,
    remotePath: '.',
    localPath: null,
    color: COLORS[0],
  };
}
