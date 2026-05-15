<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { sessions, loadSessions, loadSessionPackets, deleteSession } from '$lib/stores/sessions';
  import { carName } from '$lib/car-name';
  import type { TelemetryPacket, SessionRow } from '$lib/types';
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';

  let { onClose }: { onClose: () => void } = $props();

  let selectedSession = $state<SessionRow | null>(null);
  let chartEl = $state<HTMLDivElement | null>(null);
  let plot: uPlot | null = null;

  onMount(async () => {
    await loadSessions();
  });

  onDestroy(() => {
    plot?.destroy();
  });

  function formatTime(seconds: number) {
    if (!seconds || seconds <= 0) return '—';
    const m = Math.floor(seconds / 60);
    const s = (seconds % 60).toFixed(3).padStart(6, '0');
    return `${m}:${s}`;
  }

  function formatDate(ms: number) {
    return new Date(ms).toLocaleString();
  }

  async function selectSession(session: SessionRow) {
    selectedSession = session;
    plot?.destroy();
    plot = null;

    const packets: TelemetryPacket[] = await loadSessionPackets(session.id);
    if (packets.length === 0 || !chartEl) return;

    const times = packets.map((_, i) => i / 60);
    const speeds = packets.map(p => p.speedMs * 2.23694);
    const throttles = packets.map(p => (p.throttle / 255) * 100);
    const brakes = packets.map(p => (p.brake / 255) * 100);
    const rpms = packets.map(p =>
      p.engineMaxRpm > 0 ? (p.currentEngineRpm / p.engineMaxRpm) * 100 : 0
    );

    const opts: uPlot.Options = {
      width: chartEl.clientWidth || 380,
      height: 200,
      series: [
        {},
        { label: 'Speed (mph)', stroke: '#3b82f6', width: 1.5 },
        { label: 'Throttle %', stroke: '#22c55e', width: 1 },
        { label: 'Brake %', stroke: '#ef4444', width: 1 },
        { label: 'RPM %', stroke: '#a855f7', width: 1 },
      ],
      axes: [
        { stroke: '#6b7280', grid: { stroke: '#1f2937' } },
        { stroke: '#6b7280', grid: { stroke: '#1f2937' } },
      ],
    };

    plot = new uPlot(opts, [times, speeds, throttles, brakes, rpms], chartEl);
  }

  async function handleDelete(session: SessionRow, e: MouseEvent) {
    e.stopPropagation();
    if (!confirm(`Delete session from ${formatDate(session.startedAt)}?`)) return;
    await deleteSession(session.id);
    if (selectedSession?.id === session.id) {
      selectedSession = null;
      plot?.destroy();
      plot = null;
    }
  }
</script>

<div class="drawer">
  <div class="drawer-header">
    <h3>Sessions</h3>
    <button onclick={onClose}>✕</button>
  </div>

  <div class="drawer-body">
    <div class="session-list">
      {#each $sessions as session}
        <div
          class="session-row"
          class:selected={selectedSession?.id === session.id}
          role="button"
          tabindex="0"
          onclick={() => selectSession(session)}
          onkeydown={(e) => e.key === 'Enter' && selectSession(session)}
        >
          <div class="session-info">
            <span class="session-car">{carName(session.carOrdinal)}</span>
            <span class="session-date">{formatDate(session.startedAt)}</span>
            <span class="session-best">Best: {formatTime(session.bestLap ?? 0)}</span>
          </div>
          <button class="delete-btn" onclick={(e) => handleDelete(session, e)}>🗑</button>
        </div>
      {:else}
        <p class="empty">No sessions recorded yet.</p>
      {/each}
    </div>

    {#if selectedSession}
      <div class="chart-area" bind:this={chartEl}></div>
    {/if}
  </div>
</div>

<style>
  .drawer {
    position: fixed; right: 0; top: 0; bottom: 0; width: 420px;
    background: #0f172a; border-left: 1px solid #1e293b;
    display: flex; flex-direction: column; z-index: 50;
    box-shadow: -4px 0 24px rgba(0,0,0,0.5);
  }
  .drawer-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 1rem; border-bottom: 1px solid #1e293b;
  }
  h3 { margin: 0; color: #f9fafb; }
  .drawer-header button { background: none; border: none; color: #6b7280; font-size: 1.1rem; cursor: pointer; }
  .drawer-body { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 1rem; padding: 0.5rem; }
  .session-list { display: flex; flex-direction: column; gap: 0.3rem; }
  .session-row {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.6rem 0.75rem; border-radius: 6px; cursor: pointer;
    border: 1px solid transparent; background: #1e293b;
  }
  .session-row:hover, .session-row.selected { border-color: #3b82f6; }
  .session-info { display: flex; flex-direction: column; gap: 0.1rem; }
  .session-car { font-size: 0.85rem; font-weight: 600; color: #e5e7eb; }
  .session-date { font-size: 0.7rem; color: #6b7280; }
  .session-best { font-size: 0.75rem; color: #a855f7; font-weight: 700; }
  .delete-btn { background: none; border: none; cursor: pointer; font-size: 0.9rem; color: #6b7280; }
  .delete-btn:hover { color: #ef4444; }
  .empty { color: #4b5563; font-size: 0.85rem; text-align: center; padding: 2rem; }
  .chart-area { min-height: 220px; border-radius: 6px; overflow: hidden; background: #111827; }
  :global(.uplot) { background: transparent !important; }
</style>
