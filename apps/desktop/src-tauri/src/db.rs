use async_trait::async_trait;
use audit_logger::{AuditEvent as PolicyAuditEvent, AuditError as AuditSinkError, AuditSink};
use serde::Serialize;
use sqlx::migrate;
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Row, SqlitePool};
use std::path::{Path, PathBuf};
use thiserror::Error;
use time::format_description::well_known::Rfc3339;

const DB_FILE: &str = "multinet-cybershield-ai.db";

static MIGRATOR: migrate::Migrator = migrate!("../../../database/sqlite/migrations");

#[derive(Debug, Error)]
pub enum DbError {
    #[error("failed to create application data directory {path}: {source}")]
    CreateDir {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("database error: {0}")]
    Database(#[from] sqlx::Error),
    #[error("migration error: {0}")]
    Migration(#[from] migrate::MigrateError),
}

pub struct DatabaseState {
    pub pool: SqlitePool,
    pub db_file: PathBuf,
}

#[derive(Debug, Serialize)]
pub struct DbStatus {
    pub db_file: String,
    pub migrations_applied: i64,
    pub latest_version: Option<i64>,
    pub foreign_keys_enabled: bool,
}

pub async fn initialize_database(data_dir: &Path) -> Result<DatabaseState, DbError> {
    std::fs::create_dir_all(data_dir).map_err(|source| DbError::CreateDir {
        path: data_dir.to_path_buf(),
        source,
    })?;

    let db_file = data_dir.join(DB_FILE);
    let connect_options = SqliteConnectOptions::new()
        .filename(&db_file)
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal)
        .busy_timeout(std::time::Duration::from_secs(5));

    let pool = SqlitePoolOptions::new()
        .max_connections(5)
        .connect_with(connect_options)
        .await?;

    MIGRATOR.run(&pool).await?;

    Ok(DatabaseState {
        pool,
        db_file,
    })
}

pub async fn database_status(state: &DatabaseState) -> Result<DbStatus, DbError> {
    let row = sqlx::query("SELECT COUNT(*) AS applied, MAX(version) AS latest FROM _sqlx_migrations")
        .fetch_one(&state.pool)
        .await?;
    let foreign_keys = sqlx::query("PRAGMA foreign_keys")
        .fetch_one(&state.pool)
        .await?;

    Ok(DbStatus {
        db_file: state.db_file.to_string_lossy().into_owned(),
        migrations_applied: row.try_get("applied")?,
        latest_version: row.try_get("latest")?,
        foreign_keys_enabled: foreign_keys.try_get::<i64, usize>(0)? == 1,
    })
}

/// Durable implementation of `audit_logger::AuditSink` writing every event into
/// the `audit_logs` table before returning. No destination for discarded events
/// exists: an insert failure is returned to the caller (the policy core maps it
/// to `PolicyError::Audit`), so audit failures are never silent.
pub struct SqliteAuditSink {
    pool: SqlitePool,
}

/// Read model for audit rows (distinct from the transient `AuditEvent` input).
#[derive(Debug, Serialize)]
pub struct StoredAuditEvent {
    pub id: String,
    pub kind: String,
    pub subject_id: Option<String>,
    pub detail: String,
    pub created_at: String,
}

impl SqliteAuditSink {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    #[allow(dead_code)] // exercised by db tests only (no audit-viewer command yet)
    pub async fn count_events(&self) -> Result<i64, DbError> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM audit_logs")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("n")?)
    }

    #[allow(dead_code)] // exercised by db tests only (no audit-viewer command yet)

    pub async fn events_of_kind(&self, kind: &str) -> Result<Vec<StoredAuditEvent>, DbError> {
        let rows = sqlx::query(
            "SELECT id, kind, subject_id, detail, created_at FROM audit_logs \
             WHERE kind = $1 ORDER BY created_at",
        )
        .bind(kind)
        .fetch_all(&self.pool)
        .await?;
        let mut events = Vec::with_capacity(rows.len());
        for row in rows.iter() {
            events.push(StoredAuditEvent {
                id: row.try_get("id")?,
                kind: row.try_get("kind")?,
                subject_id: row.try_get("subject_id")?,
                detail: row.try_get("detail")?,
                created_at: row.try_get("created_at")?,
            });
        }
        Ok(events)
    }

    #[allow(dead_code)] // exercised by db tests only (no audit-viewer command yet)

    pub async fn count_events_for_subject(&self, subject_id: &str) -> Result<i64, DbError> {
        let row = sqlx::query("SELECT COUNT(*) AS n FROM audit_logs WHERE subject_id = $1")
            .bind(subject_id)
            .fetch_one(&self.pool)
            .await?;
        Ok(row.try_get("n")?)
    }
}

