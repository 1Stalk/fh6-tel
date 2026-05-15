import { writable } from 'svelte/store';
import { invoke } from '@tauri-apps/api/core';
import type { SessionRow, TelemetryPacket, AppSettings } from '$lib/types';

export const sessions = writable<SessionRow[]>([]);
export const settings = writable<AppSettings | null>(null);

export async function loadSessions() {
  const rows = await invoke<SessionRow[]>('get_sessions');
  sessions.set(rows);
}

export async function loadSessionPackets(sessionId: number): Promise<TelemetryPacket[]> {
  return invoke<TelemetryPacket[]>('get_session_packets', { sessionId });
}

export async function deleteSession(sessionId: number) {
  await invoke('delete_session', { sessionId });
  await loadSessions();
}

export async function loadSettings(): Promise<AppSettings> {
  const s = await invoke<AppSettings>('get_settings');
  settings.set(s);
  return s;
}

export async function saveSettings(s: AppSettings) {
  await invoke('save_settings', { newSettings: s });
  settings.set(s);
}
