# FH6 Telemetry Dashboard — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Tauri 2 Windows desktop app that receives live UDP telemetry from Forza Horizon 6, displays it on a full-screen real-time dashboard, and records driving sessions to SQLite for later review.

**Architecture:** Rust backend owns the UDP socket, binary packet parser, and SQLite session store. Every parsed packet is broadcast to the Svelte 5 frontend via a Tauri event (`telemetry_tick`) at up to 60fps. Session recording activates automatically on `IsRaceOn` transitions; live display runs regardless. Single installer, no external runtime.

**Tech Stack:** Tauri 2 · Rust (tokio, byteorder, rusqlite, serde, dirs) · Svelte 5 + TypeScript · SvelteKit with adapter-static · uPlot · NSIS installer (built by `cargo tauri build`)

---

## File Map

```
fh6-tel/
├── src-tauri/
│   ├── Cargo.toml
│   ├── build.rs
│   ├── tauri.conf.json
│   ├── capabilities/
│   │   └── default.json
│   └── src/
│       ├── main.rs          ← entry point, starts tokio runtime
│       ├── lib.rs           ← Tauri builder, AppState, command registration
│       ├── parser.rs        ← binary packet → TelemetryPacket struct
│       ├── udp.rs           ← tokio UDP loop, emits telemetry_tick events
│       ├── db.rs            ← SQLite init, session CRUD, packet insert
│       ├── session.rs       ← IsRaceOn state machine, ties udp+db together
│       ├── commands.rs      ← Tauri commands exposed to frontend
│       └── settings.rs      ← Settings struct, load/save to JSON
├── src/
│   ├── app.html
│   ├── routes/
│   │   └── +page.svelte     ← root dashboard page
│   └── lib/
│       ├── types.ts         ← TypeScript types mirroring Rust structs
│       ├── stores/
│       │   ├── telemetry.ts ← writable store + Tauri event listener
│       │   └── sessions.ts  ← session list store + Tauri command wrappers
│       ├── car-ordinals.json
│       └── components/
│           ├── TopBar.svelte
│           ├── InputStrip.svelte
│           ├── CenterPanel.svelte
│           ├── TireWidget.svelte
│           ├── LapBar.svelte
│           ├── SessionDrawer.svelte
│           └── SettingsModal.svelte
├── package.json
├── svelte.config.js
├── vite.config.ts
└── tsconfig.json
```

---

## Task 1: Project Scaffolding

**Files:**
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/build.rs`
- Create: `package.json`
- Create: `svelte.config.js`
- Create: `vite.config.ts`
- Create: `tsconfig.json`

- [ ] **Step 1: Scaffold Tauri 2 + Svelte project**

From `H:\fh6-tel`, run (non-interactive):
```
npm create tauri-app@latest . -- --template svelte-ts --manager npm --yes
```

If the command prompts interactively, answer:
- App name: `fh6-tel`
- Identifier: `ai.survyo.fh6-tel`
- Frontend language: TypeScript
- Package manager: npm
- UI template: Svelte

- [ ] **Step 2: Replace `src-tauri/Cargo.toml` dependencies**

```toml
[package]
name = "fh6-tel"
version = "0.1.0"
edition = "2021"

[lib]
name = "fh6_tel_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[dependencies]
tauri = { version = "2", features = ["tray-icon"] }
tauri-plugin-shell = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
rusqlite = { version = "0.31", features = ["bundled"] }
byteorder = "1"
dirs = "5"
thiserror = "1"

[build-dependencies]
tauri-build = { version = "2", features = [] }

[profile.release]
strip = true
opt-level = "s"
lto = true
```

- [ ] **Step 3: Install frontend dependencies**

```
npm install
npm install uplot
npm install --save-dev @types/uplot
```

- [ ] **Step 4: Replace `svelte.config.js`**

```javascript
import adapter from '@sveltejs/adapter-static';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';

/** @type {import('@sveltejs/kit').Config} */
const config = {
  preprocess: vitePreprocess(),
  kit: {
    adapter: adapter({ fallback: 'index.html' }),
  },
};

export default config;
```

- [ ] **Step 5: Replace `vite.config.ts`**

```typescript
import { defineConfig } from 'vite';
import { sveltekit } from '@sveltejs/kit/vite';

