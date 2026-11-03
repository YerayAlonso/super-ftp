<script lang="ts">
  import {
    ArrowDown,
    ArrowUp,
    ArrowUpDown,
    ChevronLeft,
    ChevronRight,
    CloudUpload,
    File,
    FileArchive,
    FileCode2,
    FileImage,
    FileText,
    Folder,
    FolderOpen,
    FolderPlus,
    Info,
    ListChecks,
    Home,
    Laptop,
    Link,
    LoaderCircle,
    Pencil,
    RefreshCw,
    Search,
    Server,
    Trash2,
    Upload,
    X,
  } from '@lucide/svelte';
  import type { FileEntry, PaneState } from '../lib/types';
  import { i18n } from '../lib/i18n.svelte';
  import {
    formatBytes,
    formatDate,
    formatPermissions,
    shortPath,
    sortEntries,
    type SortKey,
  } from '../lib/format';

  interface Props {
    kind: 'local' | 'remote';
    pane: PaneState;
    connected?: boolean;
    host?: string;
    showHidden: boolean;
    dropActive?: boolean;
    onNavigate: (path: string) => void;
    onBack: () => void;
    onForward: () => void;
    onHome: () => void;
    onRefresh: () => void;
    onSelectionChange: (entries: FileEntry[]) => void;
    onDismissError: () => void;
    onChoose?: () => void;
    onConnect?: () => void;
    onTransfer: () => void;
    onUploadFiles?: (directory: boolean) => void;
    onNewFolder?: () => void;
    onRename?: () => void;
    onDelete?: () => void;
    onProperties: (entry: FileEntry) => void;
    onInternalDrop?: (paths: string[]) => void;
  }
  let {
    kind,
    pane,
    connected = false,
    host = '',
    showHidden,
    dropActive = false,
    onNavigate,
    onBack,
    onForward,
    onHome,
    onRefresh,
    onSelectionChange,
    onDismissError,
    onChoose,
    onConnect,
    onTransfer,
    onUploadFiles,
    onNewFolder,
    onRename,
    onDelete,
    onProperties,
    onInternalDrop,
  }: Props = $props();

  let query = $state('');
  let pathInput = $state('');
  let sort = $state<SortKey>('name');
  let ascending = $state(true);
  let scrollTop = $state(0);
  let viewportHeight = $state(400);
  let viewport = $state<HTMLDivElement>();
  let anchor = $state<string | null>(null);
  let uploadMenu = $state(false);
  let contextMenu = $state<{
    x: number;
    y: number;
    entries: FileEntry[];
    isBackground: boolean;
  } | null>(null);
  let contextElement = $state<HTMLDivElement>();
  let drag = $state<{
    x: number;
    y: number;
    startX: number;
    startY: number;
    paths: string[];
    active: boolean;
  } | null>(null);
  let suppressClick = false;

  const rowHeight = 40;
  const remote = $derived(kind === 'remote');
  const ready = $derived(!remote || connected);
  const entries = $derived(
    sortEntries(pane.listing?.entries ?? [], query, showHidden, sort, ascending, i18n.locale),
  );
  const start = $derived(Math.max(0, Math.floor(scrollTop / rowHeight) - 8));
  const end = $derived(
    Math.min(entries.length, start + Math.ceil(viewportHeight / rowHeight) + 16),
  );
  const visibleEntries = $derived(entries.slice(start, end));
  const selectedPaths = $derived(new Set(pane.selected.map((entry) => entry.path)));

  $effect(() => {
    pathInput = pane.listing?.path ?? '';
  });
  $effect(() => {
    const path = pane.listing?.path;
    if (path && viewport) viewport.scrollTop = 0;
    anchor = null;
    contextMenu = null;
  });
  $effect(() => {
    if (!contextElement || !contextMenu) return;
    const menu = contextElement;
    const bounds = menu.getBoundingClientRect();
    menu.style.left = `${Math.max(8, Math.min(contextMenu.x, window.innerWidth - bounds.width - 8))}px`;
    menu.style.top = `${Math.max(8, Math.min(contextMenu.y, window.innerHeight - bounds.height - 8))}px`;
    menu.querySelector<HTMLButtonElement>('button:not(:disabled)')?.focus();
  });
  $effect(() => {
    if (!viewport) return;
    const observer = new ResizeObserver(([entry]) => (viewportHeight = entry.contentRect.height));
    observer.observe(viewport);
    return () => observer.disconnect();
  });

  function setSort(key: SortKey) {
    if (sort === key) ascending = !ascending;
    else {
      sort = key;
      ascending = true;
    }
  }

  function select(entry: FileEntry, event: MouseEvent | KeyboardEvent) {
    if (suppressClick) {
      suppressClick = false;
      return;
    }
    if (event.shiftKey && anchor) {
      const first = entries.findIndex((item) => item.path === anchor);
      const last = entries.findIndex((item) => item.path === entry.path);
      onSelectionChange(
        entries.slice(Math.min(Math.max(0, first), last), Math.max(first, last) + 1),
      );
    } else if (event.ctrlKey || event.metaKey) {
      onSelectionChange(
        selectedPaths.has(entry.path)
          ? pane.selected.filter((item) => item.path !== entry.path)
          : [...pane.selected, entry],
      );
      anchor = entry.path;
    } else {
      onSelectionChange([entry]);
      anchor = entry.path;
    }
  }

  function openEntry(entry: FileEntry) {
    if (entry.kind === 'directory' || entry.kind === 'symlink') onNavigate(entry.path);
    else {
      onSelectionChange([entry]);
      onTransfer();
    }
  }

  function fileIcon(entry: FileEntry) {
    if (entry.kind === 'directory') return Folder;
    if (entry.kind === 'symlink') return Link;
    if (/\.(png|jpg|jpeg|svg|webp|gif|avif|ico)$/i.test(entry.name)) return FileImage;
    if (/\.(zip|gz|tar|rar|7z|bz2)$/i.test(entry.name)) return FileArchive;
    if (/\.(js|ts|jsx|tsx|json|rs|py|css|html|php|sh|toml|yml)$/i.test(entry.name))
      return FileCode2;
    if (/\.(txt|md|log|csv|pdf)$/i.test(entry.name)) return FileText;
    return File;
  }

  function startDrag(entry: FileEntry, event: PointerEvent) {
    if (remote || event.button !== 0 || event.ctrlKey || event.metaKey || event.shiftKey) return;
    drag = {
      x: event.clientX,
      y: event.clientY,
      startX: event.clientX,
      startY: event.clientY,
      paths: selectedPaths.has(entry.path) ? pane.selected.map((item) => item.path) : [entry.path],
      active: false,
    };
  }

  function moveDrag(event: PointerEvent) {
    if (!drag) return;
    if (Math.hypot(event.clientX - drag.startX, event.clientY - drag.startY) > 7)
      drag.active = true;
    drag.x = event.clientX;
    drag.y = event.clientY;
  }

  function finishDrag(event: PointerEvent) {
    if (drag?.active) {
      suppressClick = true;
      // Suppress only the synthetic click belonging to this drag, never the
      // user's next real selection after dropping onto the other pane.
      setTimeout(() => {
        suppressClick = false;
      }, 0);
      if (
        document
          .elementFromPoint(event.clientX, event.clientY)
          ?.closest('[data-drop-zone="remote"]')
      )
        onInternalDrop?.(drag.paths);
    }
    drag = null;
  }

  function gridKeydown(event: KeyboardEvent) {
    if (contextMenu) return;
    if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'a') {
      event.preventDefault();
      onSelectionChange([...entries]);
    }
    if (event.key === 'Escape') onSelectionChange([]);
    if (event.key === 'Delete' && remote && pane.selected.length) {
      event.preventDefault();
      onDelete?.();
    }
    if (event.key === 'F2' && remote && pane.selected.length === 1) {
      event.preventDefault();
      onRename?.();
    }
  }

  function showContextMenu(event: MouseEvent) {
    event.preventDefault();
    event.stopPropagation();
    if (!pane.listing || pane.loading) return;
    const target = event.target instanceof Element ? event.target : null;
    const row = target?.closest<HTMLElement>('.file-row');
    // Column headers remain sort controls, not a folder background.
    if (!row && !target?.closest('.file-scroll')) return;
    const entry = row ? entries.find((item) => item.path === row.dataset.path) : undefined;
    if (row && !entry) return;
    const selection = entry ? (selectedPaths.has(entry.path) ? [...pane.selected] : [entry]) : [];
    onSelectionChange(selection);
    if (entry) anchor = entry.path;
    uploadMenu = false;
    const keyboard = event.detail === 0 && event.clientX === 0 && event.clientY === 0;
    const rect = (row ?? viewport)?.getBoundingClientRect();
    contextMenu = {
      x: keyboard && rect ? rect.left + 16 : event.clientX,
      y: keyboard && rect ? rect.top + 16 : event.clientY,
      entries: selection,
      isBackground: !entry,
    };
  }

  function dismissContext(event?: Event) {
    if (event?.target instanceof Node && contextElement?.contains(event.target)) return;
    contextMenu = null;
  }

  function menuKeydown(event: KeyboardEvent) {
    if (event.key === 'Escape') {
      event.preventDefault();
      event.stopPropagation();
      contextMenu = null;
      viewport?.focus();
      return;
    }
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
    event.preventDefault();
    const buttons = [
      ...(contextElement?.querySelectorAll<HTMLButtonElement>('button:not(:disabled)') ?? []),
    ];
    if (!buttons.length) return;
    const index = buttons.indexOf(document.activeElement as HTMLButtonElement);
    const next =
      event.key === 'Home'
        ? 0
        : event.key === 'End'
          ? buttons.length - 1
          : (index + (event.key === 'ArrowDown' ? 1 : -1) + buttons.length) % buttons.length;
    buttons[next].focus();
  }

  function runMenu(action: () => void) {
    contextMenu = null;
    action();
  }
