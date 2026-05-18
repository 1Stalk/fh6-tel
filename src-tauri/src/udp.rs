use std::time::{SystemTime, UNIX_EPOCH};
use tauri::{AppHandle, Emitter, Manager};
use tokio::net::UdpSocket;

use crate::{
    db,
    parser,
    session::SessionAction,
    AppState,
};

pub async fn run(app: AppHandle, port: u16) {
    let addr = format!("0.0.0.0:{port}");
    let socket = match UdpSocket::bind(&addr).await {
        Ok(s) => s,
        Err(e) => {
            eprintln!("[udp] failed to bind {addr}: {e}");
            let _ = app.emit("udp_bind_failed", format!("Cannot bind port {port}: {e}"));
            return;
        }
    };
    println!("[udp] listening on {addr}");

    let mut buf = vec![0u8; 1024];
    let mut prev_in_event = false;
    let mut debug_logged = false;
    // Grace period before closing session — prevents pause-menu from splitting a run.
    // At ~30 packets/s, 150 ≈ 5 seconds of tolerance.
    let mut close_pending: u32 = 0;
    const CLOSE_GRACE: u32 = 150;

    loop {
        let (len, _) = match socket.recv_from(&mut buf).await {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[udp] recv error: {e}");
                continue;
            }
        };

        let raw = &buf[..len];

        if !debug_logged {
            debug_logged = true;
            println!("[udp] first packet: {len} bytes");
            if raw.len() >= 323 {
                let speed = f32::from_le_bytes(raw[256..260].try_into().unwrap_or([0; 4]));
                let thr = raw[315];
                let brk = raw[316];
                let gear = raw[319];
                let pos = raw[314];
                let tire_f_raw = f32::from_le_bytes(raw[268..272].try_into().unwrap_or([0; 4]));
                println!("[udp] speed={speed:.2}m/s thr={thr} brk={brk} gear={gear} race_pos={pos} tire_fl_raw={tire_f_raw:.1}°F");
            }
        }

        let pkt = match parser::parse(raw) {
            Ok(p) => p,
            Err(_) => continue,
        };

        // Always emit live data regardless of session state
        let _ = app.emit("telemetry_tick", &pkt);

        // Record whenever a lap is being timed: races/Rivals (race_position > 0)
        // and Time Trial (race_position 0 but the lap clock runs). Free-roam has
        // no lap timer so it stays unrecorded. Grace period stops pause-menu
        // packets from splitting a session.
        let timed_lap = pkt.current_lap > 0.0 || pkt.last_lap > 0.0 || pkt.lap_number > 0;
        let raw_in_event = pkt.is_race_on && (pkt.race_position > 0 || timed_lap);
        if raw_in_event {
            close_pending = 0;
        } else {
            close_pending = close_pending.saturating_add(1);
        }
        let in_event = raw_in_event || close_pending < CLOSE_GRACE;

        let state = app.state::<AppState>();
        handle_session(&app, &state, &pkt, raw, prev_in_event, in_event);
        prev_in_event = in_event;
    }
}

fn handle_session(app: &AppHandle, state: &AppState, pkt: &parser::TelemetryPacket, raw: &[u8], prev_in_event: bool, in_event: bool) {
    let mut sm = state.session_manager.lock().unwrap();
    let db = state.db.lock().unwrap();

    let now_ms: u64 = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis() as u64;

    // Progress timer for rewind detection. Races/Rivals report a cumulative
    // current_race_time; Time Trial leaves it at 0 and only the lap clock
    // advances — fall back to that so rewinds still stitch the session.
    let progress = if pkt.current_race_time > 0.0 {
        pkt.current_race_time
    } else {
        pkt.current_lap
    };

    // Apply event transition before inserting so the opening packet is captured
    let action = sm.on_race_on_change(prev_in_event, in_event, pkt.car_ordinal, pkt.car_class, pkt.car_pi);

    match action {
        SessionAction::Open { car_ordinal, car_class, car_pi } => {
            // Check if the new stream looks like a rewind into the previous session:
            // race time went backward within the rewind window.
            if let Some(reopen_id) = sm.check_reopen(progress, now_ms) {
                match db::reopen_session(&db, reopen_id) {
                    Ok(()) => {
                        sm.set_active_id(Some(reopen_id));
                        println!("[session] rewind detected, continuing #{reopen_id}");
                    }
                    Err(e) => eprintln!("[session] reopen error: {e}"),
                }
            } else {
                match db::open_session(&db, now_ms as i64, car_ordinal, car_class, car_pi) {
                    Ok(id) => {
                        sm.set_active_id(Some(id));
                        println!("[session] opened #{id}");
                    }
                    Err(e) => {
                        eprintln!("[session] open error: {e}");
                        let _ = app.emit("session_error", format!("Failed to open session: {e}"));
                    }
                }
            }
        }
        SessionAction::Close => {
            if let Some(id) = sm.active_session_id() {
                sm.note_close(now_ms);
                // The lap in progress at session end never crossed the line —
                // record it and let it count toward the best lap.
                if let Some(lap) = sm.take_final_lap() {
                    if let Err(e) = db::insert_lap(&db, id, lap.lap_number, lap.lap_time) {
                        eprintln!("[session] final lap insert error: {e}");
                    }
                }
                let best = sm.best_for_close();
                if let Err(e) = db::close_session(&db, id, now_ms as i64, best) {
                    eprintln!("[session] close error: {e}");
                    let _ = app.emit("session_error", format!("Failed to close session: {e}"));
                } else {
                    println!("[session] closed #{id}");
                }
            }
            sm.set_active_id(None);
        }
        SessionAction::None => {}
    }

    if let Some(session_id) = sm.active_session_id() {
        sm.update_best_lap(pkt.best_lap);
        sm.update_race_time(progress);
        // Persist each lap as the lap counter advances.
        if let Some(lap) = sm.note_tick(pkt.lap_number, pkt.current_lap, pkt.last_lap) {
            if let Err(e) = db::insert_lap(&db, session_id, lap.lap_number, lap.lap_time) {
                eprintln!("[session] lap insert error: {e}");
            }
        }
        if let Err(e) = db::insert_packet(&db, session_id, pkt.timestamp_ms, raw) {
            eprintln!("[session] insert error: {e}");
            let _ = app.emit("session_error", format!("Failed to write telemetry: {e}"));
        }
        // Lazily fill car metadata: the opening packet sometimes arrives before the
        // game has populated car_ordinal. This no-ops once car_ordinal is non-zero.
        if pkt.car_ordinal != 0 {
            db::update_session_car_if_unknown(&db, session_id, pkt.car_ordinal, pkt.car_class, pkt.car_pi).ok();
        }
    }
}
