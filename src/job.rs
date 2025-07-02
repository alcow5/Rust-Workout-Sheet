use anyhow::Result;
use google_sheets4::{Sheets, hyper_rustls, hyper};
use tracing::{info, debug, warn};
use crate::{
    cfg::Cfg,
    state::{load_state, save_state},
    sheets::fetch_rows,
    transform::normalize_row,
    csv_sink::append,
};

pub async fn run_job(
    cfg: Cfg,
    hub: Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
) -> Result<()> {
    info!("Starting job execution");
    
    // Validate configuration
    cfg.validate()?;
    
    // Step a) Load last_processed_row from state.json, default zero
    let mut state = load_state(&cfg.state_path)?;
    let start_row = state.last_processed_row + 1;
    
    info!("Starting from row: {}", start_row);
    
    // Step b) GET values from raw_range starting at last_processed_row plus one
    let raw_rows = fetch_rows(&hub, &cfg.sheet_id, &cfg.raw_range, start_row).await?;
    
    if raw_rows.is_empty() {
        info!("No new rows found, job complete");
        return Ok(());
    }
    
    info!("Found {} new rows to process", raw_rows.len());
    
    // Step c) For each returned Vec<String> call normalize_row, collect NormalizedRow values
    let mut normalized_rows = Vec::new();
    let mut processed_count = 0;
    
    for (index, raw_row) in raw_rows.iter().enumerate() {
        match normalize_row(raw_row.clone()) {
            Ok(normalized) => {
                normalized_rows.push(normalized);
                processed_count += 1;
            }
            Err(e) => {
                warn!("Failed to normalize row {}: {}", start_row + index, e);
                // TODO: Decide whether to continue or fail on normalization errors
                continue;
            }
        }
    }
    
    debug!("Successfully normalized {}/{} rows", processed_count, raw_rows.len());
    
    // Step d) If list not empty, call csv_sink::append
    if !normalized_rows.is_empty() {
        append(&cfg.output_csv.path, &normalized_rows, cfg.output_csv.ensure)?;
        info!("Appended {} normalized rows to CSV", normalized_rows.len());
    } else {
        warn!("No rows were successfully normalized");
    }
    
    // Step e) Update state.json with new last_processed_row
    state.update_processed(raw_rows.len());
    save_state(&cfg.state_path, &state)?;
    
    // Step f) Log completion
    info!("Job completed successfully. Processed {} rows. Total processed: {}", 
          raw_rows.len(), state.total_processed);
    
    Ok(())
}

pub async fn run_with_error_handling(
    cfg: Cfg,
    hub: Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
) -> Result<()> {
    match run_job(cfg, hub).await {
        Ok(()) => {
            info!("Job completed successfully");
            Ok(())
        }
        Err(e) => {
            warn!("Job failed with error: {}", e);
            // TODO: Add error recovery logic, notifications, etc.
            Err(e)
        }
    }
}

pub fn should_run_job() -> bool {
    // TODO: Add logic to determine if job should run
    // This could check for lock files, time-based schedules, etc.
    true
} 