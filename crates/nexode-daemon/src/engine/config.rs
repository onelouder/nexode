use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use crate::observer::ObserverConfig;

pub(crate) const DEFAULT_LISTEN_ADDR: &str = "127.0.0.1:50051";
pub(crate) const DEFAULT_TICK_INTERVAL: Duration = Duration::from_secs(2);
pub(crate) const DEFAULT_CHECKPOINT_INTERVAL: Duration = Duration::from_secs(60);
pub(crate) const DEFAULT_VERIFICATION_TIMEOUT: Duration = Duration::from_secs(300);
pub(crate) const DEFAULT_WATCHDOG_POLL_INTERVAL: Duration = Duration::from_millis(250);
pub(crate) const DEFAULT_ACCOUNTING_DB: &str = ".nexode/token-accounting.sqlite3";
pub(crate) const DEFAULT_TARGET_BRANCH: &str = "main";

#[derive(Debug, Clone)]
pub struct DaemonConfig {
    pub session_path: PathBuf,
    pub listen_addr: SocketAddr,
    pub accounting_db_path: PathBuf,
    pub tick_interval: Duration,
    pub checkpoint_interval: Duration,
    pub verification_timeout: Duration,
    pub observer: ObserverConfig,
}

impl DaemonConfig {
    pub fn new(session_path: impl Into<PathBuf>) -> Self {
        Self {
            session_path: session_path.into(),
            listen_addr: DEFAULT_LISTEN_ADDR
                .parse()
                .expect("default daemon listen address is valid"),
            accounting_db_path: PathBuf::from(DEFAULT_ACCOUNTING_DB),
            tick_interval: DEFAULT_TICK_INTERVAL,
            checkpoint_interval: DEFAULT_CHECKPOINT_INTERVAL,
            verification_timeout: DEFAULT_VERIFICATION_TIMEOUT,
            observer: ObserverConfig::default(),
        }
    }
}
