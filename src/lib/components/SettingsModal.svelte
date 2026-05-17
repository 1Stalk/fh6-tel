<script lang="ts">
  import { settings, saveSettings } from '$lib/stores/sessions';
  import { invoke } from '@tauri-apps/api/core';
  import { carName } from '$lib/car-name';
  import type { AppSettings } from '$lib/types';

  let { onClose }: { onClose: () => void } = $props();

  let draft = $state<AppSettings | null>(null);
  let exportMsg = $state('');

  $effect(() => {
    if ($settings && !draft) draft = { ...$settings };
  });

  async function save() {
    if (!draft) return;
    await saveSettings(draft);
    onClose();
  }

  async function exportOrdinals() {
    const ordinals: number[] = await invoke('get_session_car_ordinals');
    if (ordinals.length === 0) { exportMsg = 'No sessions recorded yet.'; return; }
    const lines = ordinals.map(o => {
      const name = carName(o);
      return `${o}: ${name.startsWith('Car #') ? '(unknown)' : name}`;
    });
    await navigator.clipboard.writeText(lines.join('\n'));
    exportMsg = `Copied ${ordinals.length} ordinal${ordinals.length !== 1 ? 's' : ''} to clipboard`;
    setTimeout(() => { exportMsg = ''; }, 3000);
  }
</script>

{#if draft}
  <div class="overlay" role="dialog" aria-modal="true">
    <div class="modal">
      <h2>Settings</h2>

      <label>
        UDP Port
        <input type="number" bind:value={draft.port} min="1024" max="65535" />
        <span class="hint">Port changes take effect after restarting the app.</span>
      </label>

      <label>
        Units
        <select bind:value={draft.useMph}>
          <option value={true}>mph</option>
          <option value={false}>kph</option>
        </select>
      </label>

      <label>
        Theme
        <select bind:value={draft.theme}>
          <option value="dark">Dark</option>
          <option value="cobalt2">Cobalt2</option>
          <option value="purple">Purple</option>
        </select>
      </label>

      <label class="checkbox-label">
        <input type="checkbox" bind:checked={draft.autoRecord} />
        Auto-record sessions
      </label>

      <fieldset>
        <legend>Tire Temp Thresholds (°C)</legend>
        <label>Cold below <input type="number" bind:value={draft.tireTempCold} /></label>
        <label>Optimal up to <input type="number" bind:value={draft.tireTempOptimal} /></label>
        <label>Hot above <input type="number" bind:value={draft.tireTempHot} /></label>
      </fieldset>

      <div class="export-row">
        <button onclick={exportOrdinals}>Export car ordinals</button>
        {#if exportMsg}<span class="hint">{exportMsg}</span>{/if}
      </div>

      <div class="actions">
        <button onclick={onClose}>Cancel</button>
        <button class="primary" onclick={save}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center; z-index: 100;
  }
  .modal {
    background: var(--bg-elevated); border: 1px solid var(--bd-muted); border-radius: 10px;
    padding: 1.5rem; width: 360px; display: flex; flex-direction: column; gap: 1rem;
  }
  h2 { margin: 0; color: var(--tx-hi); font-size: 1.1rem; }
  label { display: flex; flex-direction: column; gap: 0.3rem; color: var(--tx-mid); font-size: 0.85rem; }
  .checkbox-label { flex-direction: row; align-items: center; gap: 0.5rem; }
  input[type="number"], select {
    background: var(--bg-body); border: 1px solid var(--bd-muted); border-radius: 4px;
    color: var(--tx-hi); padding: 0.4rem; font-size: 0.9rem;
  }
  fieldset { border: 1px solid var(--bd-muted); border-radius: 6px; padding: 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
  legend { color: var(--tx-lo); font-size: 0.75rem; padding: 0 0.25rem; }
  .export-row { display: flex; align-items: center; gap: 0.75rem; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
  button {
    padding: 0.4rem 1rem; border-radius: 5px; border: 1px solid var(--bd-muted);
    background: var(--bg-elevated); color: var(--tx-mid); cursor: pointer; font-size: 0.85rem;
  }
  button.primary { background: var(--ac); border-color: var(--ac); color: var(--bg-body); }
  button:hover { filter: brightness(1.2); }
  .hint { font-size: 0.7rem; color: var(--tx-dim); margin-top: 0.15rem; }
</style>
