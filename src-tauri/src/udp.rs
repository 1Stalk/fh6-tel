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

fn handle_session(state: &AppState, pkt: &parser::TelemetryPacket, raw: &[u8], prev_race_on: bool) {
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