const host = process.env.TAURI_DEV_HOST;

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host ? { protocol: 'ws', host, port: 1421 } : undefined,
    watch: { ignored: ['**/src-tauri/**'] },
  },
});
```

- [ ] **Step 6: Verify the scaffold builds**

```
npm run tauri dev
```

Expected: Tauri window opens with the default Svelte template. Close it and continue.

---

## Task 2: Packet Parser

**Files:**
- Create: `src-tauri/src/parser.rs`

The FH5/FH6 "Dash" packet is little-endian. The first 232 bytes are the "Sled" (motion platform data), bytes 232–310 are Dash-only fields, and bytes 311+ are optional tire wear fields (present in 324-byte packets).

- [ ] **Step 1: Write the failing test**

Add to `src-tauri/src/parser.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn zero_packet(len: usize) -> Vec<u8> {
        vec![0u8; len]
    }

    fn packet_with_speed(speed_ms: f32) -> Vec<u8> {
        let mut buf = zero_packet(311);
        // Speed is at byte offset 244 in the Dash packet
        buf[244..248].copy_from_slice(&speed_ms.to_le_bytes());
        buf
    }

    #[test]
    fn rejects_short_packet() {
        let buf = zero_packet(100);
        assert!(parse(&buf).is_err());
    }

    #[test]
    fn parses_speed_field() {
        let buf = packet_with_speed(44.44); // ~100 mph
        let pkt = parse(&buf).unwrap();
        assert!((pkt.speed_ms - 44.44).abs() < 0.001);
    }

    #[test]
    fn accepts_311_byte_packet() {
        let buf = zero_packet(311);
        assert!(parse(&buf).is_ok());
    }

    #[test]
    fn accepts_324_byte_packet_with_tire_wear() {
        let mut buf = zero_packet(324);
        buf[311..315].copy_from_slice(&0.85f32.to_le_bytes()); // TireWearFL
        let pkt = parse(&buf).unwrap();
        assert!((pkt.tire_wear_fl.unwrap() - 0.85).abs() < 0.001);
    }

    #[test]
    fn is_race_on_zero_parses_as_false() {
        let buf = zero_packet(311);
        let pkt = parse(&buf).unwrap();
        assert!(!pkt.is_race_on);
    }

    #[test]
    fn is_race_on_one_parses_as_true() {
        let mut buf = zero_packet(311);
        buf[0..4].copy_from_slice(&1i32.to_le_bytes());
        let pkt = parse(&buf).unwrap();
        assert!(pkt.is_race_on);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```
cd src-tauri && cargo test parser -- --nocapture 2>&1
```

Expected: compile error — `parse`, `TelemetryPacket` not defined yet.

- [ ] **Step 3: Implement `parser.rs`**

```rust
use byteorder::{LittleEndian, ReadBytesExt};
use serde::Serialize;
use std::io::{Cursor, Read};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ParseError {
    #[error("packet too short: {0} bytes (need ≥311)")]
    TooShort(usize),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TelemetryPacket {
    pub is_race_on: bool,
    pub timestamp_ms: u32,
    pub engine_max_rpm: f32,
    pub engine_idle_rpm: f32,
    pub current_engine_rpm: f32,
    pub accel_x: f32,
    pub accel_y: f32,
    pub accel_z: f32,
    pub vel_x: f32,
    pub vel_y: f32,
    pub vel_z: f32,
    pub tire_slip_ratio_fl: f32,
    pub tire_slip_ratio_fr: f32,
    pub tire_slip_ratio_rl: f32,
    pub tire_slip_ratio_rr: f32,
    pub tire_slip_angle_fl: f32,
    pub tire_slip_angle_fr: f32,
    pub tire_slip_angle_rl: f32,
    pub tire_slip_angle_rr: f32,
    pub car_ordinal: i32,
    pub car_class: i32,
    pub car_pi: i32,
    pub drivetrain_type: i32,
    pub speed_ms: f32,
    pub power: f32,
    pub torque: f32,
    pub tire_temp_fl: f32,
    pub tire_temp_fr: f32,
    pub tire_temp_rl: f32,
    pub tire_temp_rr: f32,
    pub boost: f32,
    pub fuel: f32,
    pub distance_traveled: f32,
    pub best_lap: f32,
    pub last_lap: f32,
    pub current_lap: f32,
    pub current_race_time: f32,
    pub lap_number: u16,
    pub race_position: u8,
    pub throttle: u8,
    pub brake: u8,
    pub clutch: u8,
    pub handbrake: u8,
    pub gear: u8,
    pub tire_wear_fl: Option<f32>,
    pub tire_wear_fr: Option<f32>,
    pub tire_wear_rl: Option<f32>,
    pub tire_wear_rr: Option<f32>,
}

pub fn parse(buf: &[u8]) -> Result<TelemetryPacket, ParseError> {
    if buf.len() < 311 {
        return Err(ParseError::TooShort(buf.len()));
    }
    let mut c = Cursor::new(buf);

    // Sled fields (bytes 0–231)
    let is_race_on = c.read_i32::<LittleEndian>()? != 0;
    let timestamp_ms = c.read_u32::<LittleEndian>()?;
    let engine_max_rpm = c.read_f32::<LittleEndian>()?;
    let engine_idle_rpm = c.read_f32::<LittleEndian>()?;
    let current_engine_rpm = c.read_f32::<LittleEndian>()?;
    let accel_x = c.read_f32::<LittleEndian>()?;
    let accel_y = c.read_f32::<LittleEndian>()?;
    let accel_z = c.read_f32::<LittleEndian>()?;
    let vel_x = c.read_f32::<LittleEndian>()?;
    let vel_y = c.read_f32::<LittleEndian>()?;
    let vel_z = c.read_f32::<LittleEndian>()?;
    skip(&mut c, 3)?; // AngularVelocity X/Y/Z
    skip(&mut c, 3)?; // Yaw/Pitch/Roll
    skip(&mut c, 4)?; // NormalizedSuspensionTravel FL/FR/RL/RR
    let tire_slip_ratio_fl = c.read_f32::<LittleEndian>()?;
    let tire_slip_ratio_fr = c.read_f32::<LittleEndian>()?;
    let tire_slip_ratio_rl = c.read_f32::<LittleEndian>()?;
    let tire_slip_ratio_rr = c.read_f32::<LittleEndian>()?;
    skip(&mut c, 4)?; // WheelRotationSpeed
    skip(&mut c, 4)?; // WheelOnRumbleStrip
    skip(&mut c, 4)?; // WheelInPuddleDepth
    skip(&mut c, 4)?; // SurfaceRumble
    let tire_slip_angle_fl = c.read_f32::<LittleEndian>()?;
    let tire_slip_angle_fr = c.read_f32::<LittleEndian>()?;
    let tire_slip_angle_rl = c.read_f32::<LittleEndian>()?;
    let tire_slip_angle_rr = c.read_f32::<LittleEndian>()?;
    skip(&mut c, 4)?; // TireCombinedSlip
    skip(&mut c, 4)?; // SuspensionTravelMeters
    let car_ordinal = c.read_i32::<LittleEndian>()?;
    let car_class = c.read_i32::<LittleEndian>()?;
    let car_pi = c.read_i32::<LittleEndian>()?;
    let drivetrain_type = c.read_i32::<LittleEndian>()?;
    let _num_cylinders = c.read_i32::<LittleEndian>()?;

    // Dash-only fields (bytes 232–310)
    skip(&mut c, 3)?; // Position X/Y/Z
    let speed_ms = c.read_f32::<LittleEndian>()?;
    let power = c.read_f32::<LittleEndian>()?;
    let torque = c.read_f32::<LittleEndian>()?;
    let tire_temp_fl = c.read_f32::<LittleEndian>()?;
    let tire_temp_fr = c.read_f32::<LittleEndian>()?;
    let tire_temp_rl = c.read_f32::<LittleEndian>()?;
    let tire_temp_rr = c.read_f32::<LittleEndian>()?;
    let boost = c.read_f32::<LittleEndian>()?;
    let fuel = c.read_f32::<LittleEndian>()?;
    let distance_traveled = c.read_f32::<LittleEndian>()?;
    let best_lap = c.read_f32::<LittleEndian>()?;
    let last_lap = c.read_f32::<LittleEndian>()?;
    let current_lap = c.read_f32::<LittleEndian>()?;
    let current_race_time = c.read_f32::<LittleEndian>()?;
    let lap_number = c.read_u16::<LittleEndian>()?;
    let race_position = c.read_u8()?;
    let throttle = c.read_u8()?;
    let brake = c.read_u8()?;
    let clutch = c.read_u8()?;
    let handbrake = c.read_u8()?;
    let gear = c.read_u8()?;
    let _steer = c.read_i8()?;
    let _driving_line = c.read_i8()?;
    let _ai_brake_diff = c.read_i8()?;
    // Now at byte 311

    // Optional tire wear (bytes 311–326, present in 324/327-byte packets)
    let tire_wear_fl = if buf.len() >= 315 { Some(c.read_f32::<LittleEndian>()?) } else { None };
    let tire_wear_fr = if buf.len() >= 319 { Some(c.read_f32::<LittleEndian>()?) } else { None };
    let tire_wear_rl = if buf.len() >= 323 { Some(c.read_f32::<LittleEndian>()?) } else { None };
    let tire_wear_rr = if buf.len() >= 327 { Some(c.read_f32::<LittleEndian>()?) } else { None };

    Ok(TelemetryPacket {
        is_race_on,
        timestamp_ms,
        engine_max_rpm,
        engine_idle_rpm,
        current_engine_rpm,
        accel_x,
        accel_y,
        accel_z,
        vel_x,
        vel_y,
        vel_z,
        tire_slip_ratio_fl,
        tire_slip_ratio_fr,
        tire_slip_ratio_rl,
        tire_slip_ratio_rr,
        tire_slip_angle_fl,
        tire_slip_angle_fr,
        tire_slip_angle_rl,
        tire_slip_angle_rr,
        car_ordinal,
        car_class,
        car_pi,
        drivetrain_type,
        speed_ms,
        power,
        torque,
        tire_temp_fl,
        tire_temp_fr,
        tire_temp_rl,
        tire_temp_rr,
        boost,
        fuel,
        distance_traveled,
        best_lap,
        last_lap,
        current_lap,
        current_race_time,
        lap_number,
        race_position,
        throttle,
        brake,
        clutch,
        handbrake,
        gear,
        tire_wear_fl,
        tire_wear_fr,
        tire_wear_rl,
        tire_wear_rr,
    })
}

fn skip(c: &mut Cursor<&[u8]>, count: usize) -> std::io::Result<()> {
    let mut sink = vec![0u8; count * 4];
    c.read_exact(&mut sink)
}
```

- [ ] **Step 4: Run tests to verify they pass**

```
cd src-tauri && cargo test parser -- --nocapture 2>&1
```

Expected: all 6 tests pass.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/parser.rs
git commit -m "feat: add FH6 Dash packet parser with byte-level tests"
```

---

## Task 3: Settings

**Files:**
- Create: `src-tauri/src/settings.rs`

- [ ] **Step 1: Write the failing test**

Add to the bottom of `src-tauri/src/settings.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port_is_20440() {
        let s = Settings::default();
        assert_eq!(s.port, 20440);
    }

    #[test]
    fn roundtrip_to_json() {
        let s = Settings { port: 9999, use_mph: false, ..Settings::default() };
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s2.port, 9999);
        assert!(!s2.use_mph);
    }
}
```

- [ ] **Step 2: Run to verify failure**

```
cd src-tauri && cargo test settings -- --nocapture 2>&1
```

Expected: compile error — `Settings` not defined.

- [ ] **Step 3: Implement `settings.rs`**

```rust
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    pub port: u16,
    pub use_mph: bool,
    pub tire_temp_cold: f32,
    pub tire_temp_optimal: f32,
    pub tire_temp_hot: f32,
    pub auto_record: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            port: 20440,
            use_mph: true,
            tire_temp_cold: 60.0,
            tire_temp_optimal: 85.0,
            tire_temp_hot: 110.0,
            auto_record: true,
        }
    }
}

#[derive(Debug, Error)]
pub enum SettingsError {
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
}

fn settings_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fh6-tel")
        .join("settings.json")
}

