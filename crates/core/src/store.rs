use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::Value;
use std::path::Path;

pub struct Store(Connection);

impl Store {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .mode(0o600)
                .open(path)?;
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON;
            CREATE TABLE IF NOT EXISTS state (id INTEGER PRIMARY KEY CHECK(id=1), value TEXT NOT NULL);")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))?;
        }
        Ok(Self(conn))
    }

    pub fn read(&self) -> Result<Option<Value>> {
        let mut query = self.0.prepare("SELECT value FROM state WHERE id=1")?;
        let mut rows = query.query([])?;
        Ok(match rows.next()? {
            Some(row) => Some(serde_json::from_str(&row.get::<_, String>(0)?)?),
            None => None,
        })
    }

    pub fn save(&self, state: &Value) -> Result<()> {
        self.0.execute("INSERT INTO state(id,value) VALUES(1,?1) ON CONFLICT(id) DO UPDATE SET value=excluded.value",
                       params![serde_json::to_string(state)?])?;
        Ok(())
    }
}