#[async_trait]
impl AuditSink for SqliteAuditSink {
    async fn record(&self, event: PolicyAuditEvent) -> Result<(), AuditSinkError> {
        let created_at = event
            .at
            .format(&Rfc3339)
            .map_err(|e| AuditSinkError::Write(format!("timestamp encode failed: {e}")))?;
        sqlx::query(
            "INSERT INTO audit_logs (id, kind, subject_id, actor, detail, created_at) \
             VALUES ($1, $2, $3, NULL, $4, $5)",
        )
        .bind(event.id.to_string())
        .bind(&event.kind)
        .bind(event.subject_id.map(|id| id.to_string()))
        .bind(&event.detail)
        .bind(created_at)
        .execute(&self.pool)
        .await
        .map_err(|e| AuditSinkError::Write(e.to_string()))?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use security_policy::{ActionRequest, Decision, HumanApprovalPolicy, SecurityPolicy};
    use time::OffsetDateTime;
    use uuid::Uuid;

    #[tokio::test]
    async fn initializes_and_applies_migrations() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();
        let status = database_status(&state).await.unwrap();

        assert_eq!(status.migrations_applied, 3);
        assert_eq!(status.latest_version, Some(3));
        assert!(status.foreign_keys_enabled);
        assert!(state.db_file.ends_with(DB_FILE));

        state.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn enforces_foreign_keys() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();

        let orphan = sqlx::query(
            "INSERT INTO connection_metrics (interface_id, measured_at, source) \
             VALUES ('no-such-interface', '2026-01-01T00:00:00Z', 'test')",
        )
        .execute(&state.pool)
        .await;

        assert!(orphan.is_err(), "orphan metric insert must violate the foreign key");

        state.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn never_touches_prewritten_schema_and_is_idempotent() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();
        initialize_database(&data_dir).await.unwrap();

        let tables = sqlx::query("SELECT COUNT(*) AS n FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%'")
            .fetch_one(&state.pool)
            .await
            .unwrap();
        assert_eq!(tables.try_get::<i64, _>("n").unwrap(), 19);

        state.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn audit_sink_persists_across_restart() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();
        let sink = SqliteAuditSink::new(state.pool.clone());
        sink.record(PolicyAuditEvent {
            id: Uuid::new_v4(),
            kind: "policy_denied".into(),
            subject_id: Some(Uuid::new_v4()),
            detail: "arbitrary shell execution denied".into(),
            at: OffsetDateTime::now_utc(),
        })
        .await
        .unwrap();
        sink.record(PolicyAuditEvent {
            id: Uuid::new_v4(),
            kind: "approval_requested".into(),
            subject_id: None,
            detail: "apply firewall rule".into(),
            at: OffsetDateTime::now_utc(),
        })
        .await
        .unwrap();
        assert_eq!(sink.count_events().await.unwrap(), 2);

        // Simulate an application restart: close the pool, re-run initialization
        // against the same file (migrations must be idempotent) and re-read.
        state.pool.close().await;
        let restarted = initialize_database(&data_dir).await.unwrap();
        let sink_after_restart = SqliteAuditSink::new(restarted.pool.clone());
        assert_eq!(sink_after_restart.count_events().await.unwrap(), 2);
        let denied = sink_after_restart.events_of_kind("policy_denied").await.unwrap();
        assert_eq!(denied.len(), 1);
        assert_eq!(denied[0].detail, "arbitrary shell execution denied");
        assert!(denied[0].subject_id.is_some());
        assert!(denied[0].created_at.ends_with('Z') && denied[0].created_at.contains('T'));
        let approvals = sink_after_restart.events_of_kind("approval_requested").await.unwrap();
        assert_eq!(approvals.len(), 1);

        restarted.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }

    #[tokio::test]
    async fn policy_decisions_are_durable() {
        let data_dir = std::env::temp_dir().join(format!("multinet-db-test-{}", uuid::Uuid::new_v4()));

        let state = initialize_database(&data_dir).await.unwrap();
        let sink = SqliteAuditSink::new(state.pool.clone());
        let policy = HumanApprovalPolicy::new(sink);

        // Shell execution must be denied and the denial must hit durable storage.
        let shell_request = ActionRequest {
            id: Uuid::new_v4(),
            action: "run shell command".into(),
            target: "host".into(),
            reason: "day 10 durability test".into(),
            requested_by: "qa".into(),
        };
        let decision = policy.validate_action(&shell_request).await.unwrap();
        assert!(matches!(decision, Decision::Deny { .. }));

        // Defensive actions require approval; the request must be audited with
        // the same request id recorded as the event subject.
        let firewall_request = ActionRequest {
            id: Uuid::new_v4(),
            action: "apply firewall rule".into(),
            target: "local".into(),
            reason: "day 10 durability test".into(),
            requested_by: "qa".into(),
        };
        let approval_id = match policy.validate_action(&firewall_request).await.unwrap() {
            Decision::RequiresApproval { request_id } => request_id,
            other => panic!("expected RequiresApproval, got {other:?}"),
        };
        policy.request_approval(&firewall_request).await.unwrap();

        // Executing must remain refused in this phase (no system mutations), and
        // the refusal must be audited as an action_approved record.
        let execution = policy
            .execute_approved_action(
                &firewall_request,
                &security_policy::Approval {
                    request_id: approval_id,
                    approved_by: "qa".into(),
                },
            )
            .await;
        assert!(execution.is_err(), "foundation phase must refuse execution");

        state.pool.close().await;

        // Restart and verify all three events survived with correct subjects.
        let restarted = initialize_database(&data_dir).await.unwrap();
        let sink_after_restart = SqliteAuditSink::new(restarted.pool.clone());
        assert_eq!(sink_after_restart.count_events().await.unwrap(), 3);
        assert_eq!(
            sink_after_restart
                .count_events_for_subject(&shell_request.id.to_string())
                .await
                .unwrap(),
            1
        );
        assert_eq!(
            sink_after_restart
                .count_events_for_subject(&approval_id.to_string())
                .await
                .unwrap(),
            2
        );

        restarted.pool.close().await;
        let _ = std::fs::remove_dir_all(&data_dir);
    }
}
