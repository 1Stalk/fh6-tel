# Forza Horizon 6 Telemetry Dashboard — Design Spec

**Date:** 2026-05-15  
**Stack:** Tauri 2 · Rust backend · Svelte + TypeScript frontend · SQLite  
**Target:** Windows 10/11 (x64), distributed as a single NSIS/MSI installer

---

## 1. Overview

A native Windows desktop application that receives live UDP telemetry from Forza Horizon 6's "Data Out" feature, displays it on a full-screen real-time dashboard, and records driving sessions to a local SQLite database for later review.

Live telemetry is always displayed regardless of whether a session is being recorded. Session recording activates automatically when the game signals the car is in a race/free-roam event and stops when it signals the car is idle.

---

## 2. Telemetry Protocol

Forza Horizon 6 uses the same UDP "Data Out" protocol as FH5:

- **Packet format:** "Dash" — 324 bytes, little-endian
- **Offset:** 12-byte offset applied vs. the "Sled" format (first 12 bytes are the Sled header)
- **Rate:** 60 packets/second when enabled
- **Default port:** `20440` (user-configurable in app settings)
- **In-game setup:** Settings → HUD and Gameplay → Data Out → On, set IP to `127.0.0.1` and port to match app setting

Packet fields parsed (subset shown; full struct in implementation):

| Field | Type | Description |
|-------|------|-------------|
| IsRaceOn | i32 | 1 = car active |
| TimestampMS | u32 | Game clock ms |
| EngineMaxRpm | f32 | Redline |
| EngineIdleRpm | f32 | Idle RPM |
| CurrentEngineRpm | f32 | Live RPM |
| AccelerationX/Y/Z | f32 | m/s² world axes |
| VelocityX/Y/Z | f32 | m/s world axes |
| Speed | f32 | m/s (converted to mph/kph) |
| Power | f32 | Watts |
| Torque | f32 | Nm |
| TireTemp FL/FR/RL/RR | f32 | °C surface temp |
| TireSlipRatio FL/FR/RL/RR | f32 | Slip ratio |
| TireSlipAngle FL/FR/RL/RR | f32 | Slip angle |
| NormalizedDrivingLine | i8 | Racing line assist |
| NormalizedAIBrakeDifference | i8 | |
| Gear | u8 | Current gear (0=R, 1–10) |
| Throttle | u8 | 0–255 |
| Brake | u8 | 0–255 |
| Clutch | u8 | 0–255 |
| HandBrake | u8 | 0–255 |
| Boost | f32 | Bar |
| Fuel | f32 | Remaining fraction |
| DistanceTraveled | f32 | m |
| BestLap | f32 | s |
| LastLap | f32 | s |
| CurrentLap | f32 | s |
| CurrentRaceTime | f32 | s |
| LapNumber | u16 | |
| RacePosition | u8 | |
| Accel | u8 | Input (duplicates Throttle in Dash) |
| CarOrdinal | i32 | Car ID |
| CarClass | i32 | D/C/B/A/S1/S2/X |
| CarPerformanceIndex | i32 | PI value |
| DrivetrainType | i32 | 0=FWD,1=RWD,2=AWD |
| NumCylinders | i32 | |

---

## 3. Architecture

```
┌─────────────────────────────────────────────────┐
│                Tauri Process                    │
│                                                 │
│  ┌──────────────┐    ┌─────────────────────┐   │
│  │  UDP Listener│───▶│  Packet Parser      │   │
│  │  (tokio UDP) │    │  (Rust struct)      │   │
│  └──────────────┘    └────────┬────────────┘   │
│                               │                 │
│                    ┌──────────▼──────────┐      │
│                    │  AppState (Mutex)   │      │
│                    │  - latest packet    │      │
│                    │  - session state    │      │
│                    └──────────┬──────────┘      │
│                               │                 │
│              ┌────────────────┴──────────────┐  │
│              │                               │  │
│   ┌──────────▼────────┐       ┌─────────────▼─┐│
│   │  Tauri Event emit │       │  SQLite writer ││
│   │  (telemetry_tick) │       │  (rusqlite)    ││
│   └──────────┬────────┘       └───────────────┘│
└──────────────┼──────────────────────────────────┘
               │ Tauri IPC
┌──────────────▼──────────────────────────────────┐
│              Svelte Frontend                    │
│                                                 │
│  Live dashboard ◀── telemetry_tick events       │
│  Session list   ◀── Tauri commands              │
└─────────────────────────────────────────────────┘
```

### Rust backend responsibilities

