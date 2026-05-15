<script lang="ts">
  import { packet } from '$lib/stores/telemetry';

  $: pkt = $packet;

  function formatTime(seconds: number): string {
    if (seconds <= 0) return '—:——.———';
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toFixed(3).padStart(6, '0')}`;
  }
</script>

<div class="lapbar">
  <div class="lap-item">
    <span class="lap-key">LAP</span>
    <span class="lap-val">{pkt ? pkt.lapNumber : '—'}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">CURRENT</span>
    <span class="lap-val current">{formatTime(pkt?.currentLap ?? 0)}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">LAST</span>
    <span class="lap-val">{formatTime(pkt?.lastLap ?? 0)}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">BEST</span>
    <span class="lap-val best">{formatTime(pkt?.bestLap ?? 0)}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">SESSION</span>
    <span class="lap-val">{formatTime(pkt?.currentRaceTime ?? 0)}</span>
  </div>
</div>

<style>
  .lapbar {
    display: flex;
    align-items: center;
    justify-content: center;
    height: 100%;
    background: #0a0a0a;
    border-top: 1px solid #222;
    padding: 0 1rem;
  }
  .lap-item { display: flex; flex-direction: column; align-items: center; padding: 0 1.5rem; }
  .lap-key { font-size: 0.55rem; font-weight: 700; letter-spacing: 0.15em; color: #6b7280; }
  .lap-val { font-size: 1.1rem; font-weight: 800; font-variant-numeric: tabular-nums; color: #e5e7eb; }
  .lap-val.current { color: #3b82f6; }
  .lap-val.best { color: #a855f7; }
  .sep { width: 1px; height: 2rem; background: #1f2937; }
</style>
