use std::path::Path;

use nexode_proto::ProjectBudgetAlert;
use rusqlite::{Connection, OptionalExtension, params};
use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use crate::session::BudgetConfig;

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS token_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    slot_id TEXT NOT NULL,
    project_id TEXT NOT NULL,
    timestamp_ms INTEGER NOT NULL,
    tokens_in INTEGER NOT NULL,
    tokens_out INTEGER NOT NULL,
    model TEXT NOT NULL,
    cost_usd REAL NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_token_log_project_id ON token_log(project_id);
CREATE INDEX IF NOT EXISTS idx_token_log_slot_id ON token_log(slot_id);

CREATE VIEW IF NOT EXISTS project_costs AS
SELECT
    project_id,
    SUM(tokens_in) AS total_tokens_in,
    SUM(tokens_out) AS total_tokens_out,
    SUM(cost_usd) AS total_cost_usd
FROM token_log
GROUP BY project_id;
"#;

#[derive(Debug, Error)]
pub enum TokenAccountantError {
    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("{field} value `{value}` cannot be represented as a signed 64-bit integer")]
    IntegerOverflow { field: &'static str, value: u64 },
    #[error("{field} value `{value}` cannot be represented as an unsigned token count")]
    NegativeCount { field: &'static str, value: i64 },
}

#[derive(Debug, Error)]
pub enum TokenAccountingServiceError {
    #[error(transparent)]
    Accountant(#[from] TokenAccountantError),
    #[error("token accounting service is unavailable")]
    Unavailable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TokenUsageRecord {
    pub slot_id: String,
    pub project_id: String,
    pub timestamp_ms: i64,
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub model: String,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct CostTotals {
    pub tokens_in: u64,
    pub tokens_out: u64,
    pub cost_usd: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UsageUpdate {
    pub project_total: CostTotals,
    pub slot_total: CostTotals,
    pub session_total: CostTotals,
    pub budget_alert: Option<ProjectBudgetAlert>,
}

#[derive(Debug)]
pub struct TokenAccountant {
    connection: Connection,
}

#[derive(Debug, Clone)]
pub struct TokenAccountingHandle {
    tx: mpsc::Sender<AccountingRequest>,
}

#[derive(Debug)]
enum AccountingRequest {
    Record {
        record: TokenUsageRecord,
        budget: BudgetConfig,
        response: oneshot::Sender<Result<UsageUpdate, TokenAccountantError>>,
    },
    ProjectTotal {
        project_id: String,
        response: oneshot::Sender<Result<CostTotals, TokenAccountantError>>,
    },
    SlotTotal {
        project_id: String,
        slot_id: String,
        response: oneshot::Sender<Result<CostTotals, TokenAccountantError>>,
    },
    SessionTotal {
        response: oneshot::Sender<Result<CostTotals, TokenAccountantError>>,
    },
    ProjectBudgetAlert {
        project_id: String,
        budget: BudgetConfig,
        response: oneshot::Sender<Result<Option<ProjectBudgetAlert>, TokenAccountantError>>,
    },
}

impl TokenAccountant {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, TokenAccountantError> {
        let connection = Connection::open(path)?;
        Self::from_connection(connection)
    }

    pub fn open_in_memory() -> Result<Self, TokenAccountantError> {
        let connection = Connection::open_in_memory()?;
        Self::from_connection(connection)
    }

    pub fn record(&self, record: &TokenUsageRecord) -> Result<(), TokenAccountantError> {
        self.connection.execute(
            r#"
            INSERT INTO token_log (
                slot_id,
                project_id,
                timestamp_ms,
                tokens_in,
                tokens_out,
                model,
                cost_usd
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
            "#,
            params![
                &record.slot_id,
                &record.project_id,
                record.timestamp_ms,
                to_sql_i64("tokens_in", record.tokens_in)?,
                to_sql_i64("tokens_out", record.tokens_out)?,
                &record.model,
                record.cost_usd,
            ],
        )?;

        Ok(())
    }

    pub fn get_project_total(&self, project_id: &str) -> Result<CostTotals, TokenAccountantError> {
        let row = self
            .connection
            .query_row(
                r#"
                SELECT
                    total_tokens_in,
                    total_tokens_out,
                    total_cost_usd
                FROM project_costs
                WHERE project_id = ?1
                "#,
                params![project_id],
                |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, i64>(1)?,
                        row.get::<_, f64>(2)?,
                    ))
                },
            )
            .optional()?;

        match row {
            Some((tokens_in, tokens_out, cost_usd)) => Ok(CostTotals {
                tokens_in: to_u64("tokens_in", tokens_in)?,
                tokens_out: to_u64("tokens_out", tokens_out)?,
                cost_usd,
            }),
            None => Ok(CostTotals::default()),
        }
    }

    pub fn get_slot_total(
        &self,
        project_id: &str,
        slot_id: &str,
    ) -> Result<CostTotals, TokenAccountantError> {
        let (tokens_in, tokens_out, cost_usd) = self.connection.query_row(
            r#"
            SELECT
                COALESCE(SUM(tokens_in), 0),
                COALESCE(SUM(tokens_out), 0),
                COALESCE(SUM(cost_usd), 0.0)
            FROM token_log
            WHERE project_id = ?1 AND slot_id = ?2
            "#,
            params![project_id, slot_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            },
        )?;

        Ok(CostTotals {
            tokens_in: to_u64("tokens_in", tokens_in)?,
            tokens_out: to_u64("tokens_out", tokens_out)?,
            cost_usd,
        })
    }

    pub fn get_session_total(&self) -> Result<CostTotals, TokenAccountantError> {
        let (tokens_in, tokens_out, cost_usd) = self.connection.query_row(
            r#"
            SELECT
                COALESCE(SUM(tokens_in), 0),
                COALESCE(SUM(tokens_out), 0),
                COALESCE(SUM(cost_usd), 0.0)
            FROM token_log
            "#,
            [],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, i64>(1)?,
                    row.get::<_, f64>(2)?,
                ))
            },
        )?;

        Ok(CostTotals {
            tokens_in: to_u64("tokens_in", tokens_in)?,
            tokens_out: to_u64("tokens_out", tokens_out)?,
            cost_usd,
        })
    }

    pub fn project_budget_alert(
        &self,
        project_id: &str,
        budget: &BudgetConfig,
    ) -> Result<Option<ProjectBudgetAlert>, TokenAccountantError> {
        let totals = self.get_project_total(project_id)?;

        if let Some(limit_usd) = budget.max_usd
            && totals.cost_usd >= limit_usd
        {
            return Ok(Some(ProjectBudgetAlert {
                project_id: project_id.to_string(),
                current_usd: totals.cost_usd,
                limit_usd,
                hard_kill: true,
            }));
        }

        if let Some(limit_usd) = budget.warn_usd
            && totals.cost_usd >= limit_usd
        {
            return Ok(Some(ProjectBudgetAlert {
                project_id: project_id.to_string(),
                current_usd: totals.cost_usd,
                limit_usd,
                hard_kill: false,
            }));
        }

        Ok(None)
    }

    fn from_connection(connection: Connection) -> Result<Self, TokenAccountantError> {
        connection.execute_batch(SCHEMA)?;
        Ok(Self { connection })
    }
}

