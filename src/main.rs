use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber;

mod args;
mod auth;
mod cfg;
mod csv_sink;
mod job;
mod sheets;
mod state;
mod transform;

use args::Args;
use cfg::Cfg;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();
    
    // Initialize logging
    init_logging(&args.log_level)?;
    
    info!("Starting sheet_watch");
    
    // Load configuration
    let cfg = Cfg::load(args)?;
    
    // Initialize authentication
    let hub = auth::create_sheets_hub().await?;
    
    if cfg.once {
        info!("Running once and exiting");
        job::run_job(cfg, hub).await?;
    } else {
        // TODO: Implement scheduler logic for repeated runs
        info!("Scheduler mode not yet implemented");
        job::run_job(cfg, hub).await?;
    }
    
    info!("sheet_watch completed successfully");
    Ok(())
}

fn init_logging(level: &str) -> Result<()> {
    let filter = match level {
        "debug" => tracing::Level::DEBUG,
        "info" => tracing::Level::INFO,
        "warn" => tracing::Level::WARN,
        "error" => tracing::Level::ERROR,
        _ => tracing::Level::INFO,
    };
    
    tracing_subscriber::fmt()
        .with_max_level(filter)
        .init();
    
    Ok(())
} 