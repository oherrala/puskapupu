use std::path::PathBuf;

use argh::FromArgs;

use puskapupu::{config, cqgma, matrix};

/// A Matrix bot alerting hunters for movements of activators
#[derive(Debug, FromArgs)]
struct Cli {
    /// config file
    #[argh(option, short = 'c', long = "config")]
    config: PathBuf,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();
    let cli: Cli = argh::from_env();

    let config = config::Config::read_from_file(cli.config)?;
    let mut fut = Vec::new();

    tracing::info!("Staring CQGMA stuff...");
    let cqgma_state = cqgma::cqgma_init(&config.cqgma).await;
    fut.push(cqgma_state.handle);

    tracing::info!("Starting Matrix stuff...");
    let handles = matrix::matrix_init(&config.matrix, cqgma_state.telnet_rx).await?;
    fut.extend(handles);

    loop {
        for handle in &fut {
            if handle.is_finished() {
                tracing::error!("Task {handle:?} has finished. This is error. Exiting.");
                panic!("Task {handle:?} has finished. This is error. Exiting.");
            }
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}