impl TokenAccountingHandle {
    pub fn start(path: impl AsRef<Path>) -> Result<Self, TokenAccountantError> {
        let accountant = TokenAccountant::open(path)?;
        Ok(Self::spawn(accountant))
    }

    pub fn start_in_memory() -> Result<Self, TokenAccountantError> {
        let accountant = TokenAccountant::open_in_memory()?;
        Ok(Self::spawn(accountant))
    }

    pub async fn record_usage(
        &self,
        record: TokenUsageRecord,
        budget: BudgetConfig,
    ) -> Result<UsageUpdate, TokenAccountingServiceError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(AccountingRequest::Record {
                record,
                budget,
                response: response_tx,
            })
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?;
        response_rx
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?
            .map_err(TokenAccountingServiceError::from)
    }

    pub async fn project_total(
        &self,
        project_id: impl Into<String>,
    ) -> Result<CostTotals, TokenAccountingServiceError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(AccountingRequest::ProjectTotal {
                project_id: project_id.into(),
                response: response_tx,
            })
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?;
        response_rx
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?
            .map_err(TokenAccountingServiceError::from)
    }

    pub async fn slot_total(
        &self,
        project_id: impl Into<String>,
        slot_id: impl Into<String>,
    ) -> Result<CostTotals, TokenAccountingServiceError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(AccountingRequest::SlotTotal {
                project_id: project_id.into(),
                slot_id: slot_id.into(),
                response: response_tx,
            })
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?;
        response_rx
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?
            .map_err(TokenAccountingServiceError::from)
    }

    pub async fn session_total(&self) -> Result<CostTotals, TokenAccountingServiceError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(AccountingRequest::SessionTotal {
                response: response_tx,
            })
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?;
        response_rx
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?
            .map_err(TokenAccountingServiceError::from)
    }

    pub async fn project_budget_alert(
        &self,
        project_id: impl Into<String>,
        budget: BudgetConfig,
    ) -> Result<Option<ProjectBudgetAlert>, TokenAccountingServiceError> {
        let (response_tx, response_rx) = oneshot::channel();
        self.tx
            .send(AccountingRequest::ProjectBudgetAlert {
                project_id: project_id.into(),
                budget,
                response: response_tx,
            })
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?;
        response_rx
            .await
            .map_err(|_| TokenAccountingServiceError::Unavailable)?
            .map_err(TokenAccountingServiceError::from)
    }

    fn spawn(accountant: TokenAccountant) -> Self {
        let (tx, mut rx) = mpsc::channel(64);
        std::thread::Builder::new()
            .name("nexode-accounting".to_string())
            .spawn(move || run_accounting_actor(accountant, &mut rx))
            .expect("spawn accounting actor");
        Self { tx }
    }
}

