use nexode_daemon::engine::{DaemonConfig, run_daemon};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let session_path = args.next().unwrap_or_else(|| "session.yaml".to_string());
    let mut config = DaemonConfig::new(session_path);

    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--listen" => {
                if let Some(addr) = args.next() {
                    config.listen_addr = addr.parse()?;
                }
            }
            "--accounting-db" => {
                if let Some(path) = args.next() {
                    config.accounting_db_path = path.into();
                }
            }
            "--tick-ms" => {
                if let Some(value) = args.next() {
                    config.tick_interval = std::time::Duration::from_millis(value.parse()?);
                }
            }
            "--verify-timeout-ms" => {
                if let Some(value) = args.next() {
                    config.verification_timeout = std::time::Duration::from_millis(value.parse()?);
                }
            }
            other => {
                return Err(format!("unknown argument `{other}`").into());
            }
        }
    }

    run_daemon(config).await?;
    Ok(())
}
