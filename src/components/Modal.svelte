<script lang="ts">
  import { onMount, type Snippet } from 'svelte';
  import { X } from '@lucide/svelte';
  import { i18n } from '../lib/i18n.svelte';

  let {
    title,
    eyebrow = 'SUPER FTP',
    small = false,
    wide = false,
    busy = false,
    onClose,
    children,
    footer,
  }: {
    title: string;
    eyebrow?: string;
    small?: boolean;
    wide?: boolean;
    busy?: boolean;
    onClose: () => void;
    children: Snippet;
    footer?: Snippet;
  } = $props();
  let dialog: HTMLDialogElement;
  onMount(() => {
    dialog.showModal();
    dialog.querySelector<HTMLElement>('[data-initial-focus]')?.focus();
  });
</script>

<dialog
  class="modal"
  class:small-modal={small}
  class:wide-modal={wide}
  bind:this={dialog}
  aria-labelledby="modal-title"
  onclose={onClose}
  oncancel={(event) => {
    if (busy) event.preventDefault();
  }}
>
  <div class="modal-header">
    <div>
      <span class="eyebrow">{eyebrow}</span>
      <h2 id="modal-title">{title}</h2>
    </div>
    <button
      class="icon-button"
      aria-label={i18n.t('Close dialog')}
      disabled={busy}
      onclick={() => dialog.close()}><X size={19} /></button
    >
  </div>
  <div class="modal-body">{@render children()}</div>
  {#if footer}<div class="modal-footer">{@render footer()}</div>{/if}
</dialog>