fn run_accounting_actor(accountant: TokenAccountant, rx: &mut mpsc::Receiver<AccountingRequest>) {
    let accountant = accountant;
    while let Some(request) = rx.blocking_recv() {
        match request {
            AccountingRequest::Record {
                record,
                budget,
                response,
            } => {
                let result = (|| {
                    accountant.record(&record)?;
                    Ok(UsageUpdate {
                        project_total: accountant.get_project_total(&record.project_id)?,
                        slot_total: accountant
                            .get_slot_total(&record.project_id, &record.slot_id)?,
                        session_total: accountant.get_session_total()?,
                        budget_alert: accountant
                            .project_budget_alert(&record.project_id, &budget)?,
                    })
                })();
                let _ = response.send(result);
            }
            AccountingRequest::ProjectTotal {
                project_id,
                response,
            } => {
                let _ = response.send(accountant.get_project_total(&project_id));
            }
            AccountingRequest::SlotTotal {
                project_id,
                slot_id,
                response,
            } => {
                let _ = response.send(accountant.get_slot_total(&project_id, &slot_id));
            }
            AccountingRequest::SessionTotal { response } => {
                let _ = response.send(accountant.get_session_total());
            }
            AccountingRequest::ProjectBudgetAlert {
                project_id,
                budget,
                response,
            } => {
                let _ = response.send(accountant.project_budget_alert(&project_id, &budget));
            }
        }
    }
}

fn to_sql_i64(field: &'static str, value: u64) -> Result<i64, TokenAccountantError> {
    i64::try_from(value).map_err(|_| TokenAccountantError::IntegerOverflow { field, value })
}

