<script lang="ts">
  import { packet, replay } from '$lib/stores/telemetry';
  import { settings } from '$lib/stores/sessions';
  import MapPanel from './MapPanel.svelte';
  import type { TelemetryPacket } from '$lib/types';

  let livePosition = $state<TelemetryPacket | null>(null);
  let bc = $state<BroadcastChannel | null>(null);

  $effect(() => {
    bc = new BroadcastChannel('fh6-tel-map');
    return () => bc?.close();
  });

  $effect(() => {
    const p = $packet;
    if ($replay.active) return;
    if (!p || !p.isRaceOn) { livePosition = null; return; }
    if (p.positionX !== 0 || p.positionZ !== 0) livePosition = p;
  });

  let pts = $derived($replay.active ? $replay.packets : (livePosition ? [livePosition] : []));
  let idx = $derived($replay.active ? $replay.index : 0);
  let drawLine = $derived($replay.active);

  let lastBroadcast = 0;
  $effect(() => {
    void pts; void idx; void drawLine;
    if (!bc) return;
    const now = Date.now();
    if (now - lastBroadcast < 50) return;
    lastBroadcast = now;
    bc.postMessage({
      type: 'map-state',
      pts: $state.snapshot(pts),
      idx,
      drawLine,
      colorByLap: $replay.active,
      fixedTrace: $replay.active,
    });
  });
</script>

{#if $settings}
  <MapPanel
    points={pts}
    currentIndex={idx}
    {drawLine}
    colorByLap={$replay.active}
    fixedTrace={$replay.active}
    settings={$settings}
  />
{/if}
