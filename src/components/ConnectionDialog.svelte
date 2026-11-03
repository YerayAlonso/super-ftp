<script lang="ts">
  import { untrack } from 'svelte';
  import {
    ArrowUpRight,
    Check,
    Eye,
    EyeOff,
    FileKey,
    FolderOpen,
    KeyRound,
    LoaderCircle,
    LockKeyhole,
    ShieldCheck,
  } from '@lucide/svelte';
  import Modal from './Modal.svelte';
  import { COLORS, newConnection, type AuthMethod, type Connection } from '../lib/types';
  import { errorMessage, chooseFiles, chooseKey } from '../lib/api';
  import { i18n, type DisplayMessage, type MessageKey } from '../lib/i18n.svelte';

  let {
    connection,
    onClose,
    onSave,
  }: {
    connection: Connection | null;
    onClose: () => void;
    onSave: (connection: Connection, secret: string | null, connect: boolean) => Promise<void>;
  } = $props();
  const initial = untrack(() => connection);
  let draft = $state<Connection>({ ...(initial ?? newConnection()) });
  let secret = $state('');
  let showSecret = $state(false);
  let busy = $state(false);
  let error = $state<DisplayMessage>('');
  let advanced = $state(
    Boolean(initial?.localPath || (initial?.remotePath && initial.remotePath !== '.')),
  );

  const authOptions: { value: AuthMethod; label: MessageKey; icon: typeof KeyRound }[] = [
    { value: 'password', label: 'Password', icon: LockKeyhole },
    { value: 'keyFile', label: 'SSH key', icon: FileKey },
    { value: 'agent', label: 'SSH agent', icon: KeyRound },
  ];

  async function pickKey() {
    try {
      const path = await chooseKey();
      if (path) draft.keyPath = path;
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
  async function pickLocal() {
    try {
      const [path] = await chooseFiles(true);
      if (path) draft.localPath = path;
    } catch (cause) {
      error = errorMessage(cause);
    }
  }
  async function submit(event: SubmitEvent) {
    event.preventDefault();
    if (busy) return;
    if (draft.authMethod === 'keyFile' && !draft.keyPath) {
      error = 'Select the private SSH key file.';
      return;
    }
    busy = true;
    error = '';
    try {
      await onSave(
        {
          ...draft,
          port: Number(draft.port),
          rememberSecret: draft.authMethod !== 'agent' && draft.rememberSecret,
        },
        secret || null,
        (event.submitter as HTMLButtonElement)?.value === 'connect',
      );
      secret = '';
    } catch (cause) {
      error = errorMessage(cause);
    } finally {
      busy = false;
    }
  }
</script>

<Modal
  title={i18n.t(connection ? 'Edit connection' : 'A new destination for your files.')}
  eyebrow={i18n.t(connection ? 'SAVED CONNECTION' : 'NEW CONNECTION')}
  {busy}
  {onClose}
>
  <form id="connection-form" class="connection-form" onsubmit={submit}>
    <p class="form-intro">{i18n.t('Your server details, all in one place. Always within reach.')}</p>
    <label class="field"
      >{i18n.t('Connection name')}<input
        data-initial-focus
        required
        maxlength="100"
        placeholder={i18n.t('e.g. Production website')}
        bind:value={draft.name}
      /></label
    >
    <div class="form-row host-row">
      <label class="field"
        >{i18n.t('Server')}<input
          required
          maxlength="253"
          placeholder="sftp.exemple.cat"
          bind:value={draft.host}
          spellcheck="false"
          autocapitalize="off"
        /></label
      ><label class="field port-field"
        >{i18n.t('Port')}<input
          type="number"
          required
          min="1"
          max="65535"
          bind:value={draft.port}
        /></label
      >
    </div>
    <label class="field"
      >{i18n.t('Username')}<input
        required
        maxlength="255"
        placeholder="deploy"
        bind:value={draft.username}
        autocomplete="username"
        spellcheck="false"
      /></label
    >
    <fieldset class="auth-fieldset">
      <legend>{i18n.t('Authentication')}</legend>
      <div class="auth-options">
        {#each authOptions as option}<button
            type="button"
            class:active={draft.authMethod === option.value}
            onclick={() => {
              draft.authMethod = option.value;
              secret = '';
              draft.rememberSecret = false;
            }}><option.icon size={15} />{i18n.t(option.label)}</button
          >{/each}
      </div>
    </fieldset>
    {#if draft.authMethod === 'keyFile'}<label class="field"
        >{i18n.t('Private key')}
        <div class="input-with-button">
          <input
            required
            placeholder="~/.ssh/id_ed25519"
            bind:value={draft.keyPath}
            spellcheck="false"
          /><button
            type="button"
            class="icon-button"
            aria-label={i18n.t('Select private SSH key')}
            onclick={pickKey}><FolderOpen size={17} /></button
          >
        </div></label
      >{/if}
    {#if draft.authMethod !== 'agent'}
      <label class="field"
        >{i18n.t(draft.authMethod === 'password' ? 'Password' : 'Passphrase')}<span
          class="optional"
          >{i18n.t(
            draft.authMethod === 'keyFile' ? 'optional' : 'can be requested when connecting',
          )}</span
        >
        <div class="input-with-button">
          <input
            type={showSecret ? 'text' : 'password'}
            bind:value={secret}
            placeholder={connection?.rememberSecret
              ? i18n.t('Leave blank to keep the saved secret')
              : '••••••••••••'}
            autocomplete="new-password"
          /><button
            type="button"
            class="icon-button"
            aria-label={i18n.t(showSecret ? 'Hide secret' : 'Show secret')}
            onclick={() => (showSecret = !showSecret)}
            >{#if showSecret}<EyeOff size={16} />{:else}<Eye size={16} />{/if}</button
          >
        </div></label
      >
      <label class="checkbox-field"
        ><input type="checkbox" bind:checked={draft.rememberSecret} /><span
          >{i18n.t('Save the secret in the system credential store')}</span
        ></label
      >
    {:else}<div class="form-note">
        <KeyRound size={16} /><span
          >{i18n.t(
            'Your SSH agent’s keys will be used. The private key never leaves the agent. Windows requires the OpenSSH service; macOS and Linux use SSH_AUTH_SOCK.',
          )}</span
        >
      </div>{/if}
    <div class="connection-options">
      <div>
        <span class="field-label">{i18n.t('Colour')}</span>
        <div class="color-options">
          {#each COLORS as color}<button
              type="button"
              style:--swatch={color}
              class:chosen={draft.color === color}
              aria-label={i18n.t('Colour {color}', { color })}
              aria-pressed={draft.color === color}
              onclick={() => (draft.color = color)}
              >{#if draft.color === color}<Check size={13} />{/if}</button
            >{/each}
        </div>
      </div>
      <button type="button" class="text-button" onclick={() => (advanced = !advanced)}
        >{i18n.t(advanced ? 'Hide options' : 'Advanced options')}
        <span>{advanced ? '−' : '+'}</span></button
      >
    </div>
    {#if advanced}<div class="advanced-fields">
        <label class="field"
          >{i18n.t('Starting remote folder')}<input
            bind:value={draft.remotePath}
            placeholder={i18n.t('. (home folder)')}
            spellcheck="false"
          /></label
        ><label class="field"
          >{i18n.t('Starting local folder')}
          <div class="input-with-button">
            <input
              bind:value={draft.localPath}
              placeholder={i18n.t('Home folder')}
              spellcheck="false"
            /><button
              type="button"
              class="icon-button"
              aria-label={i18n.t('Select starting local folder')}
              onclick={pickLocal}><FolderOpen size={16} /></button
            >
          </div></label
        >
      </div>{/if}
    {#if error}<div class="form-error" role="alert">{i18n.text(error)}</div>{/if}
    <div class="security-note">
      <ShieldCheck size={16} /><span
        >{i18n.t('SFTP only. Verified identity. No plaintext secrets.')}</span
      >
    </div>
  </form>
  {#snippet footer()}<button class="button ghost" disabled={busy} onclick={onClose}
      >{i18n.t('Cancel')}</button
    >
    <div class="footer-actions">
      <button
        class="button secondary"
        type="submit"
        form="connection-form"
        value="save"
        disabled={busy}>{i18n.t('Save')}</button
      ><button
        class="button primary"
        type="submit"
        form="connection-form"
        value="connect"
        disabled={busy}
        >{#if busy}<LoaderCircle size={15} class="spin" />{:else}<ArrowUpRight
            size={16}
          />{/if}{i18n.t('Save and connect')}</button
      >
    </div>{/snippet}
</Modal>
