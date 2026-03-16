use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use nexode_daemon::engine::{DaemonConfig, run_daemon};

#[derive(Debug, Parser)]
#[command(name = "nexode-daemon", about = "Nexode daemon process", version)]
struct Cli {
    #[arg(value_name = "SESSION", conflicts_with = "session")]
    session_path: Option<PathBuf>,
    #[arg(long, value_name = "PATH")]
    session: Option<PathBuf>,
    #[arg(long)]
    listen: Option<SocketAddr>,
    #[arg(long, conflicts_with = "listen")]
    port: Option<u16>,
    #[arg(long = "accounting-db", value_name = "PATH")]
    accounting_db: Option<PathBuf>,
    #[arg(long = "tick-ms", value_name = "MILLIS")]
    tick_ms: Option<u64>,
    #[arg(long = "verify-timeout-ms", value_name = "MILLIS")]
    verify_timeout_ms: Option<u64>,
}

impl Cli {
    fn into_config(self) -> DaemonConfig {
        let session_path = self
            .session
            .or(self.session_path)
            .unwrap_or_else(|| PathBuf::from("session.yaml"));
        let mut config = DaemonConfig::new(session_path);

        if let Some(listen_addr) = self.listen {
            config.listen_addr = listen_addr;
        } else if let Some(port) = self.port {
            config.listen_addr = SocketAddr::new(config.listen_addr.ip(), port);
        }
        if let Some(accounting_db_path) = self.accounting_db {
            config.accounting_db_path = accounting_db_path;
        }
        if let Some(tick_ms) = self.tick_ms {
            config.tick_interval = Duration::from_millis(tick_ms);
        }
        if let Some(verify_timeout_ms) = self.verify_timeout_ms {
            config.verification_timeout = Duration::from_millis(verify_timeout_ms);
        }

        config
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    run_daemon(Cli::parse().into_config()).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use clap::{CommandFactory, error::ErrorKind};

    use super::*;

    #[test]
    fn parses_existing_flags_and_positional_session_path() {
        let cli = Cli::try_parse_from([
            "nexode-daemon",
            "custom-session.yaml",
            "--listen",
            "127.0.0.1:6000",
            "--accounting-db",
            "acct.sqlite3",
            "--tick-ms",
            "25",
            "--verify-timeout-ms",
            "900",
        ])
        .expect("parse daemon cli");
        let config = cli.into_config();

        assert_eq!(config.session_path, PathBuf::from("custom-session.yaml"));
        assert_eq!(
            config.listen_addr,
            "127.0.0.1:6000".parse().expect("valid socket addr")
        );
        assert_eq!(config.accounting_db_path, PathBuf::from("acct.sqlite3"));
        assert_eq!(config.tick_interval, Duration::from_millis(25));
        assert_eq!(config.verification_timeout, Duration::from_millis(900));
    }

    #[test]
    fn parses_session_and_port_flags() {
        let cli = Cli::try_parse_from([
            "nexode-daemon",
            "--session",
            "session.yaml",
            "--port",
            "7000",
        ])
        .expect("parse daemon cli");
        let config = cli.into_config();

        assert_eq!(config.session_path, PathBuf::from("session.yaml"));
        assert_eq!(config.listen_addr, "127.0.0.1:7000".parse().unwrap());
    }

    #[test]
    fn help_and_version_flags_are_exposed() {
        let help = Cli::try_parse_from(["nexode-daemon", "--help"]).unwrap_err();
        assert_eq!(help.kind(), ErrorKind::DisplayHelp);

        let version = Cli::try_parse_from(["nexode-daemon", "--version"]).unwrap_err();
        assert_eq!(version.kind(), ErrorKind::DisplayVersion);

        let help_text = Cli::command().render_help().to_string();
        assert!(help_text.contains("--session"));
        assert!(help_text.contains("--version"));
    }
}
