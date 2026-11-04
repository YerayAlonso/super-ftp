<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { version as appVersion } from '../package.json';
  import { getCurrentWebview } from '@tauri-apps/api/webview';
  import {
    Activity,
    ArrowDown,
    ArrowRight,
    ArrowRightLeft,
    ArrowUp,
    ArrowUpRight,
    Check,
    ChevronDown,
    ChevronRight,
    CircleHelp,
    Command,
    Eye,
    EyeOff,
    FileKey,
    Folder,
    Globe,
    KeyRound,
    Keyboard,
    LayoutGrid,
    ListFilter,
    LoaderCircle,
    LockKeyhole,
    Moon,
    MoreHorizontal,
    PanelLeftClose,
    PanelLeftOpen,
    Plus,
    Search,
    Server,
    ShieldAlert,
    ShieldCheck,
    SlidersHorizontal,
    Sun,
    Terminal,
    Trash2,
    X,
  } from '@lucide/svelte';
  import FilePane from './components/FilePane.svelte';
  import ConnectionDialog from './components/ConnectionDialog.svelte';
  import Modal from './components/Modal.svelte';
  import TransferPanel from './components/TransferPanel.svelte';
  import LanguageSelect from './components/LanguageSelect.svelte';
  import ConflictDialog from './components/ConflictDialog.svelte';
  import { api, asError, errorMessage, chooseFiles, isDesktop } from './lib/api';
  import { i18n, msg, type DisplayMessage } from './lib/i18n.svelte';
  import {
    createPane,
    type Connection,
    type Direction,
    type FileEntry,
    type HostKeyChallenge,
    type PaneState,
    type SessionTab,
    type Transfer,
  } from './lib/types';
  import { formatBytes, formatDate, formatPermissions, isActive } from './lib/format';

  let connections = $state<Connection[]>([]);
  let tabs = $state<SessionTab[]>([]);
  let activeTabId = $state<string | null>(null);
  let workspaceLocal = $state(createPane());
  let emptyRemote = $state(createPane());
  let localHome = $state('');
  let transfers = $state<Transfer[]>([]);
  let connecting = $state<string[]>([]);
  let connectionSearch = $state('');
  let connectionMenu = $state<string | null>(null);
  let editor = $state<{ connection: Connection | null } | null>(null);
  let hostPrompt = $state<{
    key: HostKeyChallenge;
    connection: Connection;
    secret: string | null;
  } | null>(null);
  let hostVerified = $state(false);
  let hostBusy = $state(false);
  let loginPrompt = $state<{ connection: Connection; error: DisplayMessage } | null>(null);
  let loginSecret = $state('');
  let loginBusy = $state(false);
  let picker = $state(false);
  let pickerSearch = $state('');
  let about = $state(false);
  let preferencesOpen = $state(false);
  let licensesOpen = $state(false);
  function initialIncludePrereleases() {
    try { return localStorage.getItem('superftp-include-previews') === 'true'; }
    catch { return false; }
  }
  let includePrereleases = $state(initialIncludePrereleases());
  let automaticUpdateNotice = $state<string | null>(null);
  let checkingUpdates = $state(false);
  let signedUpdateVersion = $state<string | null>(null);
  let installingUpdate = $state(false);
  let updateError = $state<DisplayMessage>('');
  let updateResult = $state<Awaited<ReturnType<typeof api.checkUpdates>> | null>(null);
  async function checkUpdates() {
    if (checkingUpdates) return;
    checkingUpdates = true;
    updateError = '';
    updateResult = null;
    try {
      updateResult = await api.checkUpdates(includePrereleases);
      signedUpdateVersion = await api.checkSignedUpdate(includePrereleases).catch(() => null);
    }
    catch (cause) { updateError = errorMessage(cause); }
    finally { checkingUpdates = false; }
  }
  async function installSignedUpdate() {
    if (!signedUpdateVersion || installingUpdate) return;
    installingUpdate = true;
    updateError = '';
    try { await api.installSignedUpdate(includePrereleases, signedUpdateVersion); }
    catch (cause) { updateError = errorMessage(cause); preferencesOpen = true; }
    finally { installingUpdate = false; }
  }
  async function openReleasesPage() {
    try { await api.openReleasesPage(); }
    catch (cause) { updateError = errorMessage(cause); }
  }
  let thirdPartyNotices = $state('');
  async function loadThirdPartyNotices() {
    if (!thirdPartyNotices) thirdPartyNotices = (await import('./lib/third-party-notices.txt?raw')).default;
  }
  let showHidden = $state(false);
  let localVisible = $state(true);
  let nativeDrop = $state(false);
  function initialTheme() {
    try {
      return localStorage.getItem('superftp-theme') === 'light' ? 'light' : 'dark';
    } catch {
      return 'dark';
    }
  }
  let theme = $state(initialTheme());
  const fontSizes = [80, 90, 100, 110, 120, 130];
  function initialFontSize() {
    try {
      const saved = Number(localStorage.getItem('superftp-font-size'));
      return fontSizes.includes(saved) ? saved : 100;
    } catch {
      return 100;
    }
  }
  let fontSize = $state(initialFontSize());
  const shortcutModifier = /Mac/.test(navigator.platform) ? '⌘' : 'Ctrl';
  let confirmAction = $state<{
    title: DisplayMessage;
    description: DisplayMessage;
    label: DisplayMessage;
    action: () => Promise<void>;
    danger?: boolean;
  } | null>(null);
  let actionBusy = $state(false);
  let textAction = $state<{
    kind: 'mkdir' | 'rename';
    sessionId: string | null;
    location: 'local' | 'remote';
    path: string;
    value: string;
  } | null>(null);
  let properties = $state<{ entry: FileEntry; location: 'local' | 'remote' } | null>(null);
  let textBusy = $state(false);
  let modalError = $state<DisplayMessage>('');
  let notices = $state<
    { id: number; message: DisplayMessage; kind: 'error' | 'success' | 'info' }[]
  >([]);
  let noticeId = 0;
  const noticeTimers = new Set<ReturnType<typeof setTimeout>>();

  const activeTab = $derived(tabs.find((tab) => tab.id === activeTabId) ?? null);
  const localPane = $derived(activeTab?.local ?? workspaceLocal);
  const remotePane = $derived(activeTab?.remote ?? emptyRemote);
  const visibleConnections = $derived(
    connections.filter((connection) =>
      `${connection.name} ${connection.host} ${connection.username}`
        .toLocaleLowerCase(i18n.locale)
        .includes(connectionSearch.toLocaleLowerCase(i18n.locale)),
    ),
  );
  const pickerConnections = $derived(
    connections.filter((connection) =>
      `${connection.name} ${connection.host}`
        .toLocaleLowerCase(i18n.locale)
        .includes(pickerSearch.toLocaleLowerCase(i18n.locale)),
    ),
  );
  const activeTransfers = $derived(transfers.filter(isActive));
  const waitingConflict = $derived(transfers.find(transfer => transfer.status === 'awaitingDecision' && transfer.conflict));
  const totalSpeed = $derived(
    activeTransfers.reduce((sum, transfer) => sum + transfer.bytesPerSecond, 0),
  );

  $effect(() => {
    document.documentElement.dataset.theme = theme;
    try {
      localStorage.setItem('superftp-theme', theme);
    } catch {
      /* Private browsing may disallow preferences. */
    }
  });

  $effect(() => {
    document.documentElement.lang = i18n.language;
  });

  $effect(() => {
    try { localStorage.setItem('superftp-include-previews', String(includePrereleases)); }
    catch { /* Optional preference storage. */ }
  });

  $effect(() => {
    document.documentElement.style.zoom = `${fontSize}%`;
    try {
      localStorage.setItem('superftp-font-size', String(fontSize));
    } catch {
      /* Optional preference storage. */
    }
  });

  function adjustFontSize(step: number) {
    const index = fontSizes.indexOf(fontSize);
    fontSize = fontSizes[Math.max(0, Math.min(fontSizes.length - 1, index + step))];
  }

  function notify(message: DisplayMessage, kind: 'error' | 'success' | 'info' = 'error') {
    const id = ++noticeId;
    notices = [...notices.slice(-3), { id, message, kind }];
    const timer = setTimeout(
      () => {
        notices = notices.filter((notice) => notice.id !== id);
        noticeTimers.delete(timer);
      },
      kind === 'error' ? 10000 : 5000,
    );
    noticeTimers.add(timer);
  }

  async function loadPane(
    pane: PaneState,
    kind: 'local' | 'remote',
    path: string,
    sessionId?: string,
    history = true,
  ) {
    const request = ++pane.request;
    pane.loading = true;
    pane.error = null;
    try {
      const listing =
        kind === 'local' ? await api.listLocal(path) : await api.listRemote(sessionId!, path);
      if (request !== pane.request) return;
      pane.listing = listing;
      pane.selected = [];
      if (history && pane.history[pane.historyIndex] !== listing.path) {
        pane.history = [...pane.history.slice(0, pane.historyIndex + 1), listing.path];
        pane.historyIndex = pane.history.length - 1;
      }
    } catch (cause) {
      if (request === pane.request) pane.error = errorMessage(cause);
    } finally {
      if (request === pane.request) pane.loading = false;
    }
  }

  const navigateLocal = (path: string) => {
    void loadPane(localPane, 'local', path);
  };
  const navigateRemote = (path: string) => {
    if (activeTab) void loadPane(activeTab.remote, 'remote', path, activeTab.id);
  };
  function historyStep(kind: 'local' | 'remote', step: number) {
    const pane = kind === 'local' ? localPane : remotePane;
    const index = pane.historyIndex + step;
    if (index < 0 || index >= pane.history.length) return;
    pane.historyIndex = index;
    void loadPane(pane, kind, pane.history[index], activeTab?.id, false);
  }
  function refresh(kind: 'local' | 'remote') {
    const pane = kind === 'local' ? localPane : remotePane;
    if (pane.listing) void loadPane(pane, kind, pane.listing.path, activeTab?.id, false);
  }

  async function openConnection(
    connection: Connection,
    forceNew = false,
    secret: string | null = null,
  ) {
    const existing = tabs.find((tab) => tab.connection.id === connection.id);
    if (existing && !forceNew) {
      activeTabId = existing.id;
      return;
    }
    if (connecting.includes(connection.id)) return;
    connecting = [...connecting, connection.id];
    try {
      const info = await api.connect(connection.id, secret);
      const local = createPane();
      local.listing = localPane.listing;
      if (local.listing) {
        local.history = [local.listing.path];
        local.historyIndex = 0;
      }
      tabs = [
        ...tabs,
        { id: info.id, connection: { ...connection }, info, local, remote: createPane() },
      ];
      activeTabId = info.id;
      const tab = tabs.find((tab) => tab.id === info.id)!;
      const initialPath = connection.remotePath === '.' ? info.homePath : connection.remotePath;
      await loadPane(tab.remote, 'remote', initialPath, info.id);
      if (tab.remote.error && initialPath !== info.homePath) {
        notify('The starting folder is unavailable. Your home folder has been opened.', 'info');
        await loadPane(tab.remote, 'remote', info.homePath, info.id);
      }
      if (connection.localPath) await loadPane(tab.local, 'local', connection.localPath);
      notify(msg('Securely connected to {name}.', { name: connection.name }), 'success');
    } catch (cause) {
      const error = asError(cause);
      if (error.hostKey) {
        hostVerified = false;
        hostPrompt = { key: error.hostKey, connection, secret };
      } else if (
        error.code === 'SECRET_REQUIRED' ||
        (error.code === 'AUTH' && connection.authMethod !== 'agent') ||
        (error.code === 'KEY' && !secret)
      ) {
        loginSecret = '';
        loginPrompt = {
          connection,
          error: error.code === 'SECRET_REQUIRED' ? '' : errorMessage(error),
        };
      } else notify(errorMessage(error));
    } finally {
      connecting = connecting.filter((id) => id !== connection.id);
    }
  }

  async function saveConnection(connection: Connection, secret: string | null, connect: boolean) {
    const saved = await api.saveConnection(connection, secret);
    connections = [...connections.filter((item) => item.id !== saved.id), saved];
    editor = null;
    if (connect) await openConnection(saved, true, secret);
    else notify('Connection saved.', 'success');
  }

  async function trustHost() {
    if (!hostPrompt || hostPrompt.key.changed || !hostVerified || hostBusy) return;
    const pending = hostPrompt;
    hostBusy = true;
    try {
      await api.trustHost(pending.key.id);
      hostPrompt = null;
      await openConnection(pending.connection, true, pending.secret);
    } catch (cause) {
      notify(errorMessage(cause));
    } finally {
      hostBusy = false;
    }
  }

  async function submitLogin(event: SubmitEvent) {
    event.preventDefault();
    if (!loginPrompt || loginBusy) return;
    const connection = loginPrompt.connection;
    const secret = loginSecret;
    loginSecret = '';
    loginPrompt = null;
    loginBusy = true;
    try {
      await openConnection(connection, true, secret);
    } finally {
      loginBusy = false;
    }
  }

  function removeConnection(connection: Connection) {
    modalError = '';
    confirmAction = {
      title: 'Delete this connection?',
      description: msg(
        '“{name}” and its saved secret will be deleted. Open sessions will stay active.',
        { name: connection.name },
      ),
      label: 'Delete connection',
      danger: true,
      action: async () => {
        await api.deleteConnection(connection.id);
        connections = connections.filter((item) => item.id !== connection.id);
      },
    };
  }

  async function closeTab(id: string) {
    async function close() {
      await api.disconnect(id);
      const index = tabs.findIndex((tab) => tab.id === id);
      tabs = tabs.filter((tab) => tab.id !== id);
      if (activeTabId === id) activeTabId = tabs[Math.min(index, tabs.length - 1)]?.id ?? null;
    }
    if (transfers.some((transfer) => transfer.sessionId === id && isActive(transfer))) {
      modalError = '';
      confirmAction = {
        title: 'Close this session?',
        description: 'This session has pending transfers. Closing it will cancel them.',
        label: 'Cancel and close',
        danger: true,
        action: close,
      };
    } else {
      try {
        await close();
      } catch (cause) {
        notify(errorMessage(cause));
      }
    }
  }

  async function pickLocal() {
    try {
      const [path] = await chooseFiles(true);
      if (path) navigateLocal(path);
    } catch (cause) {
      notify(errorMessage(cause));
    }
  }

  async function queue(direction: Direction, sources?: string[], tab = activeTab) {
    if (!tab) {
      notify('Connect to a server before transferring files.', 'info');
      return;
    }
    const from = direction === 'upload' ? tab.local : tab.remote;
    const to = direction === 'upload' ? tab.remote : tab.local;
    const paths = sources ?? from.selected.map((entry) => entry.path);
    if (!paths.length) return;
    if (!to.listing) {
      notify('Open a destination folder before transferring.');
      return;
    }
    try {
      const added = await api.enqueue(tab.id, direction, paths, to.listing.path);
      // An event can beat the command response. Never replace fresher progress.
      transfers = [
        ...transfers,
        ...added.filter((item) => !transfers.some((existing) => existing.id === item.id)),
      ];
    } catch (cause) {
      notify(errorMessage(cause));
    }
  }

  async function uploadFiles(directory: boolean) {
    const tab = activeTab;
    try {
      const paths = await chooseFiles(directory);
      if (paths.length) await queue('upload', paths, tab);
    } catch (cause) {
      notify(errorMessage(cause));
    }
  }

  async function retryTransfer(transfer: Transfer) {
    if (!tabs.some((tab) => tab.id === transfer.sessionId)) {
      notify(
        'The original session is closed. Reconnect and start a new transfer.',
        'info',
      );
      return;
    }
    try {
      const added = await api.enqueue(
        transfer.sessionId,
        transfer.direction,
        [transfer.source],
        transfer.destination,
      );
      transfers = [
        ...transfers,
        ...added.filter((item) => !transfers.some((existing) => existing.id === item.id)),
      ];
    } catch (cause) {
      notify(errorMessage(cause));
    }
  }

  async function clearTransfers() {
    try {
      await api.clearFinished();
      transfers = transfers.filter(isActive);
    } catch (cause) {
      notify(errorMessage(cause));
    }
  }

  function promptMkdir(location: 'local' | 'remote' = 'remote') {
    const pane = location === 'local' ? localPane : remotePane;
    if (!pane.listing || (location === 'remote' && !activeTab)) return;
    modalError = '';
    textAction = {
      kind: 'mkdir',
      sessionId: activeTab?.id ?? null,
      location,
      path: pane.listing.path,
      value: '',
    };
  }
  function promptRename() {
    if (!activeTab || remotePane.selected.length !== 1) return;
    const entry = remotePane.selected[0];
    modalError = '';
    textAction = {
      kind: 'rename',
      sessionId: activeTab.id,
      location: 'remote',
      path: entry.path,
      value: entry.name,
    };
  }
  function promptDelete() {
    if (!activeTab || !remotePane.selected.length) return;
    const tab = activeTab;
    const paths = remotePane.selected.map((entry) => entry.path);
    modalError = '';
    confirmAction = {
      title: paths.length === 1 ? 'Delete this item?' : 'Delete these items?',
      description: msg(
        '{count} server items will be permanently deleted, including all folder contents.',
        { count: paths.length },
      ),
      label: 'Delete from server',
      danger: true,
      action: async () => {
        try {
          await api.remove(tab.id, paths);
        } finally {
          if (tab.remote.listing)
            await loadPane(tab.remote, 'remote', tab.remote.listing.path, tab.id, false);
        }
      },
    };
  }
  function promptDeleteLocal() {
    const pane = localPane;
    if (!pane.selected.length) return;
    const paths = pane.selected.map((entry) => entry.path);
    modalError = '';
    confirmAction = {
      title: paths.length === 1 ? 'Delete this item?' : 'Delete these items?',
      description: msg(
        '{count} local items will be permanently deleted, including all folder contents.',
        { count: paths.length },
      ),
      label: 'Delete from computer',
      danger: true,
      action: async () => {
        try {
          await api.removeLocal(paths);
        } finally {
          if (pane.listing) await loadPane(pane, 'local', pane.listing.path, undefined, false);
        }
      },
    };
  }
  async function submitText(event: SubmitEvent) {
    event.preventDefault();
    if (!textAction || textBusy) return;
    textBusy = true;
    modalError = '';
    try {
      if (textAction.kind === 'mkdir' && textAction.location === 'local')
        await api.mkdirLocal(textAction.path, textAction.value);
      else if (textAction.kind === 'mkdir')
        await api.mkdir(textAction.sessionId!, textAction.path, textAction.value);
      else await api.rename(textAction.sessionId!, textAction.path, textAction.value);
      const location = textAction.location;
      const tab = tabs.find((tab) => tab.id === textAction!.sessionId);
      textAction = null;
      if (location === 'local') {
        const pane = tab?.local ?? workspaceLocal;
        if (pane.listing) await loadPane(pane, 'local', pane.listing.path, undefined, false);
      } else if (tab?.remote.listing)
        await loadPane(tab.remote, 'remote', tab.remote.listing.path, tab.id, false);
    } catch (cause) {
      modalError = errorMessage(cause);
    } finally {
      textBusy = false;
    }
  }
  async function runConfirmation() {
    if (!confirmAction || actionBusy) return;
    actionBusy = true;
    modalError = '';
    try {
      await confirmAction.action();
      confirmAction = null;
    } catch (cause) {
      modalError = errorMessage(cause);
    } finally {
      actionBusy = false;
    }
  }
  function openPicker() {
    if (!connections.length) editor = { connection: null };
    else {
      pickerSearch = '';
      picker = true;
    }
  }
  function shortcuts(event: KeyboardEvent) {
    if (document.querySelector('dialog[open]')) return;
    if (event.ctrlKey || event.metaKey) {
      if (event.key.toLowerCase() === 'n') {
        event.preventDefault();
        editor = { connection: null };
      }
      if (event.key.toLowerCase() === 'k') {
        event.preventDefault();
        openPicker();
      }
      if (event.key.toLowerCase() === 'l') {
        event.preventDefault();
        const input = document.getElementById(
          activeTab ? 'remote-path' : 'local-path',
        ) as HTMLInputElement | null;
        input?.focus();
        input?.select();
      }
      if (event.key.toLowerCase() === 'h') {
        event.preventDefault();
        showHidden = !showHidden;
      }
    }
    if (event.key === 'F5') {
      event.preventDefault();
      refresh('local');
      refresh('remote');
    }
  }

  onMount(() => {
    if (!isDesktop)
      return () => {
        noticeTimers.forEach(clearTimeout);
      };
    let disposed = false;
    const startupPreviewSetting = includePrereleases;
    // Background checks are read-only; failure must never interrupt startup.
    void api.checkUpdates(startupPreviewSetting).then((result) => {
      if (!disposed && includePrereleases === startupPreviewSetting && result.updateAvailable && result.latestVersion)
        automaticUpdateNotice = result.latestVersion;
    }).catch(() => {});
    void api.checkSignedUpdate(startupPreviewSetting).then((version) => {
      if (!disposed && includePrereleases === startupPreviewSetting) signedUpdateVersion = version;
    }).catch(() => {});
    const cleanup: (() => void)[] = [];
    async function bootstrap() {
      // Subscribe before querying the snapshot so progress events aren't lost.
      const unlisten = await listen<Transfer>('transfer-update', ({ payload }) => {
        const previous = transfers.find((item) => item.id === payload.id);
        transfers = previous
          ? transfers.map((item) => (item.id === payload.id ? payload : item))
          : [...transfers, payload];
        if (payload.status === 'completed' && previous?.status !== 'completed') {
          const tab = tabs.find((tab) => tab.id === payload.sessionId);
          const pane = payload.direction === 'upload' ? tab?.remote : tab?.local;
          if (tab && pane?.listing)
            void loadPane(
              pane,
              payload.direction === 'upload' ? 'remote' : 'local',
              pane.listing.path,
              tab.id,
              false,
            );
        }
      });
      if (disposed) {
        unlisten();
        return;
      }
      cleanup.push(unlisten);
      const undrop = await getCurrentWebview().onDragDropEvent(({ payload }) => {
        if (payload.type === 'leave') {
          nativeDrop = false;
          return;
        }
        const rect = document.querySelector('[data-drop-zone="remote"]')?.getBoundingClientRect();
        const position = payload.position;
        const x = position.x / window.devicePixelRatio;
        const y = position.y / window.devicePixelRatio;
        const inside = Boolean(
          rect &&
          x >= rect.left &&
          x <= rect.right &&
          y >= rect.top &&
          y <= rect.bottom &&
          activeTab,
        );
        if (payload.type === 'drop') {
          nativeDrop = false;
          if (inside) void queue('upload', payload.paths);
        } else nativeDrop = inside;
      });
      if (disposed) {
        undrop();
        return;
      }
      cleanup.push(undrop);
      await Promise.allSettled([
        api
          .connections()
          .then((items) => {
            connections = items;
          })
          .catch((cause) => notify(errorMessage(cause))),
        api
          .localHome()
          .then(async (path) => {
            localHome = path;
            await loadPane(workspaceLocal, 'local', path);
          })
          .catch((cause) => notify(errorMessage(cause))),
        api
          .transfers()
          .then((items) => {
            transfers = [
              ...transfers,
              ...items.filter((item) => !transfers.some((current) => current.id === item.id)),
            ];
          })
          .catch((cause) => notify(errorMessage(cause))),
      ]);
    }
    void bootstrap().catch((cause) => notify(errorMessage(cause)));
    return () => {
      disposed = true;
      cleanup.forEach((fn) => fn());
      noticeTimers.forEach(clearTimeout);
    };
  });
