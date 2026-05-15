<script lang="ts">
  import { onMount } from 'svelte';
  import { startTelemetryListener } from '$lib/stores/telemetry';
  import { loadSettings, settings } from '$lib/stores/sessions';
  import TopBar from '$lib/components/TopBar.svelte';
  import InputStrip from '$lib/components/InputStrip.svelte';
  import CenterPanel from '$lib/components/CenterPanel.svelte';
  import TireWidget from '$lib/components/TireWidget.svelte';
  import LapBar from '$lib/components/LapBar.svelte';
  import SessionDrawer from '$lib/components/SessionDrawer.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';

  let showSessions = $state(false);
  let showSettings = $state(false);

  onMount(async () => {
    await loadSettings();
    await startTelemetryListener();
  });

  let s = $derived($settings);
</script>

<div class="dashboard">
  <TopBar
    useMph={s?.useMph ?? true}
    onSettings={() => (showSettings = true)}
    onSessions={() => (showSessions = !showSessions)}
  />

  <div class="main">
    <div class="left-strip">
      <InputStrip />
    </div>

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
  <SessionDrawer onClose={() => (showSessions = false)} />
{/if}

{#if showSettings}
  <SettingsModal onClose={() => (showSettings = false)} />
{/if}

<style>
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: #030712;
    color: #f9fafb;
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
    grid-template-columns: 80px 1fr 160px;
    min-height: 0;
  }

  .left-strip { border-right: 1px solid #1f2937; background: #030712; }
  .center-area { background: #030712; }
  .right-strip { border-left: 1px solid #1f2937; background: #030712; }
  .lap-bar { height: 3.5rem; flex-shrink: 0; }
</style>
