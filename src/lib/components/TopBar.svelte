<script lang="ts">
  import { isConnected, packet } from '$lib/stores/telemetry';
  import { carName } from '$lib/car-name';
  import { CAR_CLASS_LABELS, DRIVETRAIN_LABELS } from '$lib/types';

  let { useMph = true, onSettings, onSessions }: {
    useMph: boolean;
    onSettings: () => void;
    onSessions: () => void;
  } = $props();

  let pkt = $derived($packet);
  let connected = $derived($isConnected);
  let carLabel = $derived(pkt ? carName(pkt.carOrdinal) : '—');
  let classLabel = $derived(pkt ? (CAR_CLASS_LABELS[pkt.carClass] ?? '?') : '—');
  let piLabel = $derived(pkt ? String(pkt.carPi) : '—');
  let driveLabel = $derived(pkt ? (DRIVETRAIN_LABELS[pkt.drivetrainType] ?? '?') : '—');
</script>

<header class="topbar">
  <div class="status">
    <span class="dot" class:live={connected}></span>
    <span class="label">{connected ? 'LIVE' : 'WAITING…'}</span>
  </div>

  <div class="car-info">
    <span class="car-name">{carLabel}</span>
    <span class="badge">{classLabel}</span>
    <span class="badge">{piLabel}</span>
    <span class="badge">{driveLabel}</span>
  </div>

  <div class="controls">
    <button class="icon-btn" onclick={onSessions} title="Sessions">⏱</button>
    <button class="icon-btn" onclick={onSettings} title="Settings">⚙</button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1rem;
    height: 2.5rem;
    background: #0a0a0a;
    border-bottom: 1px solid #222;
    flex-shrink: 0;
  }
  .status { display: flex; align-items: center; gap: 0.4rem; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: #444;
    transition: background 0.3s;
  }
  .dot.live { background: #22c55e; box-shadow: 0 0 6px #22c55e; }
  .label { font-size: 0.7rem; font-weight: 700; letter-spacing: 0.1em; color: #888; }
  .car-info { display: flex; align-items: center; gap: 0.5rem; }
  .car-name { font-size: 0.85rem; font-weight: 600; color: #e5e7eb; }
  .badge {
    font-size: 0.65rem; font-weight: 700; padding: 0.1rem 0.4rem;
    border: 1px solid #333; border-radius: 3px; color: #9ca3af;
  }
  .controls { display: flex; gap: 0.25rem; }
  .icon-btn {
    background: none; border: none; cursor: pointer;
    font-size: 1rem; color: #6b7280; padding: 0.25rem 0.5rem;
    border-radius: 4px;
  }
  .icon-btn:hover { background: #1f2937; color: #e5e7eb; }
</style>