fn to_u64(field: &'static str, value: i64) -> Result<u64, TokenAccountantError> {
    u64::try_from(value).map_err(|_| TokenAccountantError::NegativeCount { field, value })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records_usage_and_aggregates_project_and_session_totals() {
        let accountant = TokenAccountant::open_in_memory().expect("in-memory accountant");

        accountant
            .record(&TokenUsageRecord {
                slot_id: "slot-a".to_string(),
                project_id: "project-1".to_string(),
                timestamp_ms: 1,
                tokens_in: 100,
                tokens_out: 40,
                model: "codex".to_string(),
                cost_usd: 1.25,
            })
            .expect("record usage");
        accountant
            .record(&TokenUsageRecord {
                slot_id: "slot-b".to_string(),
                project_id: "project-1".to_string(),
                timestamp_ms: 2,
                tokens_in: 50,
                tokens_out: 10,
                model: "codex".to_string(),
                cost_usd: 0.75,
            })
            .expect("record usage");
        accountant
            .record(&TokenUsageRecord {
                slot_id: "slot-c".to_string(),
                project_id: "project-2".to_string(),
                timestamp_ms: 3,
                tokens_in: 20,
                tokens_out: 5,
                model: "claude-code".to_string(),
                cost_usd: 0.5,
            })
            .expect("record usage");

        assert_eq!(
            accountant
                .get_project_total("project-1")
                .expect("project totals"),
            CostTotals {
                tokens_in: 150,
                tokens_out: 50,
                cost_usd: 2.0,
            }
        );
        assert_eq!(
            accountant
                .get_slot_total("project-1", "slot-a")
                .expect("slot totals"),
            CostTotals {
                tokens_in: 100,
                tokens_out: 40,
                cost_usd: 1.25,
            }
        );
        assert_eq!(
            accountant.get_session_total().expect("session totals"),
            CostTotals {
                tokens_in: 170,
                tokens_out: 55,
                cost_usd: 2.5,
            }
        );
    }

    #[test]
    fn emits_soft_then_hard_budget_alerts() {
        let accountant = TokenAccountant::open_in_memory().expect("in-memory accountant");
        let budget = BudgetConfig {
            max_usd: Some(10.0),
            warn_usd: Some(8.0),
        };

        accountant
            .record(&TokenUsageRecord {
                slot_id: "slot-a".to_string(),
                project_id: "project-1".to_string(),
                timestamp_ms: 1,
                tokens_in: 100,
                tokens_out: 50,
                model: "codex".to_string(),
                cost_usd: 8.5,
            })
            .expect("record usage");

        let soft_alert = accountant
            .project_budget_alert("project-1", &budget)
            .expect("soft alert check")
            .expect("soft alert");
        assert!(!soft_alert.hard_kill);
        assert_eq!(soft_alert.limit_usd, 8.0);

        accountant
            .record(&TokenUsageRecord {
                slot_id: "slot-b".to_string(),
                project_id: "project-1".to_string(),
                timestamp_ms: 2,
                tokens_in: 10,
                tokens_out: 10,
                model: "codex".to_string(),
                cost_usd: 2.0,
            })
            .expect("record usage");

        let hard_alert = accountant
            .project_budget_alert("project-1", &budget)
            .expect("hard alert check")
            .expect("hard alert");
        assert!(hard_alert.hard_kill);
        assert_eq!(hard_alert.limit_usd, 10.0);
        assert_eq!(hard_alert.current_usd, 10.5);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn accounting_handle_serializes_usage_updates() {
        let handle = TokenAccountingHandle::start_in_memory().expect("start accounting handle");
        let budget = BudgetConfig {
            max_usd: Some(10.0),
            warn_usd: Some(8.0),
        };

        let mut tasks = Vec::new();
        for idx in 0..4u64 {
            let handle = handle.clone();
            let budget = budget.clone();
            tasks.push(tokio::spawn(async move {
                handle
                    .record_usage(
                        TokenUsageRecord {
                            slot_id: format!("slot-{idx}"),
                            project_id: "project-1".to_string(),
                            timestamp_ms: idx as i64,
                            tokens_in: 10,
                            tokens_out: 5,
                            model: "codex".to_string(),
                            cost_usd: 2.5,
                        },
                        budget,
                    )
                    .await
                    .expect("record usage")
            }));
        }

        let mut saw_hard_alert = false;
        for task in tasks {
            let update = task.await.expect("join record task");
            saw_hard_alert |= update
                .budget_alert
                .as_ref()
                .is_some_and(|alert| alert.hard_kill);
        }

        assert!(saw_hard_alert);
        assert_eq!(
            handle
                .project_total("project-1")
                .await
                .expect("project totals after records"),
            CostTotals {
                tokens_in: 40,
                tokens_out: 20,
                cost_usd: 10.0,
            }
        );
        assert_eq!(
            handle
                .session_total()
                .await
                .expect("session totals after records"),
            CostTotals {
                tokens_in: 40,
                tokens_out: 20,
                cost_usd: 10.0,
            }
        );
    }
}
