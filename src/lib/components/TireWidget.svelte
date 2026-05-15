<script lang="ts">
  import { packet } from '$lib/stores/telemetry';

  let {
    tireTempCold = 60,
    tireTempOptimal = 85,
    tireTempHot = 110,
  }: {
    tireTempCold: number;
    tireTempOptimal: number;
    tireTempHot: number;
  } = $props();

  let pkt = $derived($packet);

  function tempColor(temp: number): string {
    if (temp < tireTempCold) return '#3b82f6';
    if (temp < tireTempOptimal) return '#22c55e';
    if (temp < tireTempHot) return '#f59e0b';
    return '#ef4444';
  }

  function slipLabel(slip: number): string {
    const abs = Math.abs(slip);
    if (abs < 0.05) return '●';
    if (abs < 0.15) return '◑';
    return '○';
  }

  let tires = $derived([
    { label: 'FL', temp: pkt?.tireTempFl ?? 0, slip: pkt?.tireSlipRatioFl ?? 0 },
    { label: 'FR', temp: pkt?.tireTempFr ?? 0, slip: pkt?.tireSlipRatioFr ?? 0 },
    { label: 'RL', temp: pkt?.tireTempRl ?? 0, slip: pkt?.tireSlipRatioRl ?? 0 },
    { label: 'RR', temp: pkt?.tireTempRr ?? 0, slip: pkt?.tireSlipRatioRr ?? 0 },
  ]);
</script>

<div class="tire-grid">
  {#each tires as tire, i}
    <div class="tire-tile" style="border-color: {tempColor(tire.temp)};">
      <span class="tire-label">{tire.label}</span>
      <span class="tire-temp" style="color: {tempColor(tire.temp)};">
        {pkt ? Math.round(tire.temp) + '°' : '—'}
      </span>
      <span class="tire-slip">{pkt ? slipLabel(tire.slip) : '—'}</span>
    </div>
  {/each}
</div>

<style>
  .tire-grid {
    display: grid;
    grid-template-columns: 1fr 1fr;
    grid-template-rows: 1fr 1fr;
    gap: 0.4rem;
    padding: 0.5rem;
    height: 100%;
    box-sizing: border-box;
  }
  .tire-tile {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    background: #111827;
    border: 2px solid #374151;
    border-radius: 6px;
    gap: 0.1rem;
    transition: border-color 0.2s;
    padding: 0.25rem;
  }
  .tire-label { font-size: 0.6rem; font-weight: 700; color: #6b7280; letter-spacing: 0.1em; }
  .tire-temp { font-size: 1rem; font-weight: 800; font-variant-numeric: tabular-nums; }
  .tire-slip { font-size: 0.75rem; color: #9ca3af; }
</style>
