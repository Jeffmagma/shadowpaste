use chrono::{DateTime, Local};
use rusqlite::{Connection, params};
use crate::monitor::ClipboardContent;

#[derive(Debug, Clone, PartialEq)]
pub struct ClipboardEntry {
    pub id: i64,
    pub content: ClipboardContent,
    pub copied_at: DateTime<Local>,
}

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open() -> rusqlite::Result<Self> {
        let db_path = dirs::data_local_dir()
            .unwrap_or_else(|| std::path::PathBuf::from("."))
            .join("shadowpaste")
            .join("shadowpaste.db");

        // make sure the directory exists
        if let Some(parent) = db_path.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        let conn = Connection::open(&db_path)?;

        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS clipboard_history (
                id           INTEGER PRIMARY KEY AUTOINCREMENT,
                content_type TEXT    NOT NULL,
                content      TEXT    NOT NULL,
                copied_at    TEXT    NOT NULL
            );"
        )?;

        Ok(Self { conn })
    }

    pub fn insert(&self, entry: &ClipboardEntry) -> rusqlite::Result<i64> {
        let (content_type, content) = match &entry.content {
            ClipboardContent::Text(t) => ("text", t.clone()),
            ClipboardContent::Image(s) => ("image", s.clone()),
            ClipboardContent::Empty => ("empty", String::new()),
        };

        self.conn.execute(
            "INSERT INTO clipboard_history (content_type, content, copied_at) VALUES (?1, ?2, ?3)",
            params![content_type, content, entry.copied_at.to_rfc3339()],
        )?;

        Ok(self.conn.last_insert_rowid())
    }

    pub fn delete_by_id(&self, id: i64) -> rusqlite::Result<()> {
        self.conn.execute(
            "DELETE FROM clipboard_history WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    pub fn load_all(&self) -> rusqlite::Result<Vec<ClipboardEntry>> {
        let mut statement = self.conn.prepare(
            "SELECT id, content_type, content, copied_at FROM clipboard_history ORDER BY copied_at ASC"
        )?;

        let entries = statement.query_map([], |row| {
            let id: i64 = row.get(0)?;
            let content_type: String = row.get(1)?;
            let content_str: String = row.get(2)?;
            let copied_at_str: String = row.get(3)?;

            let content = match content_type.as_str() {
                "text" => ClipboardContent::Text(content_str),
                "image" => ClipboardContent::Image(content_str),
                _ => ClipboardContent::Empty,
            };

            let copied_at = DateTime::parse_from_rfc3339(&copied_at_str)
                .map(|dt| dt.with_timezone(&Local))
                .unwrap_or_else(|_| Local::now());

            Ok(ClipboardEntry { id, content, copied_at })
        })?.collect::<Result<Vec<_>, _>>()?;

        Ok(entries)
    }
}
