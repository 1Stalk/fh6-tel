import { writable, derived } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { TelemetryPacket } from '$lib/types';

export const packet = writable<TelemetryPacket | null>(null);
export const isConnected = writable(false);

// Freezes at last isRaceOn=true packet so pause menu doesn't clear the display
let _frozen: TelemetryPacket | null = null;
export const displayPacket = derived(packet, ($p): TelemetryPacket | null => {
  if ($p !== null && $p.isRaceOn) {
    _frozen = $p;
    return $p;
  }
  return _frozen ?? $p;
});

export const speedMph = derived(displayPacket, ($p) =>
  $p ? $p.speedMs * 2.23694 : 0
);

export const speedKph = derived(displayPacket, ($p) =>
  $p ? $p.speedMs * 3.6 : 0
);

export const rpmPercent = derived(displayPacket, ($p) => {
  if (!$p || $p.engineMaxRpm === 0) return 0;
  return ($p.currentEngineRpm / $p.engineMaxRpm) * 100;
});

let lastPacketTime = 0;
let connectionTimer: ReturnType<typeof setInterval> | null = null;

export async function startTelemetryListener() {
  await listen<TelemetryPacket>('telemetry_tick', (event) => {
    packet.set(event.payload);
    lastPacketTime = Date.now();
    isConnected.set(true);
  });

  if (connectionTimer) clearInterval(connectionTimer);
  connectionTimer = setInterval(() => {
    if (Date.now() - lastPacketTime > 2000) {
      isConnected.set(false);
    }
  }, 1000);
}
