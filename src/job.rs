use anyhow::Result;
use google_sheets4::{Sheets, hyper_rustls, hyper};
use tracing::{info, warn};
use crate::{
    cfg::Cfg,
    state::{load_state, save_state},
    sheets::{fetch_rows, discover_block_tabs, detect_block_extent},
    transform::normalize_block_data,
    csv_sink::append,
};

pub async fn run_job(
    cfg: Cfg,
    hub: Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
) -> Result<()> {
    info!("Starting job execution");
    
    // Validate configuration
    cfg.validate()?;
    
    // Load state
    let mut state = load_state(&cfg.state_path)?;
    
    // Get all ranges to process - either from legacy config or auto-discovery
    let ranges = if let Some(legacy_ranges) = cfg.get_legacy_block_ranges() {
        info!("Using configured ranges");
        legacy_ranges
    } else {
        info!("Auto-discovering block tabs from spreadsheet");
        let discovered_blocks = discover_block_tabs(&hub, &cfg.sheet_id).await?;
        
        if discovered_blocks.is_empty() {
            anyhow::bail!("No block tabs found in the spreadsheet. Expected sheets with names like 'Block 1', 'Block 2', etc.");
        }
        
        info!("Discovered {} block tabs", discovered_blocks.len());
        
        // For each discovered block, detect its optimal range dynamically
        let mut optimized_ranges = Vec::new();
        for block in discovered_blocks.iter() {
            match detect_block_extent(&hub, &cfg.sheet_id, &block.name).await {
                Ok(optimized_range) => {
                    info!("Block {}: Using optimized range {}", block.name, optimized_range);
                    optimized_ranges.push(optimized_range);
                }
                Err(e) => {
                    warn!("Failed to detect extent for {}, using fallback template: {}", block.name, e);
                    let fallback_range = cfg.block_range_template.replace("{}", &block.block_number.to_string());
                    info!("Block {}: Using fallback range {}", block.name, fallback_range);
                    optimized_ranges.push(fallback_range);
                }
            }
        }
        optimized_ranges
    };
    
    info!("Processing {} range(s)", ranges.len());
    
    let mut total_new_rows = 0;
    let mut all_normalized_rows = Vec::new();
    
    // Process each range (block or single legacy range)
    for (range_index, range) in ranges.iter().enumerate() {
        info!("Processing range {}/{}: {}", range_index + 1, ranges.len(), range);
        
        // Get the starting row for this specific range/block
        let start_row = if ranges.len() == 1 {
            // Legacy mode: use global state
            state.last_processed_row + 1
        } else {
            // Block mode: use per-block state
            state.get_next_row_for_block(range)
        };
        
        info!("Starting from row {} for range: {}", start_row, range);
        
        // Fetch rows from this specific range
        let raw_rows = fetch_rows(&hub, &cfg.sheet_id, range, start_row).await?;
        
        if raw_rows.is_empty() {
            info!("No new rows found in range: {}", range);
            continue;
        }
        
        info!("Found {} new rows in range: {}", raw_rows.len(), range);
        
        // Extract block name from range (e.g., "Block 1!A2:Z" -> "Block 1")
        let block_name = range.split('!').next().unwrap_or(range);
        
        // Use the new block-aware processing
        let range_normalized_rows = match normalize_block_data(raw_rows.clone(), block_name) {
            Ok(records) => {
                info!("Successfully parsed {} workout records from {}", records.len(), block_name);
                records
            }
            Err(e) => {
                warn!("Failed to parse block data for {}: {}", block_name, e);
                // Fallback to empty vec
                Vec::new()
            }
        };
        
        let _range_processed_count = range_normalized_rows.len();
        
        // Add to global collection
        all_normalized_rows.extend(range_normalized_rows);
        total_new_rows += raw_rows.len();
        
        // Update state for this range
        if ranges.len() == 1 {
            // Legacy mode: update global state
            state.update_processed(raw_rows.len());
        } else {
            // Block mode: update per-block state
            state.update_block_state(range, raw_rows.len());
        }
        
        info!("Completed processing range: {} ({} rows)", range, raw_rows.len());
    }
    
    // Write all normalized rows to CSV
    if !all_normalized_rows.is_empty() {
        append(&cfg.output_csv.path, &all_normalized_rows, cfg.output_csv.ensure)?;
        info!("Appended {} normalized rows to CSV from all ranges", all_normalized_rows.len());
    } else {
        info!("No rows were successfully normalized from any range");
    }
    
    // Save updated state
    save_state(&cfg.state_path, &state)?;
    
    // Log completion
    info!("Job completed successfully. Processed {} total rows across {} ranges. Total ever processed: {}", 
          total_new_rows, ranges.len(), state.total_processed);
    
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