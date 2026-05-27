# FH6 Telemetry Dashboard

[![Latest Release](https://img.shields.io/github/v/release/TheBanHammer/fh6-tel?label=version&color=blue)](https://github.com/TheBanHammer/fh6-tel/releases/latest)
[![Download](https://img.shields.io/github/downloads/TheBanHammer/fh6-tel/total)](https://github.com/TheBanHammer/fh6-tel/releases/latest)
[![Build](https://img.shields.io/github/actions/workflow/status/TheBanHammer/fh6-tel/release.yml?label=build)](https://github.com/TheBanHammer/fh6-tel/actions/workflows/release.yml)
[![License](https://img.shields.io/badge/license-MIT-green)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-Windows-0078d4?logo=windows)](https://github.com/TheBanHammer/fh6-tel/releases/latest)

Real-time telemetry dashboard for Forza Horizon 6. Displays speed, RPM, heading, attitude, tire temps, driver inputs, and lap times. Auto-records timed sessions to SQLite with a full replay/analysis viewer and a calibrated track map.

### Realtime Dashboard
<img width="1919" height="1054" alt="image" src="https://github.com/user-attachments/assets/2e4e2367-c6cc-43be-a97a-4565d0f057c0" />

### Auto Recorded Session Management (timed events)
<img width="333" height="295" alt="image" src="https://github.com/user-attachments/assets/f904c7e5-028d-40c4-bac0-cf7f06766cc8" /><img width="320" height="285" alt="image" src="https://github.com/user-attachments/assets/0de3029c-9cdc-430d-86dd-46bd4c1f91cb" />

### Session Replay
<img width="1919" height="1054" alt="image" src="https://github.com/user-attachments/assets/830d8d19-2af2-4dee-bdd1-4a3507a417e0" />



## Install

Download the latest `.exe` installer from [Releases](https://github.com/TheBanHammer/fh6-tel/releases/latest) and run it. No additional software required — WebView2 is pre-installed on Windows 10/11.

## Forza Horizon 6 Setup

1. In FH6, go to **Settings → HUD and Gameplay**
2. Scroll to the **DATA OUT** section
3. Set **Data Out** to **On**
4. Set **Data Out IP Address** to `127.0.0.1`
5. Set **Data Out IP Port** to `20440` (or your custom port from the app's Settings)

The dashboard shows a green dot in the top-left when packets are received.

## Dashboard Layout

```
┌─────────────────────────────────────────────────────────┐
│  TopBar — car name · class · PI · drivetrain · position  │
├─────────────────────────────────────────────────────────┤
│         CompassBar — scrolling heading tape              │
├───────────────────────────────────────┬─────────────────┤
│                                       │                 │
│  CenterPanel                          │  TireWidget     │
│  · RPM arc gauge                      │  · FL  FR       │
│  · Speed (large) + gear               │  · RL  RR       │
│  · G-meter                            │                 │
│  · Throttle / Brake / Clutch bars     │  Each corner:   │
│  · Handbrake + boost LEDs             │  temp colour    │
│  · Attitude indicator (ADI)           │  slip dot       │
│  · Steering wheel indicator           │  wear %         │
│                                       │  suspension     │
├───────────────────────────────────────┴─────────────────┤
│    LapBar — lap · current · last · best · session        │
└─────────────────────────────────────────────────────────┘
```

Click **⚙** to open Settings (port, units, theme, tire thresholds, track map).  
Click **⏱** to open the Session Drawer; click a session to open the replay/analysis viewer.

## Themes

Three built-in themes selectable from Settings:

| Theme | Accent |
|-------|--------|
| Dark (default) | Blue `#3b82f6` |
| Cobalt2 | Yellow `#ffc600` |
| Purple | Purple `#c084fc` |

## Session Recording

Sessions are recorded automatically whenever a lap is being timed — races, Rivals, and **Time Trial** (which reports no race position, so recording keys off the lap clock). Free-roam is not recorded.

Each session captures every telemetry packet plus per-lap times. The lap in progress when a session ends is finalised and counts toward the best lap, so a Time Trial hotlap is never lost. Sessions survive in-race **rewinds**: a rewind reopens and stitches the existing session (in races *and* Time Trial) rather than starting a new one, and the best pre-rewind lap time is always preserved.

From the Session Drawer you can bookmark, rename, delete, or **Clear all** sessions. Opening a session shows:

- **Analysis** — grouped charts (driver inputs, speed/RPM, G-forces, tire temps) with vertical lap-boundary markers and per-lap times
- **Map** — the driven racing line on the track map
- **Replay** — play the session back on the live dashboard via a docked timeline (scrub, play/pause, 0.5×–4× speed)

Data is stored in `%LOCALAPPDATA%\fh6-tel\sessions.db`.

## Track Map

A toggleable track map (right strip on the dashboard, plus the viewer's Map tab) draws the driven line over Forza Horizon 6: Japan map tiles, with a heading arrow for the car. It follows the car at a sensible zoom while free-roaming, and frames the whole track (static, pan/zoom-bounded) during a recording or replay. Tiles can be zoomed past their native resolution.

Calibrate world coordinates to the map via **Settings → Track Map → Calibrate map…**: drive to two landmarks, capture each, and click them on the map. View, zoom, and a custom tile source can be overridden in Settings.

## Building from Source

Prerequisites: Rust 1.75+, Node.js 18+, Windows 10/11 with WebView2 (pre-installed).

```bash
npm install
npm run tauri build
```

Installer output: `src-tauri/target/release/bundle/nsis/FH6 Telemetry_0.1.0_x64-setup.exe`

## Running from Source

Prerequisites: same as Building from Source.

```bash
npm install
npm run tauri dev
```

Vite hot-reloads the Svelte frontend on save; Rust changes trigger a rebuild and relaunch of the app window.

## Releasing

Releases are created via the **Release** GitHub Actions workflow:

1. Go to **Actions → Release → Run workflow**
2. Enter the version number (e.g. `1.0.0`)
3. The workflow bumps versions in `package.json` and `tauri.conf.json`, commits, tags `v1.0.0`, builds the NSIS and MSI installers, and publishes a draft GitHub release
4. Review and publish the draft from the [Releases](https://github.com/TheBanHammer/fh6-tel/releases) page
