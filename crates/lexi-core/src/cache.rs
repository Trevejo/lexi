use crate::models::{LookupResult, LookupSource};
use rusqlite::{params, Connection};
use std::path::Path;

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, rusqlite::Error> {
        let conn = Connection::open(path)?;
        let cache = Self { conn };
        cache.init_schema()?;
        Ok(cache)
    }

    pub fn in_memory() -> Result<Self, rusqlite::Error> {
        let conn = Connection::open_in_memory()?;
        let cache = Self { conn };
        cache.init_schema()?;
        Ok(cache)
    }

    fn init_schema(&self) -> Result<(), rusqlite::Error> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS cache (
                normalized_query TEXT PRIMARY KEY,
                original_query TEXT NOT NULL,
                translation TEXT NOT NULL,
                pos TEXT,
                definition TEXT NOT NULL,
                example TEXT,
                source TEXT NOT NULL,
                lookup_count INTEGER NOT NULL DEFAULT 1,
                last_lookup_timestamp INTEGER NOT NULL
            );",
            [],
        )?;
        Ok(())
    }

    pub fn get(&self, normalized: &str) -> Option<LookupResult> {
        let clean = normalized.trim().to_lowercase();
        let mut stmt = self.conn.prepare(
            "SELECT original_query, translation, pos, definition, example FROM cache WHERE normalized_query = ?1 LIMIT 1"
        ).ok()?;

        let mut rows = stmt.query(params![clean]).ok()?;
        if let Some(row) = rows.next().ok()? {
            let original_query: String = row.get(0).ok()?;
            let translation: String = row.get(1).ok()?;
            let pos: Option<String> = row.get(2).ok();
            let definition: String = row.get(3).ok()?;
            let example: Option<String> = row.get(4).ok();

            // Update hit count asynchronously or in place
            let _ = self.conn.execute(
                "UPDATE cache SET lookup_count = lookup_count + 1, last_lookup_timestamp = unixepoch() WHERE normalized_query = ?1",
                params![clean],
            );

            return Some(LookupResult {
                query: original_query,
                normalized: clean,
                translation,
                part_of_speech: pos,
                definition,
                example,
                source: LookupSource::Cache,
            });
        }
        None
    }

    pub fn set(&self, result: &LookupResult) -> Result<(), rusqlite::Error> {
        let clean = result.normalized.trim().to_lowercase();
        let source_str = match result.source {
            LookupSource::Cache => "cache",
            LookupSource::OfflineDictionary => "offline_dictionary",
            LookupSource::Llm => "llm",
        };

        self.conn.execute(
            "INSERT INTO cache (normalized_query, original_query, translation, pos, definition, example, source, lookup_count, last_lookup_timestamp)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 1, unixepoch())
             ON CONFLICT(normalized_query) DO UPDATE SET
                translation = excluded.translation,
                pos = excluded.pos,
                definition = excluded.definition,
                example = excluded.example,
                lookup_count = lookup_count + 1,
                last_lookup_timestamp = unixepoch()",
            params![clean, result.query, result.translation, result.part_of_speech, result.definition, result.example, source_str],
        )?;
        Ok(())
    }
}