pub fn load() -> Settings {
    let path = settings_path();
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(s: &Settings) -> Result<(), SettingsError> {
    let path = settings_path();
    std::fs::create_dir_all(path.parent().unwrap())?;
    std::fs::write(&path, serde_json::to_string_pretty(s)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_port_is_20440() {
        let s = Settings::default();
        assert_eq!(s.port, 20440);
    }

    #[test]
    fn roundtrip_to_json() {
        let s = Settings { port: 9999, use_mph: false, ..Settings::default() };
        let json = serde_json::to_string(&s).unwrap();
        let s2: Settings = serde_json::from_str(&json).unwrap();
        assert_eq!(s2.port, 9999);
        assert!(!s2.use_mph);
    }
}
```

- [ ] **Step 4: Run tests**

```
cd src-tauri && cargo test settings -- --nocapture 2>&1
```

Expected: 2 tests pass.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/settings.rs
git commit -m "feat: add Settings struct with load/save"
```

---

## Task 4: Database

**Files:**
- Create: `src-tauri/src/db.rs`

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory() -> rusqlite::Connection {
        let conn = rusqlite::Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn
    }

    #[test]
    fn init_creates_tables() {
        let conn = in_memory();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('sessions','session_packets')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn open_and_close_session() {
        let conn = in_memory();
        let id = open_session(&conn, 12345, 3, 900).unwrap();
        assert!(id > 0);
        close_session(&conn, id, 1000, 78.5).unwrap();
        let ended: Option<i64> = conn
            .query_row("SELECT ended_at FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert!(ended.is_some());
    }

    #[test]
    fn insert_and_count_packets() {
        let conn = in_memory();
        let id = open_session(&conn, 0, 0, 0).unwrap();
        let blob = vec![0u8; 311];
        insert_packet(&conn, id, 1000, &blob).unwrap();
        insert_packet(&conn, id, 2000, &blob).unwrap();
        let count: i64 = conn
            .query_row("SELECT packet_count FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}
```

- [ ] **Step 2: Run to verify failure**

```
cd src-tauri && cargo test db -- --nocapture 2>&1
```

Expected: compile error.

- [ ] **Step 3: Implement `db.rs`**

```rust
use rusqlite::{Connection, Result};
use std::path::PathBuf;

pub fn db_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("fh6-tel")
        .join("sessions.db")
}

pub fn open() -> Result<Connection> {
    let path = db_path();
    std::fs::create_dir_all(path.parent().unwrap()).ok();
    let conn = Connection::open(&path)?;
    conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
    init(&conn)?;
    Ok(conn)
}

pub fn init(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS sessions (
            id INTEGER PRIMARY KEY,
            started_at INTEGER NOT NULL,
            ended_at INTEGER,
            car_ordinal INTEGER NOT NULL DEFAULT 0,
            car_class INTEGER NOT NULL DEFAULT 0,
            car_pi INTEGER NOT NULL DEFAULT 0,
            best_lap REAL,
            packet_count INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS session_packets (
            id INTEGER PRIMARY KEY,
            session_id INTEGER NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
            timestamp_ms INTEGER NOT NULL,
            data BLOB NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_packets_session ON session_packets(session_id);",
    )
}

pub fn open_session(
    conn: &Connection,
    started_at: i64,
    car_ordinal: i32,
    car_pi: i32,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO sessions (started_at, car_ordinal, car_pi) VALUES (?1, ?2, ?3)",
        rusqlite::params![started_at, car_ordinal, car_pi],
    )?;
    Ok(conn.last_insert_rowid())
}

pub fn close_session(conn: &Connection, id: i64, ended_at: i64, best_lap: f32) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET ended_at=?1, best_lap=?2 WHERE id=?3",
        rusqlite::params![ended_at, best_lap, id],
    )?;
    Ok(())
}

pub fn insert_packet(
    conn: &Connection,
    session_id: i64,
    timestamp_ms: u32,
    data: &[u8],
) -> Result<()> {
    conn.execute(
        "INSERT INTO session_packets (session_id, timestamp_ms, data) VALUES (?1, ?2, ?3)",
        rusqlite::params![session_id, timestamp_ms, data],
    )?;
    conn.execute(
        "UPDATE sessions SET packet_count = packet_count + 1 WHERE id=?1",
        [session_id],
    )?;
    Ok(())
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRow {
    pub id: i64,
    pub started_at: i64,
    pub ended_at: Option<i64>,
    pub car_ordinal: i32,
    pub car_class: i32,
    pub car_pi: i32,
    pub best_lap: Option<f32>,
    pub packet_count: i64,
}

pub fn list_sessions(conn: &Connection) -> Result<Vec<SessionRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, started_at, ended_at, car_ordinal, car_class, car_pi, best_lap, packet_count
         FROM sessions ORDER BY started_at DESC LIMIT 100",
    )?;
    let rows = stmt.query_map([], |r| {
        Ok(SessionRow {
            id: r.get(0)?,
            started_at: r.get(1)?,
            ended_at: r.get(2)?,
            car_ordinal: r.get(3)?,
            car_class: r.get(4)?,
            car_pi: r.get(5)?,
            best_lap: r.get(6)?,
            packet_count: r.get(7)?,
        })
    })?;
    rows.collect()
}

pub fn get_session_packets(conn: &Connection, session_id: i64) -> Result<Vec<Vec<u8>>> {
    let mut stmt = conn.prepare(
        "SELECT data FROM session_packets WHERE session_id=?1 ORDER BY timestamp_ms ASC",
    )?;
    let rows = stmt.query_map([session_id], |r| r.get::<_, Vec<u8>>(0))?;
    rows.collect()
}

pub fn delete_session(conn: &Connection, id: i64) -> Result<()> {
    conn.execute("DELETE FROM sessions WHERE id=?1", [id])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_memory() -> Connection {
        let conn = Connection::open_in_memory().unwrap();
        init(&conn).unwrap();
        conn
    }

    #[test]
    fn init_creates_tables() {
        let conn = in_memory();
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name IN ('sessions','session_packets')",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 2);
    }

    #[test]
    fn open_and_close_session() {
        let conn = in_memory();
        let id = open_session(&conn, 12345, 3, 900).unwrap();
        assert!(id > 0);
        close_session(&conn, id, 1000, 78.5).unwrap();
        let ended: Option<i64> = conn
            .query_row("SELECT ended_at FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert!(ended.is_some());
    }

    #[test]
    fn insert_and_count_packets() {
        let conn = in_memory();
        let id = open_session(&conn, 0, 0, 0).unwrap();
        let blob = vec![0u8; 311];
        insert_packet(&conn, id, 1000, &blob).unwrap();
        insert_packet(&conn, id, 2000, &blob).unwrap();
        let count: i64 = conn
            .query_row("SELECT packet_count FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}
```

- [ ] **Step 4: Run tests**

```
cd src-tauri && cargo test db -- --nocapture 2>&1
```

Expected: 3 tests pass.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/db.rs
git commit -m "feat: add SQLite schema and session CRUD"
```

---

## Task 5: Session Manager

**Files:**
- Create: `src-tauri/src/session.rs`

Watches `is_race_on` transitions. `0→1` opens a session; `1→0` closes it. Also tracks the running best lap within the session.

- [ ] **Step 1: Write the failing test**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_session_when_not_racing() {
        let mut sm = SessionManager::new(true);
        assert!(sm.active_session_id().is_none());
    }

    #[test]
    fn opens_session_on_race_start() {
        let mut sm = SessionManager::new(true);
        let action = sm.on_race_on_change(false, true, 99, 800);
        assert!(matches!(action, SessionAction::Open { car_ordinal: 99, .. }));
    }

    #[test]
    fn closes_session_on_race_end() {
        let mut sm = SessionManager::new(true);
        sm.on_race_on_change(false, true, 0, 0);
        let action = sm.on_race_on_change(true, false, 0, 0);
        assert!(matches!(action, SessionAction::Close { .. }));
    }

    #[test]
    fn no_action_when_race_on_unchanged() {
        let mut sm = SessionManager::new(true);
        let action = sm.on_race_on_change(true, true, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }

    #[test]
    fn disabled_auto_record_never_opens() {
        let mut sm = SessionManager::new(false);
        let action = sm.on_race_on_change(false, true, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }
}
```

- [ ] **Step 2: Run to verify failure**

```
cd src-tauri && cargo test session -- --nocapture 2>&1
```

Expected: compile error.

- [ ] **Step 3: Implement `session.rs`**

```rust
pub enum SessionAction {
    Open { car_ordinal: i32, car_pi: i32 },
    Close { best_lap: f32 },
    None,
}

pub struct SessionManager {
    auto_record: bool,
    active_id: Option<i64>,
    best_lap: f32,
}

impl SessionManager {
    pub fn new(auto_record: bool) -> Self {
        Self { auto_record, active_id: None, best_lap: f32::MAX }
    }

    pub fn active_session_id(&self) -> Option<i64> {
        self.active_id
    }

    pub fn set_active_id(&mut self, id: Option<i64>) {
        self.active_id = id;
        if id.is_none() {
            self.best_lap = f32::MAX;
        }
    }

    pub fn update_best_lap(&mut self, lap: f32) {
        if lap > 0.0 && lap < self.best_lap {
            self.best_lap = lap;
        }
    }

    pub fn on_race_on_change(
        &mut self,
        was_racing: bool,
        is_racing: bool,
        car_ordinal: i32,
        car_pi: i32,
    ) -> SessionAction {
        match (was_racing, is_racing) {
            (false, true) if self.auto_record => SessionAction::Open { car_ordinal, car_pi },
            (true, false) if self.active_id.is_some() => {
                let best = if self.best_lap == f32::MAX { -1.0 } else { self.best_lap };
                SessionAction::Close { best_lap: best }
            }
            _ => SessionAction::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_session_when_not_racing() {
        let sm = SessionManager::new(true);
        assert!(sm.active_session_id().is_none());
    }

    #[test]
    fn opens_session_on_race_start() {
        let mut sm = SessionManager::new(true);
        let action = sm.on_race_on_change(false, true, 99, 800);
        assert!(matches!(action, SessionAction::Open { car_ordinal: 99, .. }));
    }

    #[test]
    fn closes_session_on_race_end() {
        let mut sm = SessionManager::new(true);
        sm.on_race_on_change(false, true, 0, 0);
        sm.set_active_id(Some(1));
        let action = sm.on_race_on_change(true, false, 0, 0);
        assert!(matches!(action, SessionAction::Close { .. }));
    }

    #[test]
    fn no_action_when_race_on_unchanged() {
        let mut sm = SessionManager::new(true);
        let action = sm.on_race_on_change(true, true, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }

    #[test]
    fn disabled_auto_record_never_opens() {
        let mut sm = SessionManager::new(false);
        let action = sm.on_race_on_change(false, true, 0, 0);
        assert!(matches!(action, SessionAction::None));
    }
}
```

- [ ] **Step 4: Run tests**

```
cd src-tauri && cargo test session -- --nocapture 2>&1
```

Expected: 5 tests pass.

- [ ] **Step 5: Commit**

```
git add src-tauri/src/session.rs
git commit -m "feat: add IsRaceOn session state machine"
```

---

## Task 6: UDP Listener

**Files:**
- Create: `src-tauri/src/udp.rs`

Spawns a tokio task that loops on a UDP socket. On each datagram: parse, emit Tauri event, notify SessionManager, write to DB if a session is open.

- [ ] **Step 1: Implement `udp.rs`**

There is no meaningful unit test for the live UDP loop (it requires a socket and a running Tauri app). The integration test is Task 6 Step 3 — sending a real UDP packet to the running app.

```rust
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Manager};
use tokio::net::UdpSocket;

use crate::{
    db,
    parser::{self, TelemetryPacket},
    session::{SessionAction, SessionManager},
    AppState,
};

pub async fn run(app: AppHandle, port: u16) {
    let addr = format!("0.0.0.0:{port}");
    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[udp] failed to bind {addr}: {e}");
            return;
        }
    };
    println!("[udp] listening on {addr}");

    let mut buf = vec![0u8; 1024];
    let mut prev_race_on = false;

    loop {
        let (len, _) = match socket.recv_from(&mut buf).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[udp] recv error: {e}");
                continue;
            }
        };

        let raw = &buf[..len];
        let pkt = match parser::parse(raw) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Always emit live data regardless of session state
        let _ = app.emit("telemetry_tick", &pkt);

        // Session management
        let state = app.state::<AppState>();
        handle_session(&state, &pkt, raw, prev_race_on);
        prev_race_on = pkt.is_race_on;
    }
}

fn handle_session(state: &AppState, pkt: &TelemetryPacket, raw: &[u8], prev_race_on: bool) {
    let mut sm = state.session_manager.lock().unwrap();
    let db = state.db.lock().unwrap();

    if let Some(session_id) = sm.active_session_id() {
        sm.update_best_lap(pkt.best_lap);
        let _ = db::insert_packet(&db, session_id, pkt.timestamp_ms, raw);
    }

    let action = sm.on_race_on_change(prev_race_on, pkt.is_race_on, pkt.car_ordinal, pkt.car_pi);

    match action {
        SessionAction::Open { car_ordinal, car_pi } => {
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_millis() as i64;
            match db::open_session(&db, now, car_ordinal, car_pi) {
                Ok(id) => {
                    sm.set_active_id(Some(id));
                    println!("[session] opened #{id}");
                }
                Err(e) => eprintln!("[session] open error: {e}"),
            }
        }
        SessionAction::Close { best_lap } => {
            if let Some(id) = sm.active_session_id() {
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_millis() as i64;
                let _ = db::close_session(&db, id, now, best_lap);
                println!("[session] closed #{id}");
            }
            sm.set_active_id(None);
        }
        SessionAction::None => {}
    }
}
```

- [ ] **Step 2: Verify it compiles (no tests yet — integration test is in Step 3)**

```
cd src-tauri && cargo build 2>&1
```

Expected: compiles (will fail on missing `AppState` — that's fine, defined in Task 7).

- [ ] **Step 3: Commit**

```
git add src-tauri/src/udp.rs
git commit -m "feat: add UDP listener with session integration"
```

---

## Task 7: Tauri Commands and App State

**Files:**
- Create: `src-tauri/src/commands.rs`
- Modify: `src-tauri/src/lib.rs`
- Modify: `src-tauri/src/main.rs`

- [ ] **Step 1: Implement `commands.rs`**

```rust
use tauri::State;
use crate::{db, settings, AppState};

#[tauri::command]
pub fn get_sessions(state: State<AppState>) -> Result<Vec<db::SessionRow>, String> {
    let conn = state.db.lock().unwrap();
    db::list_sessions(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_session_packets(
    state: State<AppState>,
    session_id: i64,
) -> Result<Vec<crate::parser::TelemetryPacket>, String> {
    let conn = state.db.lock().unwrap();
    let blobs = db::get_session_packets(&conn, session_id).map_err(|e| e.to_string())?;
    blobs
        .iter()
        .filter_map(|b| crate::parser::parse(b).ok())
        .collect::<Vec<_>>()
        .pipe(Ok)
}

#[tauri::command]
pub fn delete_session(state: State<AppState>, session_id: i64) -> Result<(), String> {
    let conn = state.db.lock().unwrap();
    db::delete_session(&conn, session_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_settings(state: State<AppState>) -> settings::Settings {
    state.settings.lock().unwrap().clone()
}

#[tauri::command]
pub fn save_settings(
    state: State<AppState>,
    new_settings: settings::Settings,
) -> Result<(), String> {
    settings::save(&new_settings).map_err(|e| e.to_string())?;
    *state.settings.lock().unwrap() = new_settings;
    Ok(())
}

trait Pipe: Sized {
    fn pipe<F: FnOnce(Self) -> R, R>(self, f: F) -> R {
        f(self)
    }
}
impl<T> Pipe for T {}
```

- [ ] **Step 2: Implement `lib.rs`**

```rust
mod commands;
mod db;
mod parser;
mod session;
mod settings;
mod udp;

use session::SessionManager;
use std::sync::Mutex;
use tauri::Manager;

pub struct AppState {
    pub db: Mutex<rusqlite::Connection>,
    pub session_manager: Mutex<SessionManager>,
    pub settings: Mutex<settings::Settings>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let loaded_settings = settings::load();
    let port = loaded_settings.port;
    let auto_record = loaded_settings.auto_record;

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AppState {
            db: Mutex::new(db::open().expect("failed to open database")),
            session_manager: Mutex::new(SessionManager::new(auto_record)),
            settings: Mutex::new(loaded_settings),
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_sessions,
            commands::get_session_packets,
            commands::delete_session,
            commands::get_settings,
            commands::save_settings,
        ])
        .setup(|app| {
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                udp::run(handle, port).await;
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error running tauri app");
}
```

- [ ] **Step 3: Implement `main.rs`**

```rust
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    fh6_tel_lib::run();
}
```

- [ ] **Step 4: Verify full Rust build**

```
cd src-tauri && cargo build 2>&1
```

Expected: compiles with no errors.

- [ ] **Step 5: Smoke-test end-to-end with a UDP packet**

In a separate PowerShell terminal, run the app in dev mode:
```
npm run tauri dev
```

Then in another terminal, send a 311-byte test packet:
```powershell
$udp = New-Object System.Net.Sockets.UdpClient
$buf = [byte[]]::new(311)
# Set IsRaceOn = 1 (bytes 0-3 little-endian)
$buf[0] = 1
# Set Speed = 44.44 m/s at offset 244
$speedBytes = [BitConverter]::GetBytes([float]44.44)
[Array]::Copy($speedBytes, 0, $buf, 244, 4)
$udp.Send($buf, $buf.Length, "127.0.0.1", 20440)
$udp.Close()
```

Expected: Rust console prints `[udp] listening on 0.0.0.0:20440`. No crash. (Frontend not wired yet.)

- [ ] **Step 6: Commit**

```
git add src-tauri/src/commands.rs src-tauri/src/lib.rs src-tauri/src/main.rs
git commit -m "feat: wire AppState, commands, and UDP startup into Tauri"
```

---

## Task 8: Tauri Configuration

**Files:**
- Modify: `src-tauri/tauri.conf.json`
- Create: `src-tauri/capabilities/default.json`

- [ ] **Step 1: Replace `src-tauri/tauri.conf.json`**

```json
{
  "productName": "FH6 Telemetry",
  "version": "0.1.0",
  "identifier": "ai.survyo.fh6-tel",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../build"
  },
  "app": {
    "windows": [
      {
        "title": "FH6 Telemetry",
        "width": 1440,
        "height": 900,
        "minWidth": 1024,
        "minHeight": 600,
        "resizable": true,
        "fullscreen": false,
        "decorations": true
      }
    ],
    "security": {
      "csp": null
    }
  },
  "bundle": {
    "active": true,
    "targets": ["nsis", "msi"],
    "icon": [
      "icons/32x32.png",
      "icons/128x128.png",
      "icons/128x128@2x.png",
      "icons/icon.icns",
      "icons/icon.ico"
    ],
    "windows": {
      "nsis": {
        "installMode": "currentUser"
      }
    }
  }
}
```

- [ ] **Step 2: Create `src-tauri/capabilities/default.json`**

```json
{
  "$schema": "../gen/schemas/desktop-schema.json",
  "identifier": "default",
  "description": "Default capabilities",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open"
  ]
}
```

- [ ] **Step 3: Verify dev mode still opens**

```
npm run tauri dev
```

Expected: window opens with default Svelte page, no errors in console.

- [ ] **Step 4: Commit**

```
git add src-tauri/tauri.conf.json src-tauri/capabilities/default.json
git commit -m "chore: configure Tauri window, capabilities, and NSIS bundler"
```

---

## Task 9: Frontend Types and Stores

**Files:**
- Create: `src/lib/types.ts`
- Create: `src/lib/stores/telemetry.ts`
- Create: `src/lib/stores/sessions.ts`

- [ ] **Step 1: Create `src/lib/types.ts`**

```typescript
export interface TelemetryPacket {
  isRaceOn: boolean;
  timestampMs: number;
  engineMaxRpm: number;
  engineIdleRpm: number;
  currentEngineRpm: number;
  accelX: number;
  accelY: number;
  accelZ: number;
  velX: number;
  velY: number;
  velZ: number;
  tireSlipRatioFl: number;
  tireSlipRatioFr: number;
  tireSlipRatioRl: number;
  tireSlipRatioRr: number;
  tireSlipAngleFl: number;
  tireSlipAngleFr: number;
  tireSlipAngleRl: number;
  tireSlipAngleRr: number;
  carOrdinal: number;
  carClass: number;
  carPi: number;
  drivetrainType: number;
  speedMs: number;
  power: number;
  torque: number;
  tireTempFl: number;
  tireTempFr: number;
  tireTempRl: number;
  tireTempRr: number;
  boost: number;
  fuel: number;
  distanceTraveled: number;
  bestLap: number;
  lastLap: number;
  currentLap: number;
  currentRaceTime: number;
  lapNumber: number;
  racePosition: number;
  throttle: number;
  brake: number;
  clutch: number;
  handbrake: number;
  gear: number;
  tireWearFl: number | null;
  tireWearFr: number | null;
  tireWearRl: number | null;
  tireWearRr: number | null;
}

export interface SessionRow {
  id: number;
  startedAt: number;
  endedAt: number | null;
  carOrdinal: number;
  carClass: number;
  carPi: number;
  bestLap: number | null;
  packetCount: number;
}

export interface AppSettings {
  port: number;
  useMph: boolean;
  tireTempCold: number;
  tireTempOptimal: number;
  tireTempHot: number;
  autoRecord: boolean;
}

export type DrivetrainLabel = 'FWD' | 'RWD' | 'AWD';
export const DRIVETRAIN_LABELS: DrivetrainLabel[] = ['FWD', 'RWD', 'AWD'];

export type CarClassLabel = 'D' | 'C' | 'B' | 'A' | 'S1' | 'S2' | 'X';
export const CAR_CLASS_LABELS: CarClassLabel[] = ['D', 'C', 'B', 'A', 'S1', 'S2', 'X'];
```

- [ ] **Step 2: Create `src/lib/stores/telemetry.ts`**

```typescript
import { writable, derived } from 'svelte/store';
import { listen } from '@tauri-apps/api/event';
import type { TelemetryPacket } from '$lib/types';

export const packet = writable<TelemetryPacket | null>(null);
export const isConnected = writable(false);

export const speedMph = derived(packet, ($p) =>
  $p ? $p.speedMs * 2.23694 : 0
);

export const speedKph = derived(packet, ($p) =>
  $p ? $p.speedMs * 3.6 : 0
);

export const rpmPercent = derived(packet, ($p) => {
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

  connectionTimer = setInterval(() => {
    if (Date.now() - lastPacketTime > 2000) {
      isConnected.set(false);
    }
  }, 1000);
}
```

- [ ] **Step 3: Create `src/lib/stores/sessions.ts`**

```typescript
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

export async function loadSettings() {
  const s = await invoke<AppSettings>('get_settings');
  settings.set(s);
  return s;
}

export async function saveSettings(s: AppSettings) {
  await invoke('save_settings', { newSettings: s });
  settings.set(s);
}
```

- [ ] **Step 4: Commit**

```
git add src/lib/types.ts src/lib/stores/telemetry.ts src/lib/stores/sessions.ts
git commit -m "feat: add TypeScript types and Svelte stores for telemetry and sessions"
```

---

## Task 10: Car Ordinals Data

**Files:**
- Create: `src/lib/car-ordinals.json`

The full car list for FH6 is not yet published. Populate with a starter set from FH5 and a fallback.

- [ ] **Step 1: Create `src/lib/car-ordinals.json`**

```json
{
  "3": "Ford GT40 Mk II 1966",
  "4": "Lamborghini Countach LP5000 QV 1988",
  "5": "Ferrari 599XX Evolution",
  "6": "McLaren F1",
  "7": "Bugatti Veyron Super Sport",
  "8": "Koenigsegg One:1",
  "9": "Pagani Huayra BC",
  "10": "Mercedes-AMG Project ONE",
  "11": "Aston Martin Vulcan",
  "12": "Ferrari LaFerrari",
  "13": "McLaren P1",
  "14": "Porsche 918 Spyder",
  "15": "Toyota GR010 HYBRID",
  "100": "Subaru Impreza WRX STI 2005",
  "101": "Mitsubishi Lancer Evolution X",
  "102": "Ford Focus RS 2016",
  "103": "Honda Civic Type R 2017",
  "200": "Nissan GT-R NISMO",
  "201": "Porsche 911 GT3 RS",
  "202": "BMW M3 Competition",
  "203": "Mercedes-AMG GT R",
  "300": "Lamborghini Huracán Performante",
  "301": "Ferrari 488 Pista",
  "302": "McLaren 720S",
  "303": "Porsche 911 GT2 RS"
}
```

- [ ] **Step 2: Create `src/lib/car-name.ts`**

```typescript
import ordinals from '$lib/car-ordinals.json';

const map: Record<string, string> = ordinals as Record<string, string>;

export function carName(ordinal: number): string {
  return map[String(ordinal)] ?? `Car #${ordinal}`;
}
```

- [ ] **Step 3: Commit**

```
git add src/lib/car-ordinals.json src/lib/car-name.ts
git commit -m "feat: add car ordinal lookup with FH5/FH6 starter data"
```

---

## Task 11: TopBar Component

**Files:**
- Create: `src/lib/components/TopBar.svelte`

- [ ] **Step 1: Implement `TopBar.svelte`**

```svelte
<script lang="ts">
  import { isConnected, packet } from '$lib/stores/telemetry';
  import { carName } from '$lib/car-name';
  import { CAR_CLASS_LABELS, DRIVETRAIN_LABELS } from '$lib/types';

  let { useMph = true, onSettings, onSessions } = $props<{
    useMph: boolean;
    onSettings: () => void;
    onSessions: () => void;
  }>();

  const connected = $derived($isConnected);
  const pkt = $derived($packet);
  const carLabel = $derived(pkt ? carName(pkt.carOrdinal) : '—');
  const classLabel = $derived(pkt ? (CAR_CLASS_LABELS[pkt.carClass] ?? '?') : '—');
  const piLabel = $derived(pkt ? String(pkt.carPi) : '—');
  const driveLabel = $derived(pkt ? (DRIVETRAIN_LABELS[pkt.drivetrainType] ?? '?') : '—');
</script>

<header class="topbar">
  <div class="status">
    <span class="dot" class:live={connected}></span>
    <span class="label">{connected ? 'LIVE' : 'WAITING…'}</span>
  </div>

  <div class="car-info">
    <span class="car-name">{carLabel}</span>
    <span class="badge">{classLabel}</span>
    <span class="badge">{piLabel}</span>
    <span class="badge">{driveLabel}</span>
  </div>

  <div class="controls">
    <button class="icon-btn" onclick={onSessions} title="Sessions">⏱</button>
    <button class="icon-btn" onclick={onSettings} title="Settings">⚙</button>
  </div>
</header>

<style>
  .topbar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 1rem;
    height: 2.5rem;
    background: #0a0a0a;
    border-bottom: 1px solid #222;
    flex-shrink: 0;
  }
  .status { display: flex; align-items: center; gap: 0.4rem; }
  .dot {
    width: 8px; height: 8px; border-radius: 50%;
    background: #444;
    transition: background 0.3s;
  }
  .dot.live { background: #22c55e; box-shadow: 0 0 6px #22c55e; }
  .label { font-size: 0.7rem; font-weight: 700; letter-spacing: 0.1em; color: #888; }
  .car-info { display: flex; align-items: center; gap: 0.5rem; }
  .car-name { font-size: 0.85rem; font-weight: 600; color: #e5e7eb; }
  .badge {
    font-size: 0.65rem; font-weight: 700; padding: 0.1rem 0.4rem;
    border: 1px solid #333; border-radius: 3px; color: #9ca3af;
  }
  .controls { display: flex; gap: 0.25rem; }
  .icon-btn {
    background: none; border: none; cursor: pointer;
    font-size: 1rem; color: #6b7280; padding: 0.25rem 0.5rem;
    border-radius: 4px;
  }
  .icon-btn:hover { background: #1f2937; color: #e5e7eb; }
</style>
```

- [ ] **Step 2: Commit**

```
git add src/lib/components/TopBar.svelte
git commit -m "feat: add TopBar with live status, car info, and nav buttons"
```

---

## Task 12: InputStrip Component

**Files:**
- Create: `src/lib/components/InputStrip.svelte`

- [ ] **Step 1: Implement `InputStrip.svelte`**

Displays four vertical bars (Throttle, Brake, Clutch, Handbrake) as 0–255 values scaled to percentage.

```svelte
<script lang="ts">
  import { packet } from '$lib/stores/telemetry';

  const pkt = $derived($packet);

  interface Bar { label: string; value: number; color: string; }

  const bars = $derived<Bar[]>([
    { label: 'THR', value: pkt ? pkt.throttle / 255 : 0, color: '#22c55e' },
    { label: 'BRK', value: pkt ? pkt.brake / 255 : 0, color: '#ef4444' },
    { label: 'CLT', value: pkt ? pkt.clutch / 255 : 0, color: '#94a3b8' },
    { label: 'HBK', value: pkt ? pkt.handbrake / 255 : 0, color: '#f97316' },
  ]);
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
```

- [ ] **Step 2: Commit**

```
git add src/lib/components/InputStrip.svelte
git commit -m "feat: add InputStrip component for throttle/brake/clutch/handbrake"
```

---

## Task 13: CenterPanel Component

**Files:**
- Create: `src/lib/components/CenterPanel.svelte`

- [ ] **Step 1: Implement `CenterPanel.svelte`**

Shows speed (large), gear (large box), RPM bar with redline marker, and boost gauge.

```svelte
<script lang="ts">
  import { packet, speedMph, speedKph, rpmPercent } from '$lib/stores/telemetry';

  let { useMph = true } = $props<{ useMph: boolean }>();

  const pkt = $derived($packet);
  const speed = $derived(useMph ? Math.round($speedMph) : Math.round($speedKph));
  const unit = $derived(useMph ? 'mph' : 'kph');
  const rpm = $derived($rpmPercent);
  const gearLabel = $derived(() => {
    if (!pkt) return '—';
    if (pkt.gear === 0) return 'R';
    if (pkt.gear === 11) return 'N';
    return String(pkt.gear);
  });
  const boostBar = $derived(pkt ? Math.min(Math.max(pkt.boost / 2.0, 0), 1) : 0);
  const isRedline = $derived(rpm > 90);
</script>

<div class="center">
  <div class="speed-row">
    <div class="speed">{speed}</div>
    <div class="gear-box" class:redline={isRedline}>{gearLabel()}</div>
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
  .speed-row {
    display: flex;
    align-items: center;
    gap: 1.5rem;
  }
  .speed {
    font-size: 6rem;
    font-weight: 900;
    font-variant-numeric: tabular-nums;
    line-height: 1;
    color: #f9fafb;
    text-shadow: 0 0 40px rgba(255,255,255,0.1);
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
  .gear-box.redline {
    border-color: #ef4444;
    color: #ef4444;
    box-shadow: 0 0 12px rgba(239,68,68,0.4);
  }
  .unit-label {
    font-size: 0.8rem;
    font-weight: 700;
    letter-spacing: 0.15em;
    color: #6b7280;
    margin-top: -0.5rem;
  }
  .gauge-row {
    width: 100%;
    display: flex;
    flex-direction: column;
    gap: 0.4rem;
  }
  .gauge-group { display: flex; flex-direction: column; gap: 0.2rem; }
  .gauge-label { font-size: 0.65rem; color: #6b7280; font-weight: 700; letter-spacing: 0.1em; }
  .gauge-track {
    height: 12px;
    background: #1f2937;
    border-radius: 3px;
    overflow: hidden;
    position: relative;
  }
  .gauge-fill {
    height: 100%;
    border-radius: 3px;
    transition: width 33ms linear;
  }
  .rpm-fill { background: #3b82f6; }
  .rpm-fill.rpm-redline { background: #ef4444; }
  .boost-fill { background: #a855f7; }
  .redline-marker {
    position: absolute;
    right: 10%;
    top: 0;
    width: 2px;
    height: 100%;
    background: rgba(239,68,68,0.6);
  }
</style>
```

- [ ] **Step 2: Commit**

```
git add src/lib/components/CenterPanel.svelte
git commit -m "feat: add CenterPanel with speed, gear, RPM, and boost"
```

---

## Task 14: TireWidget Component

**Files:**
- Create: `src/lib/components/TireWidget.svelte`

- [ ] **Step 1: Implement `TireWidget.svelte`**

Four tiles in car-corner positions. Each tile shows temperature (color-coded) and slip ratio.

```svelte
<script lang="ts">
  import { packet } from '$lib/stores/telemetry';

  let {
    tireTempCold = 60,
    tireTempOptimal = 85,
    tireTempHot = 110,
  } = $props<{
    tireTempCold: number;
    tireTempOptimal: number;
    tireTempHot: number;
  }>();

  const pkt = $derived($packet);

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

  interface TireData { label: string; temp: number; slip: number; }

  const tires = $derived<TireData[]>([
    { label: 'FL', temp: pkt?.tireTempFl ?? 0, slip: pkt?.tireSlipRatioFl ?? 0 },
    { label: 'FR', temp: pkt?.tireTempFr ?? 0, slip: pkt?.tireSlipRatioFr ?? 0 },
    { label: 'RL', temp: pkt?.tireTempRl ?? 0, slip: pkt?.tireSlipRatioRl ?? 0 },
    { label: 'RR', temp: pkt?.tireTempRr ?? 0, slip: pkt?.tireSlipRatioRr ?? 0 },
  ]);
</script>

<div class="tire-grid">
  {#each tires as tire, i}
    <div class="tire-tile" style="grid-area: t{i}; border-color: {tempColor(tire.temp)};">
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
    grid-template-areas:
      "t0 t1"
      "t2 t3";
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
```

- [ ] **Step 2: Commit**

```
git add src/lib/components/TireWidget.svelte
git commit -m "feat: add TireWidget with temp color coding and slip indicator"
```

---

## Task 15: LapBar Component

**Files:**
- Create: `src/lib/components/LapBar.svelte`

- [ ] **Step 1: Implement `LapBar.svelte`**

```svelte
<script lang="ts">
  import { packet } from '$lib/stores/telemetry';

  const pkt = $derived($packet);

  function formatTime(seconds: number): string {
    if (seconds <= 0) return '—:——.———';
    const mins = Math.floor(seconds / 60);
    const secs = seconds % 60;
    return `${mins}:${secs.toFixed(3).padStart(6, '0')}`;
  }
</script>

<div class="lapbar">
  <div class="lap-item">
    <span class="lap-key">LAP</span>
    <span class="lap-val">{pkt ? pkt.lapNumber : '—'}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">CURRENT</span>
    <span class="lap-val current">{formatTime(pkt?.currentLap ?? 0)}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">LAST</span>
    <span class="lap-val">{formatTime(pkt?.lastLap ?? 0)}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">BEST</span>
    <span class="lap-val best">{formatTime(pkt?.bestLap ?? 0)}</span>
  </div>
  <div class="sep"></div>
  <div class="lap-item">
    <span class="lap-key">SESSION</span>
    <span class="lap-val">{formatTime(pkt?.currentRaceTime ?? 0)}</span>
  </div>
</div>

<style>
  .lapbar {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 0;
    height: 100%;
    background: #0a0a0a;
    border-top: 1px solid #222;
    padding: 0 1rem;
  }
  .lap-item {
    display: flex;
    flex-direction: column;
    align-items: center;
    padding: 0 1.5rem;
  }
  .lap-key {
    font-size: 0.55rem;
    font-weight: 700;
    letter-spacing: 0.15em;
    color: #6b7280;
  }
  .lap-val {
    font-size: 1.1rem;
    font-weight: 800;
    font-variant-numeric: tabular-nums;
    color: #e5e7eb;
  }
  .lap-val.current { color: #3b82f6; }
  .lap-val.best { color: #a855f7; }
  .sep { width: 1px; height: 2rem; background: #1f2937; }
</style>
```

- [ ] **Step 2: Commit**

```
git add src/lib/components/LapBar.svelte
git commit -m "feat: add LapBar with current, last, best, and session time"
```

---

## Task 16: SettingsModal Component

**Files:**
- Create: `src/lib/components/SettingsModal.svelte`

- [ ] **Step 1: Implement `SettingsModal.svelte`**

```svelte
<script lang="ts">
  import { settings, saveSettings } from '$lib/stores/sessions';
  import type { AppSettings } from '$lib/types';

  let { onClose } = $props<{ onClose: () => void }>();

  let draft = $state<AppSettings | null>(null);
  $effect(() => {
    if ($settings && !draft) draft = { ...$settings };
  });

  async function save() {
    if (!draft) return;
    await saveSettings(draft);
    onClose();
  }
</script>

{#if draft}
  <div class="overlay" role="dialog">
    <div class="modal">
      <h2>Settings</h2>

      <label>
        UDP Port
        <input type="number" bind:value={draft.port} min="1024" max="65535" />
      </label>

      <label>
        Units
        <select bind:value={draft.useMph}>
          <option value={true}>mph</option>
          <option value={false}>kph</option>
        </select>
      </label>

      <label>
        <input type="checkbox" bind:checked={draft.autoRecord} />
        Auto-record sessions
      </label>

      <fieldset>
        <legend>Tire Temp Thresholds (°C)</legend>
        <label>Cold below <input type="number" bind:value={draft.tireTempCold} /></label>
        <label>Optimal up to <input type="number" bind:value={draft.tireTempOptimal} /></label>
        <label>Hot above <input type="number" bind:value={draft.tireTempHot} /></label>
      </fieldset>

      <div class="actions">
        <button onclick={onClose}>Cancel</button>
        <button class="primary" onclick={save}>Save</button>
      </div>
    </div>
  </div>
{/if}

<style>
  .overlay {
    position: fixed; inset: 0; background: rgba(0,0,0,0.7);
    display: flex; align-items: center; justify-content: center;
    z-index: 100;
  }
  .modal {
    background: #111827; border: 1px solid #374151; border-radius: 10px;
    padding: 1.5rem; width: 360px; display: flex; flex-direction: column; gap: 1rem;
  }
  h2 { margin: 0; color: #f9fafb; font-size: 1.1rem; }
  label { display: flex; flex-direction: column; gap: 0.3rem; color: #d1d5db; font-size: 0.85rem; }
  input[type="number"], select {
    background: #1f2937; border: 1px solid #374151; border-radius: 4px;
    color: #f9fafb; padding: 0.4rem; font-size: 0.9rem;
  }
  input[type="checkbox"] { width: auto; }
  fieldset { border: 1px solid #374151; border-radius: 6px; padding: 0.75rem; display: flex; flex-direction: column; gap: 0.5rem; }
  legend { color: #9ca3af; font-size: 0.75rem; padding: 0 0.25rem; }
  .actions { display: flex; justify-content: flex-end; gap: 0.5rem; }
  button {
    padding: 0.4rem 1rem; border-radius: 5px; border: 1px solid #374151;
    background: #1f2937; color: #d1d5db; cursor: pointer; font-size: 0.85rem;
  }
  button.primary { background: #2563eb; border-color: #2563eb; color: white; }
  button:hover { filter: brightness(1.2); }
</style>
```

- [ ] **Step 2: Commit**

```
git add src/lib/components/SettingsModal.svelte
git commit -m "feat: add SettingsModal for port, units, and tire thresholds"
```

---

## Task 17: SessionDrawer Component

**Files:**
- Create: `src/lib/components/SessionDrawer.svelte`

Slides in from the right. Shows list of sessions; clicking one loads its packets and renders a uPlot chart.

- [ ] **Step 1: Implement `SessionDrawer.svelte`**

```svelte
<script lang="ts">
  import { onMount, onDestroy } from 'svelte';
  import { sessions, loadSessions, loadSessionPackets, deleteSession } from '$lib/stores/sessions';
  import { carName } from '$lib/car-name';
  import type { TelemetryPacket, SessionRow } from '$lib/types';
  import uPlot from 'uplot';
  import 'uplot/dist/uPlot.min.css';

  let { onClose } = $props<{ onClose: () => void }>();

  let selectedSession = $state<SessionRow | null>(null);
  let chartEl = $state<HTMLDivElement | null>(null);
  let uplot: uPlot | null = null;

  onMount(async () => {
    await loadSessions();
  });

  onDestroy(() => {
    uplot?.destroy();
  });

  function formatTime(seconds: number) {
    if (!seconds || seconds <= 0) return '—';
    const m = Math.floor(seconds / 60);
    const s = (seconds % 60).toFixed(3).padStart(6, '0');
    return `${m}:${s}`;
  }

  function formatDate(ms: number) {
    return new Date(ms).toLocaleString();
  }

  async function selectSession(session: SessionRow) {
    selectedSession = session;
    uplot?.destroy();
    uplot = null;

    const packets: TelemetryPacket[] = await loadSessionPackets(session.id);
    if (packets.length === 0 || !chartEl) return;

    const times = packets.map((_, i) => i / 60);
    const speeds = packets.map(p => p.speedMs * 2.23694);
    const throttles = packets.map(p => (p.throttle / 255) * 100);
    const brakes = packets.map(p => (p.brake / 255) * 100);
    const rpms = packets.map(p =>
      p.engineMaxRpm > 0 ? (p.currentEngineRpm / p.engineMaxRpm) * 100 : 0
    );

    const opts: uPlot.Options = {
      width: chartEl.clientWidth,
      height: 200,
      series: [
        {},
        { label: 'Speed (mph)', stroke: '#3b82f6', width: 1.5 },
        { label: 'Throttle %', stroke: '#22c55e', width: 1 },
        { label: 'Brake %', stroke: '#ef4444', width: 1 },
        { label: 'RPM %', stroke: '#a855f7', width: 1 },
      ],
      axes: [
        { stroke: '#6b7280', grid: { stroke: '#1f2937' } },
        { stroke: '#6b7280', grid: { stroke: '#1f2937' } },
      ],
    };

    uplot = new uPlot(opts, [times, speeds, throttles, brakes, rpms], chartEl);
  }

  async function handleDelete(session: SessionRow, e: MouseEvent) {
    e.stopPropagation();
    if (!confirm(`Delete session from ${formatDate(session.startedAt)}?`)) return;
    await deleteSession(session.id);
    if (selectedSession?.id === session.id) {
      selectedSession = null;
      uplot?.destroy();
      uplot = null;
    }
  }
</script>

<div class="drawer">
  <div class="drawer-header">
    <h3>Sessions</h3>
    <button onclick={onClose}>✕</button>
  </div>

  <div class="drawer-body">
    <div class="session-list">
      {#each $sessions as session}
        <div
          class="session-row"
          class:selected={selectedSession?.id === session.id}
          role="button"
          tabindex="0"
          onclick={() => selectSession(session)}
          onkeydown={(e) => e.key === 'Enter' && selectSession(session)}
        >
          <div class="session-info">
            <span class="session-car">{carName(session.carOrdinal)}</span>
            <span class="session-date">{formatDate(session.startedAt)}</span>
            <span class="session-best">Best: {formatTime(session.bestLap ?? 0)}</span>
          </div>
          <button class="delete-btn" onclick={(e) => handleDelete(session, e)}>🗑</button>
        </div>
      {:else}
        <p class="empty">No sessions recorded yet.</p>
      {/each}
    </div>

    {#if selectedSession}
      <div class="chart-area" bind:this={chartEl}></div>
    {/if}
  </div>
</div>

<style>
  .drawer {
    position: fixed; right: 0; top: 0; bottom: 0; width: 420px;
    background: #0f172a; border-left: 1px solid #1e293b;
    display: flex; flex-direction: column; z-index: 50;
    box-shadow: -4px 0 24px rgba(0,0,0,0.5);
  }
  .drawer-header {
    display: flex; justify-content: space-between; align-items: center;
    padding: 1rem; border-bottom: 1px solid #1e293b;
  }
  h3 { margin: 0; color: #f9fafb; }
  .drawer-header button {
    background: none; border: none; color: #6b7280; font-size: 1.1rem; cursor: pointer;
  }
  .drawer-body { flex: 1; overflow-y: auto; display: flex; flex-direction: column; gap: 1rem; padding: 0.5rem; }
  .session-list { display: flex; flex-direction: column; gap: 0.3rem; }
  .session-row {
    display: flex; align-items: center; justify-content: space-between;
    padding: 0.6rem 0.75rem; border-radius: 6px; cursor: pointer;
    border: 1px solid transparent; background: #1e293b;
  }
  .session-row:hover, .session-row.selected { border-color: #3b82f6; }
  .session-info { display: flex; flex-direction: column; gap: 0.1rem; }
  .session-car { font-size: 0.85rem; font-weight: 600; color: #e5e7eb; }
  .session-date { font-size: 0.7rem; color: #6b7280; }
  .session-best { font-size: 0.75rem; color: #a855f7; font-weight: 700; }
  .delete-btn { background: none; border: none; cursor: pointer; font-size: 0.9rem; color: #6b7280; }
  .delete-btn:hover { color: #ef4444; }
  .empty { color: #4b5563; font-size: 0.85rem; text-align: center; padding: 2rem; }
  .chart-area { min-height: 220px; border-radius: 6px; overflow: hidden; background: #111827; }
  :global(.uplot) { background: transparent !important; }
</style>
```

- [ ] **Step 2: Commit**

```
git add src/lib/components/SessionDrawer.svelte
git commit -m "feat: add SessionDrawer with session list and uPlot replay chart"
```

---

## Task 18: Main Dashboard Layout

**Files:**
- Modify: `src/routes/+page.svelte`
- Create: `src/routes/+layout.ts`

Wire all components into the full-screen dashboard.

- [ ] **Step 1: Create `src/routes/+layout.ts`**

```typescript
export const prerender = true;
export const ssr = false;
```

- [ ] **Step 2: Replace `src/routes/+page.svelte`**

```svelte
<script lang="ts">
  import { onMount } from 'svelte';
  import { startTelemetryListener } from '$lib/stores/telemetry';
  import { loadSettings, settings } from '$lib/stores/sessions';
  import TopBar from '$lib/components/TopBar.svelte';
  import InputStrip from '$lib/components/InputStrip.svelte';
  import CenterPanel from '$lib/components/CenterPanel.svelte';
  import TireWidget from '$lib/components/TireWidget.svelte';
  import LapBar from '$lib/components/LapBar.svelte';
  import SessionDrawer from '$lib/components/SessionDrawer.svelte';
  import SettingsModal from '$lib/components/SettingsModal.svelte';

  let showSessions = $state(false);
  let showSettings = $state(false);

  onMount(async () => {
    await loadSettings();
    await startTelemetryListener();
  });

  const s = $derived($settings);
</script>

<div class="dashboard">
  <TopBar
    useMph={s?.useMph ?? true}
    onSettings={() => (showSettings = true)}
    onSessions={() => (showSessions = !showSessions)}
  />

  <div class="main">
    <div class="left-strip">
      <InputStrip />
    </div>

    <div class="center-area">
      <CenterPanel useMph={s?.useMph ?? true} />
    </div>

    <div class="right-strip">
      <TireWidget
        tireTempCold={s?.tireTempCold ?? 60}
        tireTempOptimal={s?.tireTempOptimal ?? 85}
        tireTempHot={s?.tireTempHot ?? 110}
      />
    </div>
  </div>

  <div class="lap-bar">
    <LapBar />
  </div>
</div>

{#if showSessions}
  <SessionDrawer onClose={() => (showSessions = false)} />
{/if}

{#if showSettings}
  <SettingsModal onClose={() => (showSettings = false)} />
{/if}

<style>
  :global(*, *::before, *::after) { box-sizing: border-box; margin: 0; padding: 0; }
  :global(body) {
    background: #030712;
    color: #f9fafb;
    font-family: 'Segoe UI', system-ui, -apple-system, sans-serif;
    overflow: hidden;
    height: 100vh;
    width: 100vw;
  }

  .dashboard {
    display: flex;
    flex-direction: column;
    height: 100vh;
    width: 100vw;
  }

  .main {
    flex: 1;
    display: grid;
    grid-template-columns: 80px 1fr 160px;
    min-height: 0;
  }

  .left-strip {
    border-right: 1px solid #1f2937;
    background: #030712;
  }

  .center-area {
    background: #030712;
  }

  .right-strip {
    border-left: 1px solid #1f2937;
    background: #030712;
  }

  .lap-bar {
    height: 3.5rem;
    flex-shrink: 0;
  }
</style>
```

- [ ] **Step 3: Run the full app and verify the dashboard renders**

```
npm run tauri dev
```

Expected: Full-screen dark dashboard. TopBar shows "WAITING…" dot. All panels visible. No console errors.

- [ ] **Step 4: Send a test UDP packet and verify the dashboard updates**

In PowerShell:
```powershell
$udp = New-Object System.Net.Sockets.UdpClient
$buf = [byte[]]::new(311)
# IsRaceOn = 1
$buf[0] = 1
# Speed = 55.0 m/s (~123 mph) at offset 244
[Array]::Copy([BitConverter]::GetBytes([float]55.0), 0, $buf, 244, 4)
# RPM = 5000 at offset 16, MaxRPM = 8000 at offset 8
[Array]::Copy([BitConverter]::GetBytes([float]8000.0), 0, $buf, 8, 4)
[Array]::Copy([BitConverter]::GetBytes([float]5000.0), 0, $buf, 16, 4)
# Throttle = 200 at offset 303
$buf[303] = 200
# Gear = 4 at offset 307
$buf[307] = 4
# TireTemp FL = 88°C at offset 256
[Array]::Copy([BitConverter]::GetBytes([float]88.0), 0, $buf, 256, 4)
$udp.Send($buf, $buf.Length, "127.0.0.1", 20440)
$udp.Close()
```

Expected:
- TopBar shows green dot "LIVE"
- Speed shows ~123 mph
- Gear box shows "4"
- RPM bar fills ~62%
- Throttle bar fills ~78%
- FL tire shows 88° in green/orange

- [ ] **Step 5: Commit**

```
git add src/routes/+page.svelte src/routes/+layout.ts
git commit -m "feat: wire full-screen dashboard layout with all components"
```

---

## Task 19: In-Game Setup Instructions (README)

**Files:**
- Create: `README.md`

- [ ] **Step 1: Create `README.md`**

```markdown
# FH6 Telemetry Dashboard

Real-time telemetry dashboard for Forza Horizon 6. Displays speed, RPM, tire temps,
inputs, and lap times. Records sessions to SQLite for later review.

## Install

Download the latest `.exe` installer from Releases and run it. No additional software required.

## Forza Horizon 6 Setup

1. In FH6, go to **Settings → HUD and Gameplay**
2. Scroll to the **DATA OUT** section
3. Set **Data Out** to **On**
4. Set **Data Out IP Address** to `127.0.0.1`
5. Set **Data Out IP Port** to `20440` (or your custom port from Settings)
6. Set **Data Out Package Format** to **Car Dash**

The dashboard will show a green dot in the top-left when packets are received.

## Building from Source

Prerequisites: Rust toolchain, Node.js 18+, Windows WebView2 (pre-installed on Win 10/11).

```
npm install
npm run tauri build
```

Installer output: `src-tauri/target/release/bundle/nsis/FH6 Telemetry_0.1.0_x64-setup.exe`
```

- [ ] **Step 2: Commit**

```
git add README.md
git commit -m "docs: add setup instructions for FH6 data out and build instructions"
```

---

## Task 20: Production Build and Installer Verification

- [ ] **Step 1: Run the release build**

```
npm run tauri build 2>&1
```

Expected: completes without errors. Output: `src-tauri/target/release/bundle/nsis/FH6 Telemetry_0.1.0_x64-setup.exe`

- [ ] **Step 2: Install and smoke-test**

Run the NSIS installer. Expected: installs to `%LOCALAPPDATA%\Programs\FH6 Telemetry\`. Launch the app. Verify:
- Window opens without admin rights
- Starts listening on port 20440 (check Windows Firewall prompt if first launch)
- Send the test UDP packet from Task 18 Step 4 and verify dashboard responds

- [ ] **Step 3: Verify session DB is created in the right location**

```powershell
Test-Path "$env:LOCALAPPDATA\fh6-tel\sessions.db"
```

Expected: `True`

- [ ] **Step 4: Final commit**

```
git add .
git commit -m "chore: verify release build and installer"
```

---

## Self-Review Checklist

| Spec requirement | Covered by |
|---|---|
| Rust UDP listener | Task 6 |
| FH6 Dash packet parsing (324-byte) | Task 2 |
| Always-live telemetry regardless of session state | Task 6 (udp.rs always emits) |
| Session recording on IsRaceOn transition | Tasks 4+5 |
| SQLite raw blob storage with session metadata | Task 4 |
| Full-screen layout | Task 18 |
| Speed + gear center panel | Task 13 |
| Input bars (throttle/brake/clutch/handbrake) | Task 12 |
| Tire widget (4 corners, temp + slip) | Task 14 |
| Lap times bar | Task 15 |
| Top bar with connection status + car info | Task 11 |
| Settings (port, units, thresholds) | Tasks 3+16 |
| Session drawer with chart replay | Task 17 |
| Car name lookup | Task 10 |
| NSIS single-file installer | Task 18 + Task 20 |
| Error handling (bad packet length, DB failure) | Task 2 (parse returns Err), Task 6 (silently continues) |
| No signal state (WAITING… indicator) | Task 11 (TopBar) |
| Data stored in %LOCALAPPDATA% | Tasks 3+4 |