</script>

<svelte:window
  onpointermove={moveDrag}
  onpointerup={finishDrag}
  onpointercancel={() => (drag = null)}
  onclick={() => (uploadMenu = false)}
  onpointerdown={dismissContext}
  onblur={() => (contextMenu = null)}
  onresize={() => (contextMenu = null)}
/>

<section
  class="file-pane"
  class:remote-pane={remote}
  class:drop-active={dropActive}
  data-drop-zone={remote ? 'remote' : 'local'}
  aria-label={i18n.t(remote ? 'Remote files' : 'Local files')}
>
  <div class="pane-heading">
    <div class:remote-icon={remote} class="pane-symbol">
      {#if remote}<Server size={17} />{:else}<Laptop size={18} />{/if}
    </div>
    <div class="pane-heading-text">
      <h2>{i18n.t(remote ? 'Remote server' : 'Your computer')}</h2>
      <span
        >{remote
          ? connected
            ? host
            : i18n.t('No active connection')
          : i18n.t('Local files')}</span
      >
    </div>
    {#if remote && connected}<span class="protocol-tag"><span></span>SFTP</span
      >{:else if !remote}<span class="subtle-label">{i18n.t('LOCAL')}</span>{/if}
    {#if ready}<button
        class="icon-button"
        title={i18n.t(remote ? 'Refresh folder' : 'Choose a local folder')}
        aria-label={i18n.t(remote ? 'Refresh remote folder' : 'Choose a local folder')}
        onclick={remote ? onRefresh : onChoose}
        >{#if remote}<RefreshCw size={16} class={pane.loading ? 'spin' : ''} />{:else}<FolderOpen
            size={17}
          />{/if}</button
      >{/if}
  </div>

  <div class="pane-navigation" class:disabled={!ready}>
    <div class="navigation-buttons">
      <button
        class="icon-button"
        aria-label={i18n.t('Previous folder')}
        title={i18n.t('Back')}
        disabled={!ready || pane.historyIndex <= 0}
        onclick={onBack}><ChevronLeft size={16} /></button
      >
      <button
        class="icon-button"
        aria-label={i18n.t('Next folder')}
        title={i18n.t('Forward')}
        disabled={!ready || pane.historyIndex >= pane.history.length - 1}
        onclick={onForward}><ChevronRight size={16} /></button
      >
      <button
        class="icon-button"
        aria-label={i18n.t('Parent folder')}
        title={i18n.t('Up one level')}
        disabled={!ready || !pane.listing?.parent}
        onclick={() => pane.listing?.parent && onNavigate(pane.listing.parent)}
        ><ArrowUp size={16} /></button
      >
    </div>
    <form
      class="path-field"
      onsubmit={(event) => {
        event.preventDefault();
        if (pathInput.trim()) onNavigate(pathInput.trim());
      }}
    >
      <button
        type="button"
        aria-label={i18n.t('Home folder')}
        title={i18n.t('Home folder')}
        disabled={!ready}
        onclick={onHome}><Home size={13} /></button
      >
      <input
        id={`${kind}-path`}
        aria-label={i18n.t(remote ? 'Remote path' : 'Local path')}
        bind:value={pathInput}
        placeholder={i18n.t(remote ? '/remote/path' : '/local/folder')}
        disabled={!ready}
        spellcheck="false"
        autocomplete="off"
      />
    </form>
  </div>

  {#if ready}
    <div class="pane-actions">
      <div class="pane-search">
        <Search size={14} /><input
          aria-label={i18n.t(remote ? 'Filter remote files' : 'Filter local files')}
          placeholder={i18n.t('Filter files…')}
          bind:value={query}
        />{#if query}<button
            class="icon-button tiny"
            aria-label={i18n.t('Clear filter')}
            onclick={() => (query = '')}><X size={13} /></button
          >{/if}
      </div>
      {#if remote}
        <button
          class="icon-button"
          title={i18n.t('Download selection')}
          aria-label={i18n.t('Download selected items')}
          disabled={!pane.selected.length}
          onclick={onTransfer}><ArrowDown size={16} /></button
        >
        <button
          class="icon-button"
          title={i18n.t('Create a folder')}
          aria-label={i18n.t('Create a remote folder')}
          onclick={onNewFolder}><FolderPlus size={16} /></button
        >
        <button
          class="icon-button"
          title={`${i18n.t('Rename')} (F2)`}
          aria-label={i18n.t('Rename selected item')}
          disabled={pane.selected.length !== 1}
          onclick={onRename}><Pencil size={14} /></button
        >
        <button
          class="icon-button danger-hover"
          title={i18n.t('Delete')}
          aria-label={i18n.t('Delete selected items')}
          disabled={!pane.selected.length}
          onclick={onDelete}><Trash2 size={15} /></button
        >
        <span class="action-divider"></span>
        <div class="upload-menu-wrap">
          <button
            class="button small secondary"
            onclick={(event) => {
              event.stopPropagation();
              uploadMenu = !uploadMenu;
            }}><Upload size={13} />{i18n.t('Upload')}</button
          >
          {#if uploadMenu}<div class="dropdown upload-dropdown">
              <button onclick={() => onUploadFiles?.(false)}
                ><File size={15} />{i18n.t('Upload files')}</button
              ><button onclick={() => onUploadFiles?.(true)}
                ><Folder size={15} />{i18n.t('Upload a folder')}</button
              >
            </div>{/if}
        </div>
      {:else}
        <button
          class="button small secondary"
          disabled={!pane.selected.length || !connected}
          onclick={onTransfer}
          title={i18n.t('Upload selection to server')}
          ><ArrowUp size={13} />{i18n.t('Upload selection')}</button
        >
      {/if}
    </div>
    {#if pane.error}<div class="pane-error" role="alert">
        <span>{i18n.text(pane.error)}</span><button
          class="icon-button tiny"
          aria-label={i18n.t('Dismiss error')}
          onclick={onDismissError}><X size={13} /></button
        >
      </div>{/if}
    <div
      class="file-grid"
      role="grid"
      aria-label={i18n.t(remote ? 'Remote listing' : 'Local listing')}
      aria-rowcount={entries.length + 1}
      onkeydown={gridKeydown}
      oncontextmenu={showContextMenu}
      tabindex="0"
    >
      <div class="file-grid-header" role="row">
        <button role="columnheader" onclick={() => setSort('name')}
          >{i18n.t('Name')}
          {#if sort === 'name'}{#if ascending}<ArrowUp size={11} />{:else}<ArrowDown
                size={11}
              />{/if}{/if}</button
        >
        <button role="columnheader" class="size-cell" onclick={() => setSort('size')}
          >{i18n.t('Size')}</button
        >
        <button role="columnheader" class="date-cell" onclick={() => setSort('modified')}
          >{i18n.t('Modified')}</button
        >
        {#if remote}<span role="columnheader" class="permissions-cell">{i18n.t('Perm.')}</span>{/if}
      </div>
      <div
        class="file-scroll"
        bind:this={viewport}
        onscroll={(event) => {
          scrollTop = event.currentTarget.scrollTop;
          contextMenu = null;
        }}
      >
        {#if pane.loading && !pane.listing}<div class="pane-placeholder">
            <LoaderCircle size={25} class="spin" /><span>{i18n.t('Reading folder…')}</span>
          </div>
        {:else if !pane.listing}<div class="pane-placeholder">
            <FolderOpen size={30} />
            <h3>{i18n.t(pane.error ? 'Unable to open folder' : 'Your files, within reach')}</h3>
            <p>
              {i18n.t(
                pane.error
                  ? 'Choose another folder to continue.'
                  : 'Open a local folder to get started.',
              )}
            </p>
            <button class="button secondary small" onclick={onChoose}
              ><FolderOpen size={14} />{i18n.t('Choose a folder')}</button
            >
          </div>
        {:else if entries.length === 0}<div class="pane-placeholder">
            <Search size={27} />
            <h3>{i18n.t(query ? 'No matching files' : 'A nice and quiet folder')}</h3>
            <p>
              {i18n.t(
                query
                  ? 'Try another name or show hidden files.'
                  : 'This folder is empty.',
              )}
            </p>
          </div>
        {:else}
          <div class="virtual-spacer" style:height={`${entries.length * rowHeight}px`}>
            <div class="virtual-rows" style:transform={`translateY(${start * rowHeight}px)`}>
              {#each visibleEntries as entry, index (entry.path)}
                {@const Icon = fileIcon(entry)}
                <div
                  class="file-row"
                  class:selected={selectedPaths.has(entry.path)}
                  class:folder-row={entry.kind === 'directory'}
                  class:hidden-file={entry.hidden}
                  role="row"
                  aria-selected={selectedPaths.has(entry.path)}
                  aria-rowindex={start + index + 2}
                  tabindex="0"
                  title={entry.name}
                  data-path={entry.path}
                  onclick={(event) => select(entry, event)}
                  ondblclick={() => openEntry(entry)}
                  onpointerdown={(event) => startDrag(entry, event)}
                  onkeydown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      openEntry(entry);
                    }
                    if (event.key === ' ') {
                      event.preventDefault();
                      select(entry, event);
                    }
                  }}
                >
                  <span class="name-cell" role="gridcell"
                    ><Icon size={17} strokeWidth={1.6} /><span>{entry.name}</span></span
                  >
                  <span class="size-cell" role="gridcell"
                    >{entry.kind === 'directory' ? '—' : formatBytes(entry.size, i18n.locale)}</span
                  >
                  <span class="date-cell" role="gridcell"
                    >{formatDate(entry.modified, i18n.locale)}</span
                  >
                  {#if remote}<span class="permissions-cell" role="gridcell"
                      >{formatPermissions(entry.permissions)}</span
                    >{/if}
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </div>
    </div>
    <div class="pane-footer">
      <span
        >{i18n.count(
          '{count} item',
          '{count} items',
          entries.length,
        )}{#if pane.selected.length}<span class="footer-dot">·</span>{i18n.count(
            '{count} item selected',
            '{count} selected',
            pane.selected.length,
          )}{/if}</span
      >{#if pane.loading}<LoaderCircle size={13} class="spin" />{:else}<span class="pane-tip"
          >{i18n.t(remote ? 'Double-click to open or download' : 'Drag to upload')}</span
        >{/if}
    </div>
  {:else}
    <div class="remote-empty">
      <div class="remote-illustration" aria-hidden="true">
        <div class="orbit orbit-one"></div>
        <div class="orbit orbit-two"></div>
        <span class="orbit-dot dot-one"></span><span class="orbit-dot dot-two"></span>
        <div class="cloud-card"><CloudUpload size={38} strokeWidth={1.2} /><span>SFTP</span></div>
        <span class="floating-file"><FileCode2 size={22} strokeWidth={1.5} /></span><span
          class="floating-folder"><Folder size={22} strokeWidth={1.5} /></span
        >
      </div>
      <span class="eyebrow">{i18n.t('FROM LOCAL TO ANYWHERE')}</span>
      <h3>{i18n.t('Another server.')}<br />{i18n.t('The same space.')}</h3>
      <p>
        {i18n.t(
          'Connect to your server and move files with the peace of mind of a secure connection.',
        )}
      </p>
      <button class="button primary" onclick={onConnect}
        ><Link size={15} />{i18n.t('Connect a server')}</button
      >
      <div class="empty-security">
        <span class="tiny-dot"></span>{i18n.t('End-to-end encryption over SSH')}
      </div>
    </div>
    <div class="pane-footer">
      <span>{i18n.t('Waiting for a connection')}</span><span class="pane-tip">SSH / SFTP</span>
    </div>
  {/if}
  {#if dropActive && remote}<div class="drop-overlay">
      <div>
        <CloudUpload size={38} />
        <h3>{i18n.t('Drop them here. We’ll upload them.')}</h3>
        <p>{i18n.t('To {path}', { path: shortPath(pane.listing?.path ?? '/') })}</p>
      </div>
    </div>{/if}
</section>

{#if contextMenu}
  <div
    class="context-menu dropdown"
    role="menu"
    tabindex="-1"
    aria-label={i18n.t('Context menu')}
    bind:this={contextElement}
    style:left={`${contextMenu.x}px`}
    style:top={`${contextMenu.y}px`}
    onkeydown={menuKeydown}
    oncontextmenu={(event) => event.preventDefault()}
  >
    {#if contextMenu.isBackground}
      <button role="menuitem" onclick={() => runMenu(() => onNewFolder?.())}
        ><FolderPlus size={15} />{i18n.t('Create a folder')}</button
      >
      <button role="menuitem" onclick={() => runMenu(onRefresh)}
        ><RefreshCw size={15} />{i18n.t('Refresh folder')}</button
      >
      <button
        role="menuitem"
        disabled={!entries.length}
        onclick={() => runMenu(() => onSelectionChange([...entries]))}
        ><ListChecks size={15} />{i18n.t('Select all')}</button
      >
      {#if remote}<div class="context-separator"></div>
        <button role="menuitem" onclick={() => runMenu(() => onUploadFiles?.(false))}
          ><Upload size={15} />{i18n.t('Upload files')}</button
        >{/if}
    {:else}
      {#if contextMenu.entries.length === 1 && (contextMenu.entries[0].kind === 'directory' || contextMenu.entries[0].kind === 'symlink')}
        <button
          role="menuitem"
          onclick={() => {
            const entry = contextMenu!.entries[0];
            runMenu(() => onNavigate(entry.path));
          }}><FolderOpen size={15} />{i18n.t('Open')}</button
        >
      {/if}
      <button role="menuitem" disabled={!connected} onclick={() => runMenu(onTransfer)}
        >{#if remote}<ArrowDown size={15} />{:else}<ArrowUp size={15} />{/if}{i18n.t(
          remote ? 'Download selection' : 'Upload selection',
        )}</button
      >
      {#if remote && contextMenu.entries.length === 1}<button
          role="menuitem"
          onclick={() => runMenu(() => onRename?.())}
          ><Pencil size={15} />{i18n.t('Rename')}</button
        >{/if}
      <button role="menuitem" class="danger-text" onclick={() => runMenu(() => onDelete?.())}
        ><Trash2 size={15} />{i18n.t('Delete')}</button
      >
      {#if contextMenu.entries.length === 1}<div class="context-separator"></div>
        <button
          role="menuitem"
          onclick={() => {
            const entry = contextMenu!.entries[0];
            runMenu(() => onProperties(entry));
          }}><Info size={15} />{i18n.t('Properties')}</button
        >{/if}
    {/if}
  </div>
{/if}

{#if drag?.active}<div
    class="drag-badge"
    style:left={`${drag.x + 15}px`}
    style:top={`${drag.y + 15}px`}
  >
    <Upload size={15} />{i18n.count('{count} item', '{count} items', drag.paths.length)}
  </div>{/if}
