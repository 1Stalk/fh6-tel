# FH6 Telemetry Dashboard

Real-time telemetry dashboard for Forza Horizon 6. Displays speed, RPM, tire temps, inputs, and lap times. Records sessions to SQLite for later review.

## Install

Download the latest `.exe` installer from Releases and run it. No additional software required — WebView2 is pre-installed on Windows 10/11.

## Forza Horizon 6 Setup

1. In FH6, go to **Settings → HUD and Gameplay**
2. Scroll to the **DATA OUT** section
3. Set **Data Out** to **On**
4. Set **Data Out IP Address** to `127.0.0.1`
5. Set **Data Out IP Port** to `20440` (or your custom port from the app's Settings)
6. Set **Data Out Package Format** to **Car Dash**

The dashboard shows a green dot in the top-left when packets are received.

## Dashboard Layout

- **Top bar** — Connection status, car name, class, PI, drivetrain. Settings (⚙) and Sessions (⏱) buttons.
- **Left strip** — Throttle / Brake / Clutch / Handbrake input bars
- **Centre** — Speed (large), gear box, RPM bar, boost gauge
- **Right strip** — Tire widget: 4 corners showing temperature (colour-coded) and slip
- **Bottom bar** — Lap number, current / last / best / session lap times

## Session Recording

Sessions are recorded automatically when the game signals the car is active. Click ⏱ to view past sessions and replay speed, throttle, brake, and RPM as a chart.

Data is stored in `%LOCALAPPDATA%\fh6-tel\sessions.db`.

## Building from Source

Prerequisites: Rust 1.75+, Node.js 18+, Windows 10/11 with WebView2 (pre-installed).

```bash
npm install
npm run tauri build
```

Installer output: `src-tauri/target/release/bundle/nsis/FH6 Telemetry_0.1.0_x64-setup.exe`
