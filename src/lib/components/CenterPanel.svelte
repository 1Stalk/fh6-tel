<script lang="ts">
  import { packet, speedMph, speedKph, rpmPercent } from '$lib/stores/telemetry';

  let { useMph = true }: { useMph: boolean } = $props();

  let pkt = $derived($packet);
  let speed = $derived(useMph ? Math.round($speedMph) : Math.round($speedKph));
  let unit = $derived(useMph ? 'mph' : 'kph');
  let rpm = $derived($rpmPercent);
  let gearLabel = $derived((() => {
    if (!pkt) return '—';
    if (pkt.gear === 0) return 'R';
    if (pkt.gear === 11) return 'N';
    return String(pkt.gear);
  })());
  let boostBar = $derived(pkt ? Math.min(Math.max(pkt.boost / 2.0, 0), 1) : 0);
  let isRedline = $derived(rpm > 90);
</script>

<div class="center">
  <div class="speed-row">
    <div class="speed">{speed}</div>
    <div class="gear-box" class:redline={isRedline}>{gearLabel}</div>
  </div>
  <div class="unit-label">{unit}</div>

  <div class="gauge-row">
    <div class="gauge-group">
      <span class="gauge-label">RPM</span>
      <div class="gauge-track">
        <div
          class="gauge-fill rpm-fill"
          class:rpm-redline={isRedline}
          style="width: {rpm}%"
        ></div>
        <div class="redline-marker"></div>
      </div>
    </div>

    {#if pkt && pkt.boost > 0.05}
      <div class="gauge-group">
        <span class="gauge-label">BOOST {pkt.boost.toFixed(2)} bar</span>
        <div class="gauge-track">
          <div class="gauge-fill boost-fill" style="width: {boostBar * 100}%"></div>
        </div>
      </div>
    {/if}
  </div>
</div>

<style>
  .center {
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100%;
    padding: 1rem;
    gap: 0.5rem;
  }
  .speed-row { display: flex; align-items: center; gap: 1.5rem; }
  .speed {
    font-size: 6rem;
    font-weight: 900;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: #f9fafb;
  }
  .gear-box {
    font-size: 3rem;
    font-weight: 900;
    width: 3.5rem;
    height: 3.5rem;
    display: flex;
    align-items: center;
    justify-content: center;
    border: 2px solid #374151;
    border-radius: 8px;
    color: #e5e7eb;
    background: #111827;
    transition: border-color 0.1s, color 0.1s;
  }
  .gear-box.redline { border-color: #ef4444; color: #ef4444; box-shadow: 0 0 12px rgba(239,68,68,0.4); }
  .unit-label { font-size: 0.8rem; font-weight: 700; letter-spacing: 0.15em; color: #6b7280; margin-top: -0.5rem; }
  .gauge-row { width: 100%; display: flex; flex-direction: column; gap: 0.4rem; }
  .gauge-group { display: flex; flex-direction: column; gap: 0.2rem; }
  .gauge-label { font-size: 0.65rem; color: #6b7280; font-weight: 700; letter-spacing: 0.1em; }
  .gauge-track { height: 12px; background: #1f2937; border-radius: 3px; overflow: hidden; position: relative; }
  .gauge-fill { height: 100%; border-radius: 3px; transition: width 33ms linear; }
  .rpm-fill { background: #3b82f6; }
  .rpm-fill.rpm-redline { background: #ef4444; }
  .boost-fill { background: #a855f7; }
  .redline-marker { position: absolute; right: 10%; top: 0; width: 2px; height: 100%; background: rgba(239,68,68,0.6); }
</style>
