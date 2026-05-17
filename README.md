# FH6 Telemetry Dashboard

[![Latest Release](https://img.shields.io/github/v/release/TheBanHammer/fh6-tel?label=download)](https://github.com/TheBanHammer/fh6-tel/releases/latest)

Real-time telemetry dashboard for Forza Horizon 6. Displays speed, RPM, heading, attitude, tire temps, driver inputs, and lap times. Records sessions to SQLite for later review.

## Install

Download the latest `.exe` installer from [Releases](https://github.com/TheBanHammer/fh6-tel/releases/latest) and run it. No additional software required — WebView2 is pre-installed on Windows 10/11.

## Forza Horizon 6 Setup

1. In FH6, go to **Settings → HUD and Gameplay**
2. Scroll to the **DATA OUT** section
3. Set **Data Out** to **On**
4. Set **Data Out IP Address** to `127.0.0.1`
5. Set **Data Out IP Port** to `20440` (or your custom port from the app's Settings)
6. Set **Data Out Package Format** to **Car Dash**

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

Click **⚙** to open Settings (port, units, theme, tire thresholds).  
Click **⏱** to open the Session Drawer (past sessions + uPlot chart).

## Themes

Three built-in themes selectable from Settings:

| Theme | Accent |
|-------|--------|
| Dark (default) | Blue `#3b82f6` |
| Cobalt2 | Yellow `#ffc600` |
| Purple | Purple `#c084fc` |

## Session Recording

Sessions are recorded automatically when the game signals the car is active in a race. Click **⏱** to view past sessions and replay speed, throttle, brake, and RPM as a chart. Sessions survive in-race rewinds — the best pre-rewind lap time is always preserved.

Data is stored in `%LOCALAPPDATA%\fh6-tel\sessions.db`.

## Building from Source

Prerequisites: Rust 1.75+, Node.js 18+, Windows 10/11 with WebView2 (pre-installed).

```bash
npm install
npm run tauri build
```

Installer output: `src-tauri/target/release/bundle/nsis/FH6 Telemetry_0.1.0_x64-setup.exe`

## Releasing

Releases are created via the **Release** GitHub Actions workflow:

1. Go to **Actions → Release → Run workflow**
2. Enter the version number (e.g. `1.0.0`)
3. The workflow bumps versions in `package.json` and `tauri.conf.json`, commits, tags `v1.0.0`, builds the NSIS and MSI installers, and publishes a draft GitHub release
4. Review and publish the draft from the [Releases](https://github.com/TheBanHammer/fh6-tel/releases) page
