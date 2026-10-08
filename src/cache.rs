// Local SQLite store. The schema is the same as the Go version's, so the
// existing cache.db opens without a migration.

use crate::problem::{Contest, Problem, RatingChange, Submission};
use anyhow::Result;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;

const SCHEMA: &str = "
CREATE TABLE IF NOT EXISTS problems (
    platform TEXT NOT NULL,
    id       TEXT NOT NULL,
    name     TEXT NOT NULL,
    url      TEXT NOT NULL,
    rating   INTEGER NOT NULL DEFAULT 0,
    tags     TEXT NOT NULL DEFAULT '[]',
    category TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (platform, id)
);
CREATE TABLE IF NOT EXISTS submissions (
    platform     TEXT NOT NULL,
    id           TEXT NOT NULL,
    problem_id   TEXT NOT NULL,
    verdict      TEXT NOT NULL,
    language     TEXT NOT NULL,
    submitted_at INTEGER NOT NULL,
    PRIMARY KEY (platform, id)
);
CREATE TABLE IF NOT EXISTS contests (
    platform         TEXT NOT NULL,
    id               TEXT NOT NULL,
    name             TEXT NOT NULL,
    url              TEXT NOT NULL,
    start_time       INTEGER NOT NULL,
    duration_seconds INTEGER NOT NULL,
    PRIMARY KEY (platform, id)
);
CREATE TABLE IF NOT EXISTS rating_history (
    platform     TEXT NOT NULL,
    contest_id   TEXT NOT NULL,
    contest_name TEXT NOT NULL,
    old_rating   INTEGER NOT NULL,
    new_rating   INTEGER NOT NULL,
    at           INTEGER NOT NULL,
    PRIMARY KEY (platform, contest_id)
);
CREATE TABLE IF NOT EXISTS sync_meta (
    key   TEXT PRIMARY KEY,
    value TEXT NOT NULL
);";

pub struct Cache {
    conn: Connection,
}

