<script lang="ts">
  import { packet, replay } from '$lib/stores/telemetry';
  import { settings, saveSettings } from '$lib/stores/sessions';
  import MapPanel from './MapPanel.svelte';
  import type { TelemetryPacket } from '$lib/types';

  // Accumulated driven trace for the current live run, sampled at ~20 Hz.
  let livePoints = $state<TelemetryPacket[]>([]);
  let frame = 0;
  let prevRaceOn = false;

  $effect(() => {
    const p = $packet;
    if ($replay.active || !p) return;

    if (p.isRaceOn && !prevRaceOn) {
      livePoints = [];
      frame = 0;
    }
    prevRaceOn = p.isRaceOn;

    if (p.isRaceOn && (p.positionX !== 0 || p.positionZ !== 0)) {
      if (frame % 3 === 0) livePoints = [...livePoints, p];
      frame++;
    }
  });

  let pts = $derived($replay.active ? $replay.packets : livePoints);
  let idx = $derived($replay.active ? $replay.index : livePoints.length - 1);

  // Draw the racing line whenever a lap is being timed (race, Rivals, or Time
  // Trial — Time Trial reports racePosition 0, so key off the lap clock, not
  // position). Free-roam has no lap timer, so it shows only the player marker.
  let inEvent = $derived(
    $replay.active ||
      (($packet?.currentLap ?? 0) > 0) ||
      (($packet?.lastLap ?? 0) > 0) ||
      (($packet?.lapNumber ?? 0) > 0)
  );

  async function toggle() {
    if (!$settings) return;
    await saveSettings({ ...$settings, mapEnabled: !$settings.mapEnabled });
  }
</script>

<div class="map-widget">
  <div class="map-header">
    <span class="map-label">{$replay.active ? 'REPLAY MAP' : 'TRACK MAP'}</span>
    <button
      class="toggle"
      class:on={$settings?.mapEnabled}
      onclick={toggle}
      title={$settings?.mapEnabled ? 'Hide map' : 'Show map'}
    >
      {$settings?.mapEnabled ? 'ON' : 'OFF'}
    </button>
  </div>

  {#if $settings?.mapEnabled && $settings}
    <MapPanel
      points={pts}
      currentIndex={idx}
      drawLine={inEvent}
      colorByLap={$replay.active}
      fixedTrace={$replay.active}
      settings={$settings}
      compact
    />
  {/if}
</div>

<style>
  .map-widget {
    border-top: 1px solid var(--bd-subtle);
    padding: 0.4rem;
    display: flex;
    flex-direction: column;
    gap: 0.25rem;
    /* Contain Leaflet's internal pane z-indexes so the map can't paint over
       the session drawer / modals. */
    isolation: isolate;
  }
  .map-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
  }
  .map-label {
    color: var(--tx-dim);
    font-size: 0.6rem;
    font-weight: 700;
    letter-spacing: 0.06em;
  }
  .toggle {
    background: var(--bg-elevated);
    border: 1px solid var(--bd-muted);
    color: var(--tx-dim);
    font-size: 0.58rem;
    font-weight: 700;
    padding: 0.1rem 0.35rem;
    border-radius: 3px;
    cursor: pointer;
  }
  .toggle.on {
    border-color: var(--ac);
    color: var(--ac);
  }
</style>
