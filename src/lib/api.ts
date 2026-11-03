import { invoke, isTauri } from '@tauri-apps/api/core';
import { open } from '@tauri-apps/plugin-dialog';
import { i18n, msg, hasMessage, type DisplayMessage } from './i18n.svelte';
import type {
  AppError,
  Connection,
  ConflictDecision,
  DirectoryListing,
  Direction,
  SessionInfo,
  Transfer,
} from './types';

export const isDesktop = isTauri();

export function asError(error: unknown): AppError {
  if (typeof error === 'object' && error !== null && 'message' in error) {
    return {
      code: 'code' in error ? String(error.code) : 'UNKNOWN',
      message: String(error.message),
      ...('hostKey' in error ? { hostKey: error.hostKey as AppError['hostKey'] } : {}),
      ...('translation' in error && validTranslation(error.translation)
        ? { translation: error.translation }
        : {}),
    };
  }
  return {
    code: 'UNKNOWN',
    message: typeof error === 'string' ? error : 'An unexpected error occurred.',
  };
}

function validTranslation(value: unknown): value is NonNullable<AppError['translation']> {
  if (
    !value ||
    typeof value !== 'object' ||
    !('key' in value) ||
    !hasMessage(value.key) ||
    !('params' in value) ||
    !value.params ||
    typeof value.params !== 'object' ||
    Array.isArray(value.params)
  )
    return false;
  return Object.values(value.params).every(
    (item) => typeof item === 'string' || (typeof item === 'number' && Number.isFinite(item)),
  );
}

export function errorMessage(cause: unknown): DisplayMessage {
  const error = asError(cause);
  if (error.translation && hasMessage(error.translation.key))
    return msg(error.translation.key, error.translation.params);
  if (hasMessage(error.message)) return error.message;
  if (error.code === 'IO') return msg('System error: {details}', { details: error.message });
  if (error.code === 'SSH') return msg('SSH error: {details}', { details: error.message });
  if (error.code === 'SFTP') return msg('SFTP error: {details}', { details: error.message });
  return error.message;
}

async function command<T>(name: string, args?: Record<string, unknown>): Promise<T> {
  if (!isDesktop) {
    throw {
      code: 'DESKTOP_ONLY',
      message:
        'Open the desktop app to access local files and SFTP servers. This is only the web interface preview.',
    } satisfies AppError;
  }
  return invoke<T>(name, args);
}

export const api = {
  connections: () => command<Connection[]>('get_connections'),
  saveConnection: (connection: Connection, secret: string | null) =>
    command<Connection>('save_connection', { connection, secret }),
  deleteConnection: (connectionId: string) => command<void>('delete_connection', { connectionId }),
  connect: (connectionId: string, secret: string | null = null) =>
    command<SessionInfo>('connect_session', { connectionId, secret }),
  trustHost: (challengeId: string) => command<void>('trust_host_key', { challengeId }),
  disconnect: (sessionId: string) => command<void>('disconnect_session', { sessionId }),
  localHome: () => command<string>('get_local_home'),
  listLocal: (path: string) => command<DirectoryListing>('list_local', { path }),
  mkdirLocal: (parent: string, name: string) =>
    command<void>('create_local_directory', { parent, name }),
  removeLocal: (paths: string[]) => command<void>('delete_local', { paths }),
  listRemote: (sessionId: string, path: string) =>
    command<DirectoryListing>('list_remote', { sessionId, path }),
  mkdir: (sessionId: string, parent: string, name: string) =>
    command<void>('create_remote_directory', { sessionId, parent, name }),
  rename: (sessionId: string, path: string, newName: string) =>
    command<void>('rename_remote', { sessionId, path, newName }),
  remove: (sessionId: string, paths: string[]) =>
    command<void>('delete_remote', { sessionId, paths }),
  enqueue: (sessionId: string, direction: Direction, sources: string[], destination: string) =>
    command<Transfer[]>('enqueue_transfers', { sessionId, direction, sources, destination }),
  transfers: () => command<Transfer[]>('get_transfers'),
  cancel: (transferId: string) => command<void>('cancel_transfer', { transferId }),
  resolveConflict: (transferId: string, conflictId: string, decision: ConflictDecision) =>
    command<void>('resolve_transfer_conflict', { transferId, conflictId, decision }),
  clearFinished: () => command<void>('clear_finished_transfers'),
};

export async function chooseFiles(directory = false): Promise<string[]> {
  if (!isDesktop) {
    await command('file_picker');
    return [];
  }
  const result = await open({
    directory,
    multiple: !directory,
    title: i18n.t(directory ? 'Select a folder' : 'Select files'),
  });
  return result === null ? [] : Array.isArray(result) ? result : [result];
}

export async function chooseKey(): Promise<string | null> {
  if (!isDesktop) {
    await command('file_picker');
    return null;
  }
  return open({
    directory: false,
    multiple: false,
    title: i18n.t('Select private SSH key'),
  });
}