impl Cache {
    pub fn open(path: &Path) -> Result<Cache> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(SCHEMA)?;
        Ok(Cache { conn })
    }

    /// An in-memory cache, for tests.
    pub fn open_in_memory() -> Result<Cache> {
        let conn = Connection::open_in_memory()?;
        conn.execute_batch(SCHEMA)?;
        Ok(Cache { conn })
    }

    pub fn upsert_problems(&mut self, problems: &[Problem]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO problems (platform, id, name, url, rating, tags, category)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
                 ON CONFLICT(platform, id) DO UPDATE SET
                    name = excluded.name, url = excluded.url, rating = excluded.rating,
                    tags = excluded.tags, category = excluded.category",
            )?;
            for p in problems {
                let tags = serde_json::to_string(&p.tags)?;
                stmt.execute(params![p.platform, p.id, p.name, p.url, p.rating, tags, p.category])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    pub fn list_problems(&self, platform: &str) -> Result<Vec<Problem>> {
        let mut stmt = self.conn.prepare(
            "SELECT platform, id, name, url, rating, tags, category FROM problems WHERE platform = ?1 ORDER BY id",
        )?;
        let rows = stmt.query_map(params![platform], |r| {
            let tags: String = r.get(5)?;
            Ok(Problem {
                platform: r.get(0)?,
                id: r.get(1)?,
                name: r.get(2)?,
                url: r.get(3)?,
                rating: r.get(4)?,
                tags: serde_json::from_str(&tags).unwrap_or_default(),
                category: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn count_problems(&self) -> Result<i64> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM problems", [], |r| r.get(0))?)
    }

    pub fn upsert_submissions(&mut self, subs: &[Submission]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO submissions (platform, id, problem_id, verdict, language, submitted_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(platform, id) DO UPDATE SET
                    problem_id = excluded.problem_id, verdict = excluded.verdict,
                    language = excluded.language, submitted_at = excluded.submitted_at",
            )?;
            for s in subs {
                stmt.execute(params![s.platform, s.id, s.problem_id, s.verdict, s.language, s.submitted_at])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// One platform's submissions, newest first.
    pub fn list_submissions(&self, platform: &str) -> Result<Vec<Submission>> {
        let mut stmt = self.conn.prepare(
            "SELECT platform, id, problem_id, verdict, language, submitted_at
             FROM submissions WHERE platform = ?1 ORDER BY submitted_at DESC",
        )?;
        let rows = stmt.query_map(params![platform], |r| {
            Ok(Submission {
                platform: r.get(0)?,
                id: r.get(1)?,
                problem_id: r.get(2)?,
                verdict: r.get(3)?,
                language: r.get(4)?,
                submitted_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn upsert_rating_changes(&mut self, changes: &[RatingChange]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO rating_history (platform, contest_id, contest_name, old_rating, new_rating, at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(platform, contest_id) DO UPDATE SET
                    contest_name = excluded.contest_name, old_rating = excluded.old_rating,
                    new_rating = excluded.new_rating, at = excluded.at",
            )?;
            for c in changes {
                stmt.execute(params![c.platform, c.contest_id, c.contest_name, c.old_rating, c.new_rating, c.at])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// One platform's rating history, oldest first.
    pub fn list_rating_changes(&self, platform: &str) -> Result<Vec<RatingChange>> {
        let mut stmt = self.conn.prepare(
            "SELECT platform, contest_id, contest_name, old_rating, new_rating, at
             FROM rating_history WHERE platform = ?1 ORDER BY at",
        )?;
        let rows = stmt.query_map(params![platform], |r| {
            Ok(RatingChange {
                platform: r.get(0)?,
                contest_id: r.get(1)?,
                contest_name: r.get(2)?,
                old_rating: r.get(3)?,
                new_rating: r.get(4)?,
                at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn upsert_contests(&mut self, contests: &[Contest]) -> Result<()> {
        let tx = self.conn.transaction()?;
        {
            let mut stmt = tx.prepare(
                "INSERT INTO contests (platform, id, name, url, start_time, duration_seconds)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                 ON CONFLICT(platform, id) DO UPDATE SET
                    name = excluded.name, url = excluded.url,
                    start_time = excluded.start_time, duration_seconds = excluded.duration_seconds",
            )?;
            for k in contests {
                stmt.execute(params![k.platform, k.id, k.name, k.url, k.start, k.duration])?;
            }
        }
        tx.commit()?;
        Ok(())
    }

    /// One platform's contests, soonest first.
    pub fn list_contests(&self, platform: &str) -> Result<Vec<Contest>> {
        let mut stmt = self.conn.prepare(
            "SELECT platform, id, name, url, start_time, duration_seconds
             FROM contests WHERE platform = ?1 ORDER BY start_time",
        )?;
        let rows = stmt.query_map(params![platform], |r| {
            Ok(Contest {
                platform: r.get(0)?,
                id: r.get(1)?,
                name: r.get(2)?,
                url: r.get(3)?,
                start: r.get(4)?,
                duration: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn set_meta(&self, key: &str, value: &str) -> Result<()> {
        self.conn.execute(
            "INSERT INTO sync_meta (key, value) VALUES (?1, ?2)
             ON CONFLICT(key) DO UPDATE SET value = excluded.value",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn get_meta(&self, key: &str) -> Result<Option<String>> {
        Ok(self
            .conn
            .query_row("SELECT value FROM sync_meta WHERE key = ?1", params![key], |r| r.get(0))
            .optional()?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn problems_upsert_and_list() {
        let mut c = Cache::open_in_memory().unwrap();
        let p = Problem {
            platform: "codeforces".into(),
            id: "4A".into(),
            name: "Watermelon".into(),
            url: "u".into(),
            rating: 800,
            tags: vec!["math".into()],
            category: String::new(),
        };
        c.upsert_problems(&[p.clone()]).unwrap();
        let mut renamed = p.clone();
        renamed.name = "Watermelon (renamed)".into();
        c.upsert_problems(&[renamed]).unwrap();
        let got = c.list_problems("codeforces").unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "Watermelon (renamed)");
        assert_eq!(got[0].tags, vec!["math".to_string()]);
        assert_eq!(c.count_problems().unwrap(), 1);
    }

    #[test]
    fn meta_round_trip() {
        let c = Cache::open_in_memory().unwrap();
        assert_eq!(c.get_meta("k").unwrap(), None);
        c.set_meta("k", "1").unwrap();
        c.set_meta("k", "2").unwrap();
        assert_eq!(c.get_meta("k").unwrap(), Some("2".to_string()));
    }

    #[test]
    fn submissions_newest_first() {
        let mut c = Cache::open_in_memory().unwrap();
        let s = |id: &str, t: i64| Submission {
            platform: "codeforces".into(),
            id: id.into(),
            problem_id: "4A".into(),
            verdict: "OK".into(),
            language: "GNU C++17".into(),
            submitted_at: t,
        };
        c.upsert_submissions(&[s("1", 100), s("2", 200)]).unwrap();
        let got = c.list_submissions("codeforces").unwrap();
        assert_eq!(got[0].id, "2");
    }
}
