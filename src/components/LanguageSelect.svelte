<script lang="ts">
  import { Languages } from '@lucide/svelte';
  import { i18n, languages } from '../lib/i18n.svelte';
</script>

<label class="language-control" title={i18n.t('Change language')}>
  <Languages size={15} aria-hidden="true" />
  <select
    aria-label={i18n.t('Language')}
    value={i18n.language}
    onchange={(event) => i18n.setLanguage(event.currentTarget.value)}
  >
    {#each languages as language}
      <option value={language.id} lang={language.id}>{language.name}</option>
    {/each}
  </select>
</label>
