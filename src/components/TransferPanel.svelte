<script lang="ts">
  import {
    ArrowDown,
    ArrowUp,
    ArrowUpDown,
    CheckCheck,
    ChevronDown,
    ChevronUp,
    CircleCheck,
    CircleX,
    FolderSync,
    ListFilter,
    LoaderCircle,
    RotateCcw,
    ShieldCheck,
    X,
  } from '@lucide/svelte';
  import type { Transfer } from '../lib/types';
  import { formatBytes, isActive, progress } from '../lib/format';
  import { i18n } from '../lib/i18n.svelte';
  import { errorMessage } from '../lib/api';

  let {
    transfers,
    onCancel,
    onRetry,
    onClear,
  }: {
    transfers: Transfer[];
    onCancel: (id: string) => void;
    onRetry: (transfer: Transfer) => void;
    onClear: () => void;
  } = $props();
  let collapsed = $state(false);
  let filter = $state('all');
  const activeCount = $derived(transfers.filter(isActive).length);
  const failedCount = $derived(transfers.filter((t) => t.status === 'failed').length);
  const completedCount = $derived(transfers.filter((t) => t.status === 'completed').length);
  const filtered = $derived(
    transfers.filter(
      (t) =>
        filter === 'all' ||
        (filter === 'active'
          ? isActive(t)
          : filter === 'failed'
            ? t.status === 'failed'
            : t.status === 'completed'),
    ),
  );
  const statuses = {
    queued: 'Queued',
    preparing: 'Preparing…',
    transferring: 'Transferring',
    awaitingDecision: 'Waiting for a decision',
    completed: 'Completed',
    failed: 'Error',
    cancelled: 'Cancelled',
  } as const;
</script>

<section class="transfer-panel" class:collapsed aria-label={i18n.t('Transfer queue')}>
  <div class="transfer-heading">
    <button
      class="transfer-title"
      onclick={() => (collapsed = !collapsed)}
      aria-expanded={!collapsed}
      ><ArrowUpDown size={16} />
      <h2>{i18n.t('Transfers')}</h2>
      <span class="count-badge">{i18n.number(activeCount)}</span>{#if collapsed}<ChevronUp
          size={14}
        />{:else}<ChevronDown size={14} />{/if}</button
    ><span class="transfer-policy"
      ><ShieldCheck size={13} />{i18n.t('Conflicts are confirmed before overwriting')}</span
    ><button
      class="text-button clear-button"
      disabled={!transfers.some((t) => !isActive(t))}
      onclick={onClear}><CheckCheck size={14} />{i18n.t('Clear')}</button
    >
  </div>
  {#if !collapsed}
    <div class="transfer-tabs">
      <button class:active={filter === 'all'} onclick={() => (filter = 'all')}
        >{i18n.t('All')} <span>{i18n.number(transfers.length)}</span></button
      ><button class:active={filter === 'active'} onclick={() => (filter = 'active')}
        >{i18n.t('In progress')} <span>{i18n.number(activeCount)}</span></button
      ><button class:active={filter === 'completed'} onclick={() => (filter = 'completed')}
        >{i18n.t('Finished')} <span>{i18n.number(completedCount)}</span></button
      ><button class:active={filter === 'failed'} onclick={() => (filter = 'failed')}
        >{i18n.t('Failed')}
        <span class:has-errors={failedCount > 0}>{i18n.number(failedCount)}</span></button
      >
    </div>
    <div class="transfer-scroll">
      {#if filtered.length === 0}<div class="transfer-empty">
          <div class="transfer-empty-icon"><FolderSync size={21} strokeWidth={1.4} /></div>
          <div>
            <h3>
              {i18n.t(
                filter === 'failed'
                  ? 'All in order.'
                  : filter === 'completed'
                    ? 'No finished transfers yet'
                    : 'All up to date. Nothing pending.',
              )}
            </h3>
            <p>
              {i18n.t(
                'Drag files to the server or select them to start a transfer.',
              )}
            </p>
          </div>
          <span class="keyboard-hint"
            >{i18n.t('drag')} <span>&</span> {i18n.t('drop')}</span
          >
        </div>
      {:else}{#each filtered as transfer (transfer.id)}
          {@const transferError = transfer.error
            ? i18n.text(
                errorMessage({
                  code: 'TRANSFER',
                  message: transfer.error,
                  translation: transfer.errorTranslation,
                }),
              )
            : null}
          <div class="transfer-row" class:failed={transfer.status === 'failed'}>
            <span class="transfer-direction" class:download={transfer.direction === 'download'}
              >{#if transfer.direction === 'upload'}<ArrowUp size={15} />{:else}<ArrowDown
                  size={15}
                />{/if}</span
            >
            <div class="transfer-file">
              <strong title={transfer.source}>{transfer.name}</strong><span
                title={transferError ?? transfer.destination}
                >{transferError ?? transfer.destination}{#if transfer.skipped}
                  · {i18n.count(
                  '{count} item skipped',
                  '{count} items skipped',
                    transfer.skipped,
                  )}{/if}</span
              >
            </div>
            <div class="transfer-progress">
              <div>
                <span>{i18n.t(statuses[transfer.status])}</span><span
                  >{i18n.number(Math.floor(progress(transfer)))}%</span
                >
              </div>
              <div class="progress-track"><span style:width={`${progress(transfer)}%`}></span></div>
            </div>
            <div class="transfer-size">
              <span
                >{formatBytes(transfer.bytesTransferred, i18n.locale)} / {formatBytes(
                  transfer.totalBytes,
                  i18n.locale,
                )}</span
              ><span
                >{transfer.bytesPerSecond
                  ? `${formatBytes(transfer.bytesPerSecond, i18n.locale)}/s`
                  : i18n.t('{done} / {total} files', {
                      done: transfer.filesDone,
                      total: transfer.filesTotal,
                    })}</span
              >
            </div>
            <div class="transfer-row-action">
              {#if isActive(transfer)}<button
                  class="icon-button"
                  title={i18n.t('Cancel transfer')}
                  aria-label={i18n.t('Cancel {name}', { name: transfer.name })}
                  onclick={() => onCancel(transfer.id)}><X size={15} /></button
                >{:else if transfer.status === 'failed' || transfer.status === 'cancelled'}<button
                  class="icon-button"
                  title={i18n.t('Retry without overwriting files')}
                  aria-label={i18n.t('Retry {name}', { name: transfer.name })}
                  onclick={() => onRetry(transfer)}><RotateCcw size={15} /></button
                >{:else}<CircleCheck size={17} class="success-icon" />{/if}
            </div>
          </div>{/each}{/if}
    </div>
  {/if}
</section>
