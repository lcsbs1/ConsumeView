//! SQLite metrics store (WAL).

use crate::domain::RequestRow;
use anyhow::{Context, Result};
use chrono::{Datelike, Local, Utc};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

pub struct MetricsDb {
    conn: Mutex<Connection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredRequest {
    pub id: String,
    pub session_id: String,
    pub provider: String,
    pub model: String,
    pub prompt_tokens: i64,
    pub completion_tokens: i64,
    pub cached_tokens: i64,
    pub total_tokens: i64,
    pub cost_usd: f64,
    pub duration_ms: i64,
    pub ttft_ms: i64,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionInfo {
    pub id: String,
    pub name: String,
    pub created_at: String,
}

impl MetricsDb {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path).context("open metrics.db")?;
        conn.execute_batch(
            "PRAGMA journal_mode=WAL;
             PRAGMA synchronous=NORMAL;",
        )?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS requests (
                id TEXT PRIMARY KEY,
                session_id TEXT DEFAULT 'cursor-workspace',
                provider TEXT NOT NULL,
                model TEXT NOT NULL,
                prompt_tokens INTEGER DEFAULT 0,
                completion_tokens INTEGER DEFAULT 0,
                cached_tokens INTEGER DEFAULT 0,
                total_tokens INTEGER DEFAULT 0,
                cost_usd REAL DEFAULT 0.0,
                duration_ms INTEGER DEFAULT 0,
                ttft_ms INTEGER DEFAULT 0,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );
            CREATE INDEX IF NOT EXISTS idx_requests_session ON requests(session_id);
            CREATE INDEX IF NOT EXISTS idx_requests_created_at ON requests(created_at);

            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at DATETIME DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS app_meta (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );
            "#,
        )?;

        let count: i64 = conn.query_row("SELECT COUNT(*) FROM sessions", [], |r| r.get(0))?;
        if count == 0 {
            conn.execute(
                "INSERT INTO sessions (id, name) VALUES (?1, ?2)",
                params!["cursor-workspace", "Cursor workspace"],
            )?;
            conn.execute(
                "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('active_session', ?1)",
                params!["cursor-workspace"],
            )?;
        }
        Ok(())
    }

    pub fn insert_request(&self, req: &StoredRequest) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            r#"INSERT INTO requests (
                id, session_id, provider, model,
                prompt_tokens, completion_tokens, cached_tokens, total_tokens,
                cost_usd, duration_ms, ttft_ms, created_at
            ) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)"#,
            params![
                req.id,
                req.session_id,
                req.provider,
                req.model,
                req.prompt_tokens,
                req.completion_tokens,
                req.cached_tokens,
                req.total_tokens,
                req.cost_usd,
                req.duration_ms,
                req.ttft_ms,
                req.created_at,
            ],
        )?;
        Ok(())
    }

    pub fn list_sessions(&self) -> Result<Vec<SessionInfo>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt =
            conn.prepare("SELECT id, name, created_at FROM sessions ORDER BY created_at ASC")?;
        let rows = stmt
            .query_map([], |r| {
                Ok(SessionInfo {
                    id: r.get(0)?,
                    name: r.get(1)?,
                    created_at: r.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn create_session(&self, id: &str, name: &str) -> Result<SessionInfo> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT INTO sessions (id, name) VALUES (?1, ?2)",
            params![id, name],
        )?;
        Ok(SessionInfo {
            id: id.to_string(),
            name: name.to_string(),
            created_at: Utc::now().to_rfc3339(),
        })
    }

    pub fn get_active_session(&self) -> Result<String> {
        let conn = self.conn.lock().unwrap();
        let val: Result<String, _> = conn.query_row(
            "SELECT value FROM app_meta WHERE key = 'active_session'",
            [],
            |r| r.get(0),
        );
        Ok(val.unwrap_or_else(|_| "cursor-workspace".into()))
    }

    pub fn set_active_session(&self, id: &str) -> Result<()> {
        let conn = self.conn.lock().unwrap();
        conn.execute(
            "INSERT OR REPLACE INTO app_meta (key, value) VALUES ('active_session', ?1)",
            params![id],
        )?;
        Ok(())
    }

    pub fn list_session_requests(&self, session_id: &str, limit: i64) -> Result<Vec<StoredRequest>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT id, session_id, provider, model,
                      prompt_tokens, completion_tokens, cached_tokens, total_tokens,
                      cost_usd, duration_ms, ttft_ms, created_at
               FROM requests WHERE session_id = ?1
               ORDER BY created_at DESC LIMIT ?2"#,
        )?;
        let rows = stmt
            .query_map(params![session_id, limit], map_stored)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn session_request_rows(&self, session_id: &str) -> Result<Vec<RequestRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT provider, prompt_tokens, completion_tokens, cached_tokens,
                      total_tokens, cost_usd, duration_ms, ttft_ms
               FROM requests WHERE session_id = ?1"#,
        )?;
        let rows = stmt
            .query_map(params![session_id], map_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn period_request_rows(&self, since_utc: &str) -> Result<Vec<RequestRow>> {
        let conn = self.conn.lock().unwrap();
        let mut stmt = conn.prepare(
            r#"SELECT provider, prompt_tokens, completion_tokens, cached_tokens,
                      total_tokens, cost_usd, duration_ms, ttft_ms
               FROM requests WHERE created_at >= ?1"#,
        )?;
        let rows = stmt
            .query_map(params![since_utc], map_row)?
            .collect::<Result<Vec<_>, _>>()?;
        Ok(rows)
    }

    pub fn month_to_date_cost(&self) -> Result<f64> {
        let now = Local::now();
        let start = format!("{:04}-{:02}-01 00:00:00", now.year(), now.month());
        let conn = self.conn.lock().unwrap();
        let cost: f64 = conn
            .query_row(
                "SELECT COALESCE(SUM(cost_usd), 0) FROM requests WHERE created_at >= ?1",
                params![start],
                |r| r.get(0),
            )
            .unwrap_or(0.0);
        Ok(cost)
    }
}

fn map_stored(r: &rusqlite::Row<'_>) -> rusqlite::Result<StoredRequest> {
    Ok(StoredRequest {
        id: r.get(0)?,
        session_id: r.get(1)?,
        provider: r.get(2)?,
        model: r.get(3)?,
        prompt_tokens: r.get(4)?,
        completion_tokens: r.get(5)?,
        cached_tokens: r.get(6)?,
        total_tokens: r.get(7)?,
        cost_usd: r.get(8)?,
        duration_ms: r.get(9)?,
        ttft_ms: r.get(10)?,
        created_at: r.get(11)?,
    })
}

fn map_row(r: &rusqlite::Row<'_>) -> rusqlite::Result<RequestRow> {
    Ok(RequestRow {
        provider: r.get(0)?,
        prompt_tokens: r.get(1)?,
        completion_tokens: r.get(2)?,
        cached_tokens: r.get(3)?,
        total_tokens: r.get(4)?,
        cost_usd: r.get(5)?,
        duration_ms: r.get(6)?,
        ttft_ms: r.get(7)?,
    })
}

pub fn default_db_path() -> PathBuf {
    let base = dirs::data_dir().unwrap_or_else(|| PathBuf::from("."));
    base.join("ConsumeView").join("metrics.db")
}
