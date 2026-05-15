<script lang="ts">
  import { packet } from '$lib/stores/telemetry';

  $: pkt = $packet;

  $: bars = [
    { label: 'THR', value: pkt ? pkt.throttle / 255 : 0, color: '#22c55e' },
    { label: 'BRK', value: pkt ? pkt.brake / 255 : 0, color: '#ef4444' },
    { label: 'CLT', value: pkt ? pkt.clutch / 255 : 0, color: '#94a3b8' },
    { label: 'HBK', value: pkt ? pkt.handbrake / 255 : 0, color: '#f97316' },
  ];
</script>

<div class="strip">
  {#each bars as bar}
    <div class="bar-col">
      <div class="bar-track">
        <div
          class="bar-fill"
          style="height: {bar.value * 100}%; background: {bar.color};"
        ></div>
      </div>
      <span class="bar-label">{bar.label}</span>
    </div>
  {/each}
</div>

<style>
  .strip {
    display: flex;
    flex-direction: row;
    gap: 0.4rem;
    align-items: flex-end;
    padding: 0.75rem 0.5rem;
    height: 100%;
    box-sizing: border-box;
  }
  .bar-col {
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 0.3rem;
    flex: 1;
    height: 100%;
  }
  .bar-track {
    flex: 1;
    width: 100%;
    background: #1f2937;
    border-radius: 3px;
    display: flex;
    flex-direction: column;
    justify-content: flex-end;
    overflow: hidden;
    min-height: 0;
  }
  .bar-fill {
    width: 100%;
    transition: height 33ms linear;
    border-radius: 3px;
  }
  .bar-label {
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.05em;
    color: #6b7280;
  }
</style>
