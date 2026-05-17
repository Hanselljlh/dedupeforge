use anyhow::{Context, Result};
use dedupe_core::{MatchRisk, ScanReport};
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

pub struct ReportDb {
    connection: Connection,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredReportSummary {
    pub id: i64,
    pub created_at_unix: i64,
    pub name: String,
    pub mode: String,
    pub risk: String,
    pub scanned_files: usize,
    pub group_count: usize,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredReport {
    pub summary: StoredReportSummary,
    pub report: ScanReport,
}

impl ReportDb {
    pub fn open(path: &Path) -> Result<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let connection = Connection::open(path)
            .with_context(|| format!("failed to open report database {}", path.display()))?;
        let db = Self { connection };
        db.initialize()?;
        Ok(db)
    }

    pub fn store_report(&self, name: &str, report: &ScanReport) -> Result<i64> {
        let report_json = serde_json::to_string(report)?;
        self.connection.execute(
            "insert into reports (
                created_at_unix,
                name,
                mode,
                risk,
                scanned_files,
                group_count,
                report_json
            ) values (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params![
                unix_now(),
                name,
                format!("{:?}", report.mode).to_lowercase(),
                risk_label(report.risk),
                report.scanned_files as i64,
                report.duplicate_groups.len() as i64,
                report_json
            ],
        )?;
        Ok(self.connection.last_insert_rowid())
    }

    pub fn list_reports(&self) -> Result<Vec<StoredReportSummary>> {
        let mut statement = self.connection.prepare(
            "select id, created_at_unix, name, mode, risk, scanned_files, group_count
             from reports
             order by id desc",
        )?;
        let rows = statement.query_map([], |row| {
            Ok(StoredReportSummary {
                id: row.get(0)?,
                created_at_unix: row.get(1)?,
                name: row.get(2)?,
                mode: row.get(3)?,
                risk: row.get(4)?,
                scanned_files: row.get::<_, i64>(5)? as usize,
                group_count: row.get::<_, i64>(6)? as usize,
            })
        })?;
        let mut reports = Vec::new();
        for row in rows {
            reports.push(row?);
        }
        Ok(reports)
    }

    pub fn load_report(&self, id: i64) -> Result<StoredReport> {
        let mut statement = self.connection.prepare(
            "select id, created_at_unix, name, mode, risk, scanned_files, group_count, report_json
             from reports
             where id = ?1",
        )?;
        let stored = statement.query_row([id], |row| {
            let report_json: String = row.get(7)?;
            let report: ScanReport = serde_json::from_str(&report_json).map_err(|err| {
                rusqlite::Error::FromSqlConversionFailure(
                    7,
                    rusqlite::types::Type::Text,
                    Box::new(err),
                )
            })?;
            Ok(StoredReport {
                summary: StoredReportSummary {
                    id: row.get(0)?,
                    created_at_unix: row.get(1)?,
                    name: row.get(2)?,
                    mode: row.get(3)?,
                    risk: row.get(4)?,
                    scanned_files: row.get::<_, i64>(5)? as usize,
                    group_count: row.get::<_, i64>(6)? as usize,
                },
                report,
            })
        })?;
        Ok(stored)
    }

    fn initialize(&self) -> Result<()> {
        self.connection.execute_batch(
            "create table if not exists reports (
                id integer primary key autoincrement,
                created_at_unix integer not null,
                name text not null,
                mode text not null,
                risk text not null,
                scanned_files integer not null,
                group_count integer not null,
                report_json text not null
            );",
        )?;
        Ok(())
    }
}

fn risk_label(risk: MatchRisk) -> &'static str {
    match risk {
        MatchRisk::Low => "low",
        MatchRisk::Medium => "medium",
        MatchRisk::High => "high",
    }
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

#[cfg(test)]
mod tests {
    use super::*;
    use dedupe_core::{DuplicateGroup, DuplicateItem, ScanMode};
    use std::path::PathBuf;

    fn sample_report() -> ScanReport {
        ScanReport {
            mode: ScanMode::Exact,
            scanned_files: 2,
            candidate_size_groups: 1,
            cache_hits: 0,
            cache_misses: 2,
            duplicate_groups: vec![DuplicateGroup {
                size: 4,
                algorithm: "blake3".to_string(),
                hash: "abcd".to_string(),
                reason: "same size + same full hash".to_string(),
                items: vec![
                    DuplicateItem {
                        path: PathBuf::from("keep.txt"),
                        size: 4,
                        modified_unix: Some(1),
                        is_protected: false,
                        suggested_keep: true,
                    },
                    DuplicateItem {
                        path: PathBuf::from("copy.txt"),
                        size: 4,
                        modified_unix: Some(2),
                        is_protected: false,
                        suggested_keep: false,
                    },
                ],
            }],
            errors: Vec::new(),
            risk: MatchRisk::Low,
        }
    }

    #[test]
    fn stores_and_loads_reports() {
        let root = std::env::temp_dir().join(format!("dedupe-report-db-{}", unix_now()));
        std::fs::create_dir_all(&root).unwrap();
        let db = ReportDb::open(&root.join("reports.sqlite3")).unwrap();

        let id = db.store_report("nightly", &sample_report()).unwrap();
        let summaries = db.list_reports().unwrap();
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].id, id);

        let loaded = db.load_report(id).unwrap();
        assert_eq!(loaded.summary.name, "nightly");
        assert_eq!(loaded.report.duplicate_groups.len(), 1);

        let _ = std::fs::remove_dir_all(root);
    }
}
