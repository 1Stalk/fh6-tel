#!/usr/bin/env node
// One-time build asset fetcher. Downloads the full Forza Horizon 6 (Japan)
// tile pyramid into static/maptiles/ so it ships inside the app build.
//
// Discovery is a flood-fill from a known seed tile: only tiles that return 200
// are followed, so we fetch exactly the map's real extent (not a huge empty
// rectangle). Resumable — existing non-empty files are kept and only their
// neighbours are explored.
//
// Usage:
//   node scripts/download-map-tiles.mjs [options]
//     --tmpl <url>     Tile template (default: mapgenie FH6 Japan)
//     --seed z,x,y     Known good tile (default: 13,4082,4077)
//     --min <z>        Min zoom (default: 0)
//     --max <z>        Max zoom (default: auto-probe from seed)
//     --out <dir>      Output dir (default: static/maptiles)
//     --concurrency N  Parallel requests (default: 8)
//     --dry            Only discover + count, don't write files

import { mkdir, writeFile, stat } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const ROOT = join(dirname(fileURLToPath(import.meta.url)), '..');

// NOTE: mapgenie's path order is z/y/x. The flood-fill is axis-symmetric and
// writes each tile to the exact path it fetched, so files land on disk as
// /{z}/{y}/{x}.jpg — which is what the app's tile template reads. The {x}/{y}
// token names below are just the two path slots after {z}; don't read into them.
const DEFAULTS = {
  tmpl: 'https://tiles.mapgenie.io/games/forza-horizon-6/one/default-v2/{z}/{x}/{y}.jpg',
  seed: [13, 4082, 4077],
  min: 0,
  max: null,
  out: join(ROOT, 'static', 'maptiles'),
  concurrency: 8,
  dry: false,
};

function parseArgs(argv) {
  const o = { ...DEFAULTS };
  for (let i = 0; i < argv.length; i++) {
    const a = argv[i];
    if (a === '--dry') o.dry = true;
    else if (a === '--tmpl') o.tmpl = argv[++i];
    else if (a === '--out') o.out = argv[++i];
    else if (a === '--min') o.min = Number(argv[++i]);
    else if (a === '--max') o.max = Number(argv[++i]);
    else if (a === '--concurrency') o.concurrency = Number(argv[++i]);
    else if (a === '--seed') o.seed = argv[++i].split(',').map(Number);
    else throw new Error(`Unknown arg: ${a}`);
  }
  return o;
}

const opts = parseArgs(process.argv.slice(2));
const EXT = (opts.tmpl.split('.').pop() || 'jpg').split(/[?#]/)[0];
const [SEED_Z, SEED_X, SEED_Y] = opts.seed;

const url = (z, x, y) =>
  opts.tmpl.replace('{z}', z).replace('{x}', x).replace('{y}', y);

const tilePath = (z, x, y) => join(opts.out, String(z), String(x), `${y}.${EXT}`);

async function fileOk(p) {
  try {
    const s = await stat(p);
    return s.isFile() && s.size > 0;
  } catch {
    return false;
  }
}

// Fetch with retry/backoff. Returns: 'ok' | 'missing' | 'error'.
async function fetchTile(z, x, y, write) {
  const p = tilePath(z, x, y);
  if (await fileOk(p)) return 'ok'; // resume

  for (let attempt = 0; attempt < 4; attempt++) {
    try {
      const res = await fetch(url(z, x, y));
      // mapgenie serves 403 (not 404) for tiles outside the map extent, so
      // treat both as "edge" — the flood-fill stops there, not an error.
      if (res.status === 404 || res.status === 403) return 'missing';
      if (res.status === 429 || res.status >= 500) {
        await sleep(500 * (attempt + 1));
        continue;
      }
      if (!res.ok) return 'error';
      const buf = Buffer.from(await res.arrayBuffer());
      if (buf.length === 0) return 'missing';
      if (write) {
        await mkdir(dirname(p), { recursive: true });
        await writeFile(p, buf);
      }
      return 'ok';
    } catch {
      await sleep(400 * (attempt + 1));
    }
  }
  return 'error';
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// Scale the seed tile coords to a given zoom.
function seedAt(z) {
  if (z === SEED_Z) return [SEED_X, SEED_Y];
  if (z < SEED_Z) {
    const f = 2 ** (SEED_Z - z);
    return [Math.floor(SEED_X / f), Math.floor(SEED_Y / f)];
  }
  const f = 2 ** (z - SEED_Z);
  return [SEED_X * f, SEED_Y * f];
}

async function probeMaxZoom() {
  let max = SEED_Z;
  for (let z = SEED_Z + 1; z <= 22; z++) {
    const [sx, sy] = seedAt(z);
    // Check the four children around the scaled seed.
    const checks = [
      [sx, sy], [sx + 1, sy], [sx, sy + 1], [sx + 1, sy + 1],
    ];
    let found = false;
    for (const [x, y] of checks) {
      if ((await fetchTile(z, x, y, false)) === 'ok') { found = true; break; }
    }
    if (!found) break;
    max = z;
  }
  return max;
}

// Flood-fill one zoom level from its scaled seed.
async function fillZoom(z, write) {
  const [sx, sy] = seedAt(z);
  const visited = new Set();
  const queue = [[sx, sy]];
  visited.add(`${sx},${sy}`);
  let ok = 0, missing = 0, errors = 0, done = 0;

  let qi = 0;
  async function worker() {
    while (qi < queue.length) {
      const [x, y] = queue[qi++];
      if (x < 0 || y < 0) { continue; }
      const r = await fetchTile(z, x, y, write);
      done++;
      if (r === 'ok') {
        ok++;
        for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
          const k = `${x + dx},${y + dy}`;
          if (!visited.has(k) && x + dx >= 0 && y + dy >= 0) {
            visited.add(k);
            queue.push([x + dx, y + dy]);
          }
        }
      } else if (r === 'missing') missing++;
      else errors++;

      if (done % 200 === 0) {
        process.stdout.write(
          `\r  z${z}: ${ok} ok, ${missing} edge, ${errors} err, queue ${queue.length - qi}   `
        );
      }
    }
  }

  // Workers cooperatively drain the growing queue.
  const workers = Array.from({ length: opts.concurrency }, () => worker());
  await Promise.all(workers);
  process.stdout.write(
    `\r  z${z}: ${ok} tiles, ${missing} edge misses, ${errors} errors${' '.repeat(20)}\n`
  );
  return { ok, errors };
}

async function main() {
  const max = opts.max ?? (await probeMaxZoom());
  console.log(`Template : ${opts.tmpl}`);
  console.log(`Seed     : z${SEED_Z} ${SEED_X},${SEED_Y}`);
  console.log(`Zoom     : ${opts.min}..${max}${opts.max == null ? ' (auto)' : ''}`);
  console.log(`Output   : ${opts.out}${opts.dry ? '  (dry run)' : ''}`);
  console.log('');

  let totalOk = 0, totalErr = 0;
  for (let z = opts.min; z <= max; z++) {
    const { ok, errors } = await fillZoom(z, !opts.dry);
    totalOk += ok;
    totalErr += errors;
  }
  console.log(`\nTotal: ${totalOk} tiles${totalErr ? `, ${totalErr} errors` : ''}.`);
  if (!opts.dry) {
    console.log('Tiles are in static/maptiles/ and will ship with the next build.');
  }
  if (totalErr > 0) process.exitCode = 1;
}

main().catch((e) => {
  console.error(e);
  process.exit(1);
});
