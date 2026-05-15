<script lang="ts">
  import { settings, saveSettings } from '$lib/stores/sessions';
  import type { AppSettings } from '$lib/types';

  let { onClose }: { onClose: () => void } = $props();

  let draft = $state<AppSettings | null>(null);

  $effect(() => {
    if ($settings && !draft) draft = { ...$settings };
  });

  async function save() {
    if (!draft) return;
    await saveSettings(draft);
    onClose();
  }
</script>

{#if draft}
  <div class="overlay" role="dialog" aria-modal="true">
    <div class="modal">
      <h2>Settings</h2>

      <label>
        UDP Port
        <input type="number" bind:value={draft.port} min="1024" max="65535" />
      </label>

      <label>
        Units
        <select bind:value={draft.useMph}>
          <option value={true}>mph</option>
          <option value={false}>kph</option>
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
    background: #111827; border: 1px solid #374151; border-radius: 10px;
    padding: 1.5rem; width: 360px; display: flex; flex-direction: column; gap: 1rem;
  }
  h2 { margin: 0; color: #f9fafb; font-size: 1.1rem; }
  label { display: flex; flex-direction: column; gap: 0.3rem; color: #d1d5db; font-size: 0.85rem; }
  .checkbox-label { flex-direction: row; align-items: center; gap: 0.5rem; }
  input[type="number"], select {
    background: #1f2937; border: 1px solid #374151; border-radius: 4px;
    color: #f9fafb; padding: 0.4rem; font-size: 0.9rem;
  }
  fieldset { border: 1px solid #374151; border-radius: 6px; padding: 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
  legend { color: #9ca3af; font-size: 0.75rem; padding: 0 0.25rem; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
  button {
    padding: 0.4rem 1rem; border-radius: 5px; border: 1px solid #374151;
    background: #1f2937; color: #d1d5db; cursor: pointer; font-size: 0.85rem;
  }
  button.primary { background: #2563eb; border-color: #2563eb; color: white; }
  button:hover { filter: brightness(1.2); }
</style>