- **`udp.rs`** — Spawns a `tokio` UDP socket task. Reads datagrams in a loop, passes raw bytes to the parser. Emits `telemetry_tick` Tauri event after each successful parse. Continues emitting when `IsRaceOn = 0` (always-live requirement).
- **`parser.rs`** — Deserializes the 324-byte Dash packet using `byteorder` or `zerocopy`. Returns a typed `TelemetryPacket` struct.
- **`session.rs`** — Watches `IsRaceOn` transitions. On `0→1`: opens a new session row in SQLite. On `1→0`: closes the session. Writes every packet to `session_packets` table during an active session.
- **`db.rs`** — SQLite schema management, session CRUD, packet batch insert.
- **`commands.rs`** — Tauri commands: `get_sessions`, `get_session_packets`, `delete_session`, `get_settings`, `save_settings`.
- **`settings.rs`** — Persists port and units preference to `AppData\Local\fh6-tel\settings.json`.

### SQLite schema

```sql
CREATE TABLE sessions (
  id INTEGER PRIMARY KEY,
  started_at INTEGER NOT NULL,  -- unix ms
  ended_at INTEGER,
  car_ordinal INTEGER,
  car_class INTEGER,
  car_pi INTEGER,
  best_lap REAL,
  packet_count INTEGER DEFAULT 0
);

CREATE TABLE session_packets (
  id INTEGER PRIMARY KEY,
  session_id INTEGER NOT NULL REFERENCES sessions(id),
  timestamp_ms INTEGER NOT NULL,
  data BLOB NOT NULL  -- raw 324-byte packet
);

CREATE INDEX idx_packets_session ON session_packets(session_id);
```

Packets stored as raw blobs — re-parsed on playback so the schema stays stable if field definitions change.

---

## 4. Frontend Layout

Single full-screen window, no navigation bar. All panels visible simultaneously.

```
┌──────────────────────────────────────────────────────────┐
│  [● LIVE]  Toyota GR010 · PI 998 · AWD    mph  ⚙        │  ← top bar
├────────┬─────────────────────────────────┬───────────────┤
│        │                                 │  FL ██ FR     │
│ THR ██ │        187 mph                  │  ████████     │  ← tire widget
│ BRK ░░ │         [  5  ]                 │  RL ██ RR     │
│ CLT ░░ │    ████████████████ RPM         │  ████████     │
│ HBK ░░ │    ████ BOOST ████              │               │
│        │                                 │  Slip / Temp  │
├────────┴─────────────────────────────────┴───────────────┤
│  Lap 3  Current: 1:23.456  Last: 1:22.891  Best: 1:21.204│  ← lap bar
└──────────────────────────────────────────────────────────┘
```

**Panels:**

- **Top bar** — Connection dot (green=receiving, red=no signal), car name (from CarOrdinal lookup table), PI, drivetrain type, mph/kph toggle, settings cog.
- **Left strip** — Vertical bars for Throttle, Brake, Clutch, Handbrake. Color-coded (green/red/grey/orange).
- **Center** — Speed (large numeral), gear (large box), RPM bar (fills left→right with redline marker), boost gauge.
- **Right strip** — Tire widget: 4 tiles in car-corner positions. Each tile shows tire temp (color: blue=cold, green=optimal, red=hot) and slip ratio indicator.
- **Bottom bar** — Lap number, current lap time (ticking), last lap, best lap, session elapsed time.

**Session drawer** — Triggered by a button in the top bar (or keyboard shortcut). Slides in from the right without leaving the dashboard. Shows a list of past sessions (car, date, best lap). Clicking a session opens a graph view overlaid on the dashboard showing speed, throttle, brake, and RPM over the session timeline with a scrubber.

---

## 5. Settings

Accessible via the cog icon in the top bar. A modal overlay (not a separate window).

- UDP listen port (default `20440`)
- Units: mph / kph
- Tire temp thresholds (cold/optimal/hot °C, with defaults)
- Session auto-record toggle (default on)

---

## 6. Distribution

Tauri's built-in bundler produces:
- **NSIS installer** (`.exe`) — recommended, single-file, user can install without admin rights
- **MSI** — optional for enterprise/silent install

WebView2 is pre-installed on Windows 10 1803+ and all Windows 11. No runtime dependency to ship.

App data stored in `%LOCALAPPDATA%\fh6-tel\`:
- `sessions.db` — SQLite database
- `settings.json` — user settings

---

## 7. Error Handling

- **No UDP signal** — top bar shows red dot and "Waiting for FH6…" message. Dashboard panels show `—` placeholders. App remains fully functional.
- **Port in use** — settings modal shows an error, prompts user to change port.
- **SQLite write failure** — logged, live display unaffected. Session silently not recorded; user notified via a non-blocking toast.
- **Unknown packet size** — packet silently dropped, counter incremented for diagnostics.

---

## 8. Out of Scope

- macOS / Linux support
- Multiplayer / cloud sync
- Car name lookup (CarOrdinal → name) is best-effort via a bundled JSON mapping; unknown cars show the ordinal number
- Replay with audio
