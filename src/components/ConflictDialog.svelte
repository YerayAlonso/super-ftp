<script lang="ts">
  import { untrack } from 'svelte';
  import { AlertTriangle, ArrowRight, LoaderCircle } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { api, errorMessage } from '../lib/api';
  import { i18n, type DisplayMessage } from '../lib/i18n.svelte';
  import { formatBytes, formatDate } from '../lib/format';
  import type { ConflictDecision, Transfer } from '../lib/types';

  let { transfer }: { transfer: Transfer } = $props();
  const conflict = untrack(() => transfer.conflict!);
  let newName = $state(conflict.name);
  let renaming = $state(false);
  let busy = $state(false);
  let error = $state<DisplayMessage>('');

  async function resolve(decision: ConflictDecision) {
    if (busy) return;
    busy = true;
    error = '';
    try { await api.resolveConflict(transfer.id, conflict.id, decision); }
    catch (cause) { error = errorMessage(cause); busy = false; }
  }
</script>

<Modal title={i18n.t('The file already exists')} eyebrow={i18n.t('TRANSFER CONFLICT')} {busy}
  onClose={() => { void resolve({ action: 'cancel' }); }}>
  <div class="conflict-intro"><AlertTriangle size={22} /><p>{i18n.t('A file already exists at the destination. What would you like to do?')}</p></div>
  <div class="conflict-details">
    <span>{i18n.t('Source')}</span><strong>{conflict.source}</strong>
    <span>{i18n.t('Destination')}</span><strong>{conflict.destination}</strong>
    <span>{i18n.t('New file')}</span><strong>{formatBytes(conflict.sourceSize, i18n.locale)}</strong>
    <span>{i18n.t('Existing file')}</span><strong>{conflict.destinationSize === null ? '—' : formatBytes(conflict.destinationSize, i18n.locale)}{#if conflict.destinationModified !== null} · {formatDate(conflict.destinationModified, i18n.locale)}{/if}</strong>
  </div>
  {#if renaming}
    <form id="conflict-rename" onsubmit={(event) => { event.preventDefault(); void resolve({ action: 'rename', name: newName }); }}>
      <label class="field">{i18n.t('New name')}<input required maxlength="255" bind:value={newName} /></label>
    </form>
  {/if}
  {#if error}<div class="form-error" role="alert">{i18n.text(error)}</div>{/if}
  <div class="conflict-actions">
    <button class="button secondary" disabled={busy} onclick={() => resolve({ action: 'skip' })}>{i18n.t('Skip this file')}</button>
    {#if !renaming}<button class="button secondary" disabled={busy} onclick={() => renaming = true}>{i18n.t('Rename the new file')}</button>{/if}
  </div>
  {#snippet footer()}
    <button class="button ghost" disabled={busy} onclick={() => resolve({ action: 'cancel' })}>{i18n.t('Cancel the entire transfer')}</button>
    {#if renaming}
      <button class="button primary" type="submit" form="conflict-rename" disabled={busy}>{#if busy}<LoaderCircle size={15} class="spin" />{:else}<ArrowRight size={15} />{/if}{i18n.t('Continue with this name')}</button>
    {:else}
      <button class="button danger" disabled={busy} onclick={() => resolve({ action: 'overwrite' })}>{#if busy}<LoaderCircle size={15} class="spin" />{/if}{i18n.t('Overwrite')}</button>
    {/if}
  {/snippet}
</Modal>