</script>

<svelte:window onkeydown={shortcuts} onclick={() => (connectionMenu = null)} />

<div class="app-shell">
  <aside class="sidebar">
    <div class="brand">
      <div class="brand-icon"><ArrowRightLeft size={23} strokeWidth={2.3} /></div>
      <div>
        <span class="brand-name">super<span>ftp</span><span class="brand-dot">.</span></span><span
          class="brand-caption">{i18n.t('SMALL APP. BIG CONNECTIONS.')}</span
        >
      </div>
    </div>
    <button class="button primary new-connection" onclick={() => (editor = { connection: null })}
      ><Plus size={17} />{i18n.t('New connection')}<kbd>{shortcutModifier} N</kbd></button
    >
    <div class="sidebar-search">
      <Search size={15} /><input
        aria-label={i18n.t('Search connections')}
        placeholder={i18n.t('Search connections…')}
        bind:value={connectionSearch}
      /><kbd>{shortcutModifier} K</kbd>
    </div>
    <div class="sidebar-section-heading">
      <span>{i18n.t('YOUR CONNECTIONS')}</span><span>{i18n.number(connections.length)}</span
      ><button
        class="icon-button tiny"
        aria-label={i18n.t('Add a connection')}
        onclick={() => (editor = { connection: null })}><Plus size={15} /></button
      >
    </div>
    <nav class="connections-list" aria-label={i18n.t('Saved connections')}>
      {#if visibleConnections.length}
        {#each visibleConnections as connection (connection.id)}
          {@const session = tabs.find((tab) => tab.connection.id === connection.id)}
          <div class="connection-row" class:active={activeTab?.connection.id === connection.id}>
            <button class="connection-main" onclick={() => openConnection(connection)}
              ><span class="connection-icon" style:--connection-color={connection.color}
                >{#if connecting.includes(connection.id)}<LoaderCircle
                    size={17}
                    class="spin"
                  />{:else}<Server size={17} />{/if}</span
              ><span class="connection-description"
                ><strong>{connection.name}</strong><span
                  >{connection.username}@{connection.host}</span
                ></span
              >{#if session}<span class="connection-live" title={i18n.t('Active session')}
                ></span>{/if}</button
            >
            <button
              class="icon-button connection-more"
              aria-label={i18n.t('Options for {name}', { name: connection.name })}
              onclick={(event) => {
                event.stopPropagation();
                connectionMenu = connectionMenu === connection.id ? null : connection.id;
              }}><MoreHorizontal size={17} /></button
            >
            {#if connectionMenu === connection.id}<div class="dropdown connection-dropdown">
                <button onclick={() => openConnection(connection, true)}
                  ><Plus size={14} />{i18n.t('Open another session')}</button
                ><button onclick={() => (editor = { connection })}
                  ><SlidersHorizontal size={14} />{i18n.t('Edit connection')}</button
                ><button class="danger-text" onclick={() => removeConnection(connection)}
                  ><Trash2 size={14} />{i18n.t('Delete connection')}</button
                >
              </div>{/if}
          </div>
        {/each}
      {:else if connectionSearch}<div class="sidebar-empty">
          <Search size={21} /><strong>{i18n.t('No matches')}</strong>
          <p>{i18n.t('Try a different connection name.')}</p>
        </div>
      {:else}<div class="sidebar-empty">
          <div class="empty-server-icon"><Server size={23} strokeWidth={1.4} /><span>+</span></div>
          <strong>{i18n.t('Your next destination, right here.')}</strong>
          <p>{i18n.t('Save your servers and connect with one click.')}</p>
          <button class="text-button mint-text" onclick={() => (editor = { connection: null })}
            >{i18n.t('Add your first connection')}<ArrowUpRight size={13} /></button
          >
        </div>{/if}
    </nav>
    <div class="sidebar-bottom">
      <div class="private-note">
        <ShieldCheck size={18} />
        <div>
          <strong>{i18n.t('Private by design')}</strong><span
            >{i18n.t('Your servers. Your data.')}</span
          >
        </div>
      </div>
      <div class="sidebar-bottom-row">
        <span
          ><span class="tiny-dot"></span>{`v${appVersion}`}
          <span class="version-label">{i18n.t('PREVIEW')}</span></span
        ><button
          class="icon-button"
          aria-label={i18n.t('Help and shortcuts')}
          title={i18n.t('Help and shortcuts')}
          onclick={() => (about = true)}><CircleHelp size={17} /></button
        >
      </div>
    </div>
  </aside>

  <main class="workspace">
    <header class="topbar">
      <div class="workspace-breadcrumb">
        <LayoutGrid size={15} /><span>{i18n.t('Workspace')}</span><ChevronRight
          size={12}
        /><strong>{activeTab?.connection.name ?? i18n.t('Home')}</strong>
      </div>
      <div class="topbar-right">
        <span class="app-status"
          ><span class="tiny-dot"></span>{i18n.t(
            connecting.length ? 'Connecting…' : activeTab ? 'Secure connection' : 'All set',
          )}</span
        ><span class="topbar-divider"></span><button class="icon-button preferences-trigger" aria-label={i18n.t('Preferences')} title={i18n.t('Preferences')} onclick={() => (preferencesOpen = true)}><SlidersHorizontal size={17} /></button
        ><button class="icon-button" aria-label={i18n.t('Help')} onclick={() => (about = true)}
          ><CircleHelp size={17} /></button
        >
      </div>
    </header>
    {#if !isDesktop}<div class="web-preview-banner">
        <Globe size={14} /><span
          >{i18n.t(
            'Web interface preview. Connections and local files require the desktop app.',
          )}</span
        ><code>npm run desktop</code>
      </div>{/if}
    <div class="workspace-intro">
      <div>
        <div class="eyebrow">{i18n.t('YOUR SPACE, WITHOUT BORDERS')}</div>
        <h1>{i18n.t('Your files, in good hands')}<span>.</span></h1>
        <p>{i18n.t('Connect. Drag. Done. We’ll handle the rest.')}</p>
      </div>
      <div class="intro-controls">
        <button
          class="icon-button"
          class:control-active={showHidden}
          aria-label={i18n.t(showHidden ? 'Hide hidden files' : 'Show hidden files')}
          title={`${i18n.t('Hidden files')} (Ctrl+H)`}
          onclick={() => (showHidden = !showHidden)}
          >{#if showHidden}<Eye size={17} />{:else}<EyeOff size={17} />{/if}</button
        ><button
          class="icon-button"
          class:control-active={!localVisible}
          aria-label={i18n.t(localVisible ? 'Hide local pane' : 'Show local pane')}
          title={i18n.t('Local pane')}
          onclick={() => (localVisible = !localVisible)}
          >{#if localVisible}<PanelLeftClose size={17} />{:else}<PanelLeftOpen
              size={17}
            />{/if}</button
        ><button class="button secondary" onclick={openPicker}
          ><Plus size={15} />{i18n.t('New session')}</button
        >
      </div>
    </div>
    <div class="session-tabs" role="tablist" aria-label={i18n.t('SFTP sessions')}>
      <button
        role="tab"
        aria-selected={!activeTab}
        class="session-tab home-tab"
        class:active={!activeTab}
        onclick={() => (activeTabId = null)}><LayoutGrid size={14} />{i18n.t('Home')}</button
      >{#each tabs as tab (tab.id)}<div
          class="session-tab-wrap"
          class:active={activeTabId === tab.id}
        >
          <button
            class="session-tab"
            class:active={activeTabId === tab.id}
            role="tab"
            aria-selected={activeTabId === tab.id}
            onclick={() => (activeTabId = tab.id)}
            ><span class="tab-dot" style:background={tab.connection.color}></span>{tab.connection
              .name}<span class="tab-protocol">SFTP</span></button
          ><button
            class="tab-close"
            aria-label={i18n.t('Close session {name}', { name: tab.connection.name })}
            onclick={() => closeTab(tab.id)}><X size={12} /></button
          >
        </div>{/each}<button
        class="icon-button new-tab"
        aria-label={i18n.t('Open a new session')}
        title={i18n.t('New session')}
        onclick={openPicker}><Plus size={16} /></button
      ><span class="session-tabs-end"
        >{tabs.length
          ? i18n.count('{count} active session', '{count} active sessions', tabs.length)
          : i18n.t('A place for every connection')}</span
      >
    </div>
    <div class="file-workspace" class:single-pane={!localVisible}>
      {#if localVisible}<FilePane
          kind="local"
          pane={localPane}
          onSelectionChange={(entries) => (localPane.selected = entries)}
          onDismissError={() => (localPane.error = null)}
          connected={Boolean(activeTab)}
          {showHidden}
          onNavigate={navigateLocal}
          onBack={() => historyStep('local', -1)}
          onForward={() => historyStep('local', 1)}
          onHome={() => localHome && navigateLocal(localHome)}
          onRefresh={() => refresh('local')}
          onChoose={pickLocal}
          onTransfer={() => queue('upload')}
          onNewFolder={() => promptMkdir('local')}
          onDelete={promptDeleteLocal}
          onProperties={(entry) => (properties = { entry, location: 'local' })}
          onInternalDrop={(paths) => queue('upload', paths)}
        />{/if}
      {#if localVisible}<div class="pane-bridge">
          <button
            class="bridge-button"
            aria-label={i18n.t('Upload selection')}
            title={i18n.t('Upload selected items')}
            disabled={!activeTab || !localPane.selected.length}
            onclick={() => queue('upload')}><ArrowRight size={15} /></button
          ><button
            class="bridge-button"
            aria-label={i18n.t('Download selection')}
            title={i18n.t('Download selected items')}
            disabled={!activeTab || !remotePane.selected.length}
            onclick={() => queue('download')}><ArrowRight size={15} class="rotate-180" /></button
          >
        </div>{/if}
      <FilePane
        kind="remote"
        pane={remotePane}
        onSelectionChange={(entries) => (remotePane.selected = entries)}
        onDismissError={() => (remotePane.error = null)}
        connected={Boolean(activeTab)}
        host={activeTab ? `${activeTab.connection.username}@${activeTab.connection.host}` : ''}
        {showHidden}
        dropActive={nativeDrop}
        onNavigate={navigateRemote}
        onBack={() => historyStep('remote', -1)}
        onForward={() => historyStep('remote', 1)}
        onHome={() => activeTab && navigateRemote(activeTab.info.homePath)}
        onRefresh={() => refresh('remote')}
        onConnect={openPicker}
        onTransfer={() => queue('download')}
        onUploadFiles={uploadFiles}
        onNewFolder={() => promptMkdir('remote')}
        onRename={promptRename}
        onDelete={promptDelete}
        onProperties={(entry) => (properties = { entry, location: 'remote' })}
      />
    </div>
    <TransferPanel
      {transfers}
      onCancel={(id) => {
        void api.cancel(id).catch((cause) => notify(errorMessage(cause)));
      }}
      onRetry={retryTransfer}
      onClear={clearTransfers}
    />
    <footer class="statusbar">
      <span
        ><LockKeyhole size={12} />{i18n.t(
          activeTab ? 'Server identity verified' : 'Secure transfers over SSH',
        )}<span class="statusbar-separator">·</span><span>{i18n.t('SFTP, no shortcuts.')}</span
        ></span
      ><span
        >{#if totalSpeed}<Activity size={12} />{formatBytes(totalSpeed, i18n.locale)}/s<span
            class="statusbar-separator">·</span
          >{/if}<span class="status-dot" class:busy={activeTransfers.length > 0}
        ></span>{activeTransfers.length
          ? i18n.count(
              '{count} transfer in progress',
              '{count} transfers in progress',
              activeTransfers.length,
            )
          : i18n.t('No active transfers')}</span
      >
    </footer>
  </main>
</div>

{#if editor}<ConnectionDialog
    connection={editor.connection}
    onClose={() => (editor = null)}
    onSave={saveConnection}
  />{/if}

{#if picker}<Modal
    title={i18n.t('Where to today?')}
    eyebrow={i18n.t('NEW SESSION')}
    small
    onClose={() => (picker = false)}
    ><div class="picker-search">
      <Search size={17} /><input
        data-initial-focus
        aria-label={i18n.t('Filter connections')}
        placeholder={i18n.t('Search for a server…')}
        bind:value={pickerSearch}
      />
    </div>
    <div class="picker-list">
      {#each pickerConnections as connection}<button
          onclick={() => {
            picker = false;
            void openConnection(connection, true);
          }}
          ><span class="connection-icon" style:--connection-color={connection.color}
            ><Server size={19} /></span
          ><span
            ><strong>{connection.name}</strong><small
              >{connection.username}@{connection.host}:{connection.port}</small
            ></span
          ><ArrowUpRight size={17} /></button
        >{/each}{#if !pickerConnections.length}<p class="muted">
          {i18n.t('No connections match your search.')}
        </p>{/if}
    </div>
    {#snippet footer()}<button
        class="text-button mint-text"
        onclick={() => {
          picker = false;
          editor = { connection: null };
        }}><Plus size={16} />{i18n.t('Create a new connection')}</button
      ><kbd>{i18n.t('ESC to close')}</kbd>{/snippet}</Modal
  >{/if}

{#if hostPrompt}<Modal
    title={i18n.t(
      hostPrompt.key.changed
        ? 'The server’s identity has changed.'
        : 'Let’s verify this server.',
    )}
    eyebrow={i18n.t('SSH VERIFICATION')}
    small
    busy={hostBusy}
    onClose={() => (hostPrompt = null)}
    ><div class="host-key-icon" class:changed={hostPrompt.key.changed}>
      {#if hostPrompt.key.changed}<ShieldAlert size={28} />{:else}<ShieldCheck size={28} />{/if}
    </div>
    <p class="dialog-description">
      {i18n.t(
        hostPrompt.key.changed
          ? 'The received key does not match the saved key. This may be a legitimate rotation or an interception attempt. No credentials have been sent.'
          : 'This is your first connection to this server. Compare its fingerprint with one provided by the administrator through an independent channel before trusting it.',
      )}
    </p>
    <div class="host-details">
      <span>{i18n.t('Server')}</span><strong>{hostPrompt.key.host}:{hostPrompt.key.port}</strong
      ><span>{i18n.t('Algorithm')}</span><strong>{hostPrompt.key.algorithm}</strong>
    </div>
    <div class="fingerprint-label">
      {i18n.t('SHA-256 fingerprint')}<code>{hostPrompt.key.fingerprint}</code>
    </div>
    {#if hostPrompt.key.previousFingerprint}<div class="fingerprint-label previous-key">
        {i18n.t('Saved fingerprint')}<code>{hostPrompt.key.previousFingerprint}</code>
      </div>{/if}{#if !hostPrompt.key.changed}<label class="checkbox-field verify-checkbox"
        ><input type="checkbox" bind:checked={hostVerified} /><span
          >{i18n.t('I have verified the fingerprint through a trusted channel.')}</span
        ></label
      >{:else}<div class="form-error">
        {i18n.t(
          'Connection blocked. Contact the administrator. A changed key cannot be accepted from this dialog.',
        )}
      </div>{/if}{#snippet footer()}<button
        class="button secondary"
        disabled={hostBusy}
        onclick={() => (hostPrompt = null)}>{i18n.t('Close')}</button
      >{#if !hostPrompt?.key.changed}<button
          class="button primary"
          disabled={!hostVerified || hostBusy}
          onclick={trustHost}
          >{#if hostBusy}<LoaderCircle size={15} class="spin" />{:else}<ShieldCheck
              size={15}
            />{/if}{i18n.t('Trust and connect')}</button
        >{/if}{/snippet}</Modal
  >{/if}

{#if loginPrompt}<Modal
    title={i18n.t('Connect to {name}', { name: loginPrompt.connection.name })}
    eyebrow={i18n.t('AUTHENTICATION')}
    small
    busy={loginBusy}
    onClose={() => {
      loginPrompt = null;
      loginSecret = '';
    }}
    ><form id="login-form" onsubmit={submitLogin}>
      <p class="dialog-description">
        {loginPrompt.connection.username}@{loginPrompt.connection.host}
      </p>
      <label class="field"
        >{i18n.t(
          loginPrompt.connection.authMethod === 'keyFile'
            ? 'SSH key passphrase'
            : 'Password',
        )}<input
          data-initial-focus
          type="password"
          bind:value={loginSecret}
          autocomplete="current-password"
        /></label
      >
      <p class="field-help">
        {i18n.t(
          'Used only for this connection. To save it, edit the profile and enable the secure credential store.',
        )}
      </p>
      {#if loginPrompt.error}<div class="form-error" role="alert">
          {i18n.text(loginPrompt.error)}
        </div>{/if}
    </form>
    {#snippet footer()}<button
        class="button ghost"
        onclick={() => {
          loginPrompt = null;
          loginSecret = '';
        }}>{i18n.t('Cancel')}</button
      ><button type="submit" form="login-form" class="button primary" disabled={loginBusy}
        ><LockKeyhole size={15} />{i18n.t('Connect')}</button
      >{/snippet}</Modal
  >{/if}

{#if confirmAction}<Modal
    title={i18n.text(confirmAction.title)}
    eyebrow={i18n.t('CONFIRMATION')}
    small
    busy={actionBusy}
    onClose={() => (confirmAction = null)}
    ><p class="dialog-description">{i18n.text(confirmAction.description)}</p>
    {#if modalError}<div class="form-error" role="alert">
        {i18n.text(modalError)}
      </div>{/if}{#snippet footer()}<button
        class="button secondary"
        disabled={actionBusy}
        onclick={() => (confirmAction = null)}>{i18n.t('Cancel')}</button
      ><button
        class="button"
        class:danger={confirmAction?.danger}
        class:primary={!confirmAction?.danger}
        disabled={actionBusy}
        onclick={runConfirmation}
        >{#if actionBusy}<LoaderCircle size={15} class="spin" />{/if}{confirmAction
          ? i18n.text(confirmAction.label)
          : ''}</button
      >{/snippet}</Modal
  >{/if}

{#if textAction}<Modal
    title={i18n.t(textAction.kind === 'mkdir' ? 'A new folder.' : 'A new name.')}
    eyebrow={i18n.t(textAction.location === 'remote' ? 'REMOTE FILES' : 'Local files')}
    small
    busy={textBusy}
    onClose={() => (textAction = null)}
    ><form id="text-action" onsubmit={submitText}>
      <label class="field"
        >{i18n.t(textAction.kind === 'mkdir' ? 'Folder name' : 'Item name')}<input
          data-initial-focus
          required
          maxlength="255"
          pattern="[^/\\\\]+"
          bind:value={textAction.value}
          placeholder={i18n.t('Name')}
        /></label
      >{#if modalError}<div class="form-error" role="alert">{i18n.text(modalError)}</div>{/if}
    </form>
    {#snippet footer()}<button
        class="button secondary"
        disabled={textBusy}
        onclick={() => (textAction = null)}>{i18n.t('Cancel')}</button
      ><button class="button primary" type="submit" form="text-action" disabled={textBusy}
        >{#if textBusy}<LoaderCircle size={15} class="spin" />{/if}{i18n.t(
          textAction?.kind === 'mkdir' ? 'Create folder' : 'Rename',
        )}</button
      >{/snippet}</Modal
  >{/if}

{#if properties}<Modal
    title={i18n.t('Properties')}
    eyebrow={i18n.t(properties.location === 'remote' ? 'Remote files' : 'Local files')}
    small
    onClose={() => (properties = null)}
  >
    <div class="properties-grid">
      <span>{i18n.t('Name')}</span><strong>{properties.entry.name}</strong>
      <span>{i18n.t('Type')}</span><strong
        >{i18n.t(
          properties.entry.kind === 'directory'
            ? 'Folder'
            : properties.entry.kind === 'file'
              ? 'File'
              : properties.entry.kind === 'symlink'
                ? 'Symbolic link'
                : 'Special item',
        )}</strong
      >
      <span>{i18n.t('Path')}</span><strong class="properties-path">{properties.entry.path}</strong>
      <span>{i18n.t('Size')}</span><strong
        >{properties.entry.kind === 'directory'
          ? '—'
          : formatBytes(properties.entry.size, i18n.locale)}</strong
      >
      <span>{i18n.t('Modified')}</span><strong
        >{formatDate(properties.entry.modified, i18n.locale)}</strong
      >
      <span>{i18n.t('Permissions')}</span><strong
        >{formatPermissions(properties.entry.permissions)}</strong
      >
    </div>
    <p class="field-help">
      {i18n.t('Listing data; it may have changed since the last refresh.')}
    </p>
    {#snippet footer()}<span></span><button
        class="button primary"
        onclick={() => (properties = null)}>{i18n.t('Close')}</button
      >{/snippet}
  </Modal>{/if}

{#if about}<Modal
    title={i18n.t('Small on the outside. Secure on the inside.')}
    eyebrow={`SUPER FTP · ${appVersion} ${i18n.t('PREVIEW')}`}
    small
    onClose={() => (about = false)}
    ><p class="dialog-description">
      {i18n.t(
        'An SFTP client built with Rust, Tauri and the system webview. No Electron or telemetry. Profiles stay on your device; secrets stay in the system credential store.',
      )}
    </p>
    <div class="about-features">
      <span><ShieldCheck size={16} />{i18n.t('Server host key verification')}</span><span
        ><KeyRound size={16} />{i18n.t('Password, private key or SSH agent')}</span
      ><span><ArrowUp size={16} />{i18n.t('3 concurrent transfers overall, 2 per session')}</span>
    </div>
    <h3 class="shortcuts-heading"><Keyboard size={16} />{i18n.t('A little faster')}</h3>
    <div class="shortcut-list">
      <span>{i18n.t('New connection')}</span><kbd>Ctrl / ⌘ N</kbd><span>{i18n.t('New session')}</span
      ><kbd>Ctrl / ⌘ K</kbd><span>{i18n.t('Edit path')}</span><kbd>Ctrl / ⌘ L</kbd><span
        >{i18n.t('Hidden files')}</span
      ><kbd>Ctrl / ⌘ H</kbd><span>{i18n.t('Refresh panes')}</span><kbd>F5</kbd><span
        >{i18n.t('Select all')}</span
      ><kbd>Ctrl / ⌘ A</kbd>
    </div>
    <div class="form-note">
      {i18n.t(
        'Symbolic links and special files are skipped during recursive transfers. Overwriting requires confirmation. Folders can be deleted with all their contents.',
      )}
    </div>
    <div class="help-links">
      <button class="button secondary" onclick={() => { about = false; licensesOpen = true; void loadThirdPartyNotices(); }}>{i18n.t('Third-party licenses')}</button>
    </div>
    {#snippet footer()}<span class="muted">{i18n.t('Made to move files. Nothing more.')}</span><button
        class="button primary"
        onclick={() => (about = false)}>{i18n.t('Got it')}<Check size={15} /></button
      >{/snippet}</Modal
  >{/if}

{#if preferencesOpen}<Modal title={i18n.t('Preferences')} small onClose={() => (preferencesOpen = false)}>
    <div class="preferences-section">
      <strong>{i18n.t('Language')}</strong>
      <LanguageSelect />
    </div>
    <div class="preferences-section">
      <strong>{i18n.t('Change theme')}</strong>
      <button class="icon-button" aria-label={i18n.t(theme === 'dark' ? 'Switch to light theme' : 'Switch to dark theme')} title={i18n.t('Change theme')} onclick={() => (theme = theme === 'dark' ? 'light' : 'dark')}>{#if theme === 'dark'}<Sun size={17} />{:else}<Moon size={17} />{/if}</button>
    </div>
    <div class="preferences-section">
      <strong>{i18n.t('Interface text size')}</strong>
      <div class="font-size-control" role="group" aria-label={i18n.t('Interface text size')}>
        <button type="button" aria-label={i18n.t('Decrease text size')} title={i18n.t('Decrease text size')} disabled={fontSize === fontSizes[0]} onclick={() => adjustFontSize(-1)}>A−</button>
        <button type="button" class="font-size-reset" aria-label={i18n.t('Reset text size')} title={i18n.t('Reset text size')} onclick={() => fontSize = 100}>{fontSize}%</button>
        <button type="button" aria-label={i18n.t('Increase text size')} title={i18n.t('Increase text size')} disabled={fontSize === fontSizes[fontSizes.length - 1]} onclick={() => adjustFontSize(1)}>A+</button>
      </div>
    </div>
    <section class="update-check" aria-label={i18n.t('Updates')}>
      <strong class="update-heading">{i18n.t('Updates')}</strong>
      <p>{i18n.t('Checking contacts GitHub without credentials. Nothing is downloaded or installed.')}</p>
      <label><input type="checkbox" bind:checked={includePrereleases} disabled={checkingUpdates || installingUpdate} onchange={() => { updateResult = null; signedUpdateVersion = null; automaticUpdateNotice = null; updateError = ''; }} /> {i18n.t('Include preview releases')}</label>
      <div class="update-actions">
        <button class="button secondary" disabled={checkingUpdates || installingUpdate} onclick={checkUpdates}>{i18n.t('Check for updates')}</button>
        {#if signedUpdateVersion}<button class="button primary" disabled={installingUpdate} onclick={installSignedUpdate}>{i18n.t('Install update')}</button>{/if}
        <button class="button ghost" onclick={openReleasesPage}>{i18n.t('Open GitHub releases')}</button>
      </div>
      {#if updateResult}
        <p role="status">{#if !updateResult.latestVersion}{i18n.t('No matching public releases were found.')}{:else if updateResult.updateAvailable}{i18n.t('A newer version is available: {version}', { version: updateResult.latestVersion })}{#if updateResult.prerelease} · {i18n.t('Preview release')}{/if}{:else}{i18n.t('No newer version was found.')}{/if}</p>
        {#if updateResult.updateAvailable && updateResult.releaseNotes}<div class="update-notes"><strong>{i18n.t('Release notes')}</strong><pre>{updateResult.releaseNotes}</pre><p>{i18n.t('Notes may be shortened; see GitHub for the full text.')}</p></div>{/if}
      {/if}
      {#if updateError}<p class="form-error" role="alert">{i18n.text(updateError)}</p>{/if}
    </section>
    {#snippet footer()}<span></span><button class="button primary" onclick={() => (preferencesOpen = false)}>{i18n.t('Close')}</button>{/snippet}
  </Modal>{/if}

{#if licensesOpen}<Modal title={i18n.t('Third-party licenses')} onClose={() => (licensesOpen = false)}>
    <div class="third-party-notices">
      <p>{i18n.t('Third-party license notices (Windows snapshot)')}</p>
      <!-- The scrollable region needs keyboard focus for users without a mouse. -->
      <!-- svelte-ignore a11y_no_noninteractive_tabindex -->
      <div class="third-party-notices-text" role="region" aria-label={i18n.t('Third-party license notices (Windows snapshot)')} tabindex="0"><pre>{thirdPartyNotices}</pre></div>
    </div>
    {#snippet footer()}<span></span><button class="button primary" onclick={() => (licensesOpen = false)}>{i18n.t('Close')}</button>{/snippet}
  </Modal>{/if}

<div class="toast-stack" aria-live="polite">
  {#if automaticUpdateNotice}
    <div class="toast">
      <CircleHelp size={18} />
      <span>{i18n.t('A newer version is available: {version}', { version: automaticUpdateNotice })}</span>
      {#if signedUpdateVersion === automaticUpdateNotice}<button class="button primary" disabled={installingUpdate} onclick={installSignedUpdate}>{i18n.t('Install update')}</button>{/if}
      <button class="button ghost" onclick={() => { automaticUpdateNotice = null; preferencesOpen = true; }}>{i18n.t('Updates')}</button>
      <button class="icon-button tiny" aria-label={i18n.t('Dismiss notification')} onclick={() => automaticUpdateNotice = null}><X size={14} /></button>
    </div>
  {/if}
  {#each notices as notice (notice.id)}<div
      class="toast"
      class:error-toast={notice.kind === 'error'}
    >
      {#if notice.kind === 'error'}<ShieldAlert
          size={18}
        />{:else if notice.kind === 'success'}<ShieldCheck size={18} />{:else}<CircleHelp
          size={18}
        />{/if}<span>{i18n.text(notice.message)}</span><button
        class="icon-button tiny"
        aria-label={i18n.t('Dismiss notification')}
        onclick={() => (notices = notices.filter((item) => item.id !== notice.id))}
        ><X size={14} /></button
      >
    </div>{/each}
</div>

{#if waitingConflict && !editor && !picker && !hostPrompt && !loginPrompt && !confirmAction && !textAction && !properties && !about && !preferencesOpen && !licensesOpen}
  {#key waitingConflict.conflict?.id}<ConflictDialog transfer={waitingConflict} />{/key}
{/if}
