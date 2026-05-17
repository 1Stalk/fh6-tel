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
    car_class: i32,
    car_pi: i32,
) -> Result<i64> {
    conn.execute(
        "INSERT INTO sessions (started_at, car_ordinal, car_class, car_pi) VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![started_at, car_ordinal, car_class, car_pi],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Sets car metadata on a session that still has car_ordinal=0.
/// Called every packet so the first non-zero ordinal wins, handling the case
/// where the opening packet arrives before the game has populated car data.
pub fn update_session_car_if_unknown(
    conn: &Connection,
    id: i64,
    car_ordinal: i32,
    car_class: i32,
    car_pi: i32,
) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET car_ordinal=?1, car_class=?2, car_pi=?3 WHERE id=?4 AND car_ordinal=0",
        rusqlite::params![car_ordinal, car_class, car_pi, id],
    )?;
    Ok(())
}

pub fn reopen_session(conn: &Connection, id: i64) -> Result<()> {
    conn.execute(
        "UPDATE sessions SET ended_at = NULL WHERE id=?1",
        [id],
    )?;
    Ok(())
}

pub fn close_session(conn: &Connection, id: i64, ended_at: i64, best_lap: f32) -> Result<()> {
    // Use MIN so a worse post-rewind lap never overwrites a better pre-rewind one.
    conn.execute(
        "UPDATE sessions SET ended_at=?1,
         best_lap = CASE WHEN ?2 > 0.0 AND (best_lap IS NULL OR ?2 < best_lap) THEN ?2 ELSE best_lap END
         WHERE id=?3",
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

pub fn get_distinct_car_ordinals(conn: &Connection) -> Result<Vec<i32>> {
    let mut stmt = conn.prepare(
        "SELECT DISTINCT car_ordinal FROM sessions WHERE car_ordinal != 0 ORDER BY car_ordinal",
    )?;
    let rows = stmt.query_map([], |r| r.get::<_, i32>(0))?;
    rows.collect()
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
        let id = open_session(&conn, 12345, 3, 5, 900).unwrap();
        assert!(id > 0);
        close_session(&conn, id, 1000, 78.5).unwrap();
        let ended: Option<i64> = conn
            .query_row("SELECT ended_at FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert!(ended.is_some());
    }

    #[test]
    fn reopen_clears_ended_at() {
        let conn = in_memory();
        let id = open_session(&conn, 12345, 3, 5, 900).unwrap();
        close_session(&conn, id, 1000, 78.5).unwrap();
        reopen_session(&conn, id).unwrap();
        let ended: Option<i64> = conn
            .query_row("SELECT ended_at FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert!(ended.is_none());
    }

    #[test]
    fn close_preserves_better_best_lap() {
        let conn = in_memory();
        let id = open_session(&conn, 0, 0, 0, 0).unwrap();
        close_session(&conn, id, 100, 65.5).unwrap();
        reopen_session(&conn, id).unwrap();
        // Worse lap after rewind — existing best must be kept
        close_session(&conn, id, 200, 70.0).unwrap();
        let best: Option<f32> = conn
            .query_row("SELECT best_lap FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(best, Some(65.5));
    }

    #[test]
    fn close_updates_to_better_best_lap() {
        let conn = in_memory();
        let id = open_session(&conn, 0, 0, 0, 0).unwrap();
        close_session(&conn, id, 100, 65.5).unwrap();
        reopen_session(&conn, id).unwrap();
        // Better lap after rewind — should update
        close_session(&conn, id, 200, 60.0).unwrap();
        let best: Option<f32> = conn
            .query_row("SELECT best_lap FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(best, Some(60.0));
    }

    #[test]
    fn close_with_no_lap_keeps_existing_best() {
        let conn = in_memory();
        let id = open_session(&conn, 0, 0, 0, 0).unwrap();
        close_session(&conn, id, 100, 65.5).unwrap();
        reopen_session(&conn, id).unwrap();
        // -1.0 means no lap was recorded post-rewind
        close_session(&conn, id, 200, -1.0).unwrap();
        let best: Option<f32> = conn
            .query_row("SELECT best_lap FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(best, Some(65.5));
    }

    #[test]
    fn insert_and_count_packets() {
        let conn = in_memory();
        let id = open_session(&conn, 0, 0, 0, 0).unwrap();
        let blob = vec![0u8; 311];
        insert_packet(&conn, id, 1000, &blob).unwrap();
        insert_packet(&conn, id, 2000, &blob).unwrap();
        let count: i64 = conn
            .query_row("SELECT packet_count FROM sessions WHERE id=?1", [id], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 2);
    }
}
