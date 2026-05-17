<script lang="ts">
  import { onMount } from 'svelte';
  import { listen } from '@tauri-apps/api/event';
  import { startTelemetryListener } from '$lib/stores/telemetry';
  import { loadSettings, settings } from '$lib/stores/sessions';
  import TopBar from '$lib/components/TopBar.svelte';
  import CompassBar from '$lib/components/CompassBar.svelte';
  import CenterPanel from '$lib/components/CenterPanel.svelte';
  import TireWidget from '$lib/components/TireWidget.svelte';
  import LapBar from '$lib/components/LapBar.svelte';
  import SessionDrawer from '$lib/components/SessionDrawer.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';

  let showSessions = $state(false);
  let showSettings = $state(false);
  let toasts = $state<{ id: number; message: string }[]>([]);
  let nextToastId = 0;

  function addToast(message: string) {
    const id = nextToastId++;
    toasts = [...toasts, { id, message }];
    setTimeout(() => { toasts = toasts.filter(t => t.id !== id); }, 4000);
  }

  onMount(async () => {
    await loadSettings();
    await startTelemetryListener();
    await listen('session_error', (e) => addToast(String(e.payload)));
    await listen('udp_bind_failed', (e) => addToast(String(e.payload)));
  });

  let s = $derived($settings);

  // Apply theme to <html> element whenever settings change
  $effect(() => {
    const theme = s?.theme ?? 'dark';
    document.documentElement.setAttribute('data-theme', theme);
  });
</script>

<div class="dashboard">
  <TopBar
    useMph={s?.useMph ?? true}
    onSettings={() => (showSettings = true)}
    onSessions={() => (showSessions = !showSessions)}
  />
  <CompassBar />

  <div class="main">
    <div class="center-area">
      <CenterPanel useMph={s?.useMph ?? true} />
    </div>

    <div class="right-strip">
      <TireWidget
        tireTempCold={s?.tireTempCold ?? 60}
        tireTempOptimal={s?.tireTempOptimal ?? 85}
        tireTempHot={s?.tireTempHot ?? 110}
      />
    </div>
  </div>

  <div class="lap-bar">
    <LapBar />
  </div>
</div>

{#if showSessions}
  <SessionDrawer onClose={() => (showSessions = false)} useMph={s?.useMph ?? true} />
{/if}

{#if toasts.length > 0}
  <div class="toast-stack">
    {#each toasts as toast (toast.id)}
      <div class="toast">{toast.message}</div>
    {/each}
  </div>
{/if}

{#if showSettings}
  <SettingsModal onClose={() => (showSettings = false)} />
{/if}

<style>
  /* ── Theme: CSS custom properties ───────────────────────────────────────── */
  :global(:root) {
    /* Dark (default) */
    --bg-body:    #030712;
    --bg-panel:   #060c14;
    --bg-card:    #080e18;
    --bg-elevated:#0d1420;
    --bg-track:   #151e2e;
    --bd-dim:     #131d2e;
    --bd-subtle:  #1e2a3a;
    --bd-muted:   #252f42;
    --bd-strong:  #2a3a50;
    --tx-hi:      #f9fafb;
    --tx-mid:     #e5e7eb;
    --tx-lo:      #9ca3af;
    --tx-dim:     #6b7280;
    --tx-xdim:    #4b5563;
    --tx-ghost:   #374151;
    --ac:         #3b82f6;
    --ac-dim:     #1e3a5f;
    --adi-sky:    #0a1628;
    --adi-ground: #1a1008;
  }

  :global([data-theme="cobalt2"]) {
    --bg-body:    #122738;
    --bg-panel:   #163448;
    --bg-card:    #193549;
    --bg-elevated:#1e4060;
    --bg-track:   #1a3b58;
    --bd-dim:     #1f4e6a;
    --bd-subtle:  #235a7a;
    --bd-muted:   #2a6d91;
    --bd-strong:  #337ba0;
    --tx-hi:      #ffffff;
    --tx-mid:     #e1efff;
    --tx-lo:      #9acfdf;
    --tx-dim:     #7eb8d4;
    --tx-xdim:    #5a96b8;
    --tx-ghost:   #3d7a9c;
    --ac:         #ffc600;
    --ac-dim:     #7a5e00;
    --adi-sky:    #0f2d47;
    --adi-ground: #1a2808;
  }

  :global([data-theme="purple"]) {
    --bg-body:    #0e0b1a;
    --bg-panel:   #130e24;
    --bg-card:    #18132e;
    --bg-elevated:#1f1840;
    --bg-track:   #1c1538;
    --bd-dim:     #251c4a;
    --bd-subtle:  #2d2260;
    --bd-muted:   #3a2b78;
    --bd-strong:  #4a3590;
    --tx-hi:      #f5f0ff;
    --tx-mid:     #ddd4ff;
    --tx-lo:      #b8a8e8;
    --tx-dim:     #8b6bb1;
    --tx-xdim:    #6248a0;
    --tx-ghost:   #4a3570;
    --ac:         #c084fc;
    --ac-dim:     #581c87;
    --adi-sky:    #0e0b28;
    --adi-ground: #1a0a2a;
  }

  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: var(--bg-body);
    color: var(--tx-hi);
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    overflow: hidden;
    height: 100vh;
    width: 100vw;
  }

  .dashboard {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }

  .main {
    flex: 1;
    display: grid;
    grid-template-columns: 1fr clamp(130px, 24vw, 210px);
    min-height: 0;
    overflow: hidden;
  }

  .center-area { background: var(--bg-body); overflow: hidden; min-width: 0; }
  .right-strip { border-left: 1px solid var(--bd-subtle); background: var(--bg-body); overflow: hidden; min-width: 0; }
  .lap-bar { height: clamp(2.5rem, 5.5vh, 4rem); flex-shrink: 0; }

  .toast-stack {
    position: fixed; bottom: 4rem; left: 50%; transform: translateX(-50%);
    display: flex; flex-direction: column; gap: 0.5rem; z-index: 200;
    pointer-events: none;
  }
  .toast {
    background: var(--bg-elevated); border: 1px solid #ef4444; border-radius: 6px;
    color: #fca5a5; font-size: 0.8rem; padding: 0.5rem 1rem;
    max-width: 420px; text-align: center;
  }
</style>
