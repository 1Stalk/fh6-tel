import ordinals from '$lib/car-ordinals.json';

const map: Record<string, string> = ordinals as Record<string, string>;

export function carName(ordinal: number): string {
  return map[String(ordinal)] ?? `Car #${ordinal}`;
}
