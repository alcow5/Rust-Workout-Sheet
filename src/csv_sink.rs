use anyhow::Result;
use csv::Writer;
use std::fs::OpenOptions;
use std::path::{Path, PathBuf};
use tracing::{info, debug};
use crate::transform::WorkoutRecord;

pub fn append(csv_path: &str, rows: &[WorkoutRecord], ensure_directories: bool) -> Result<()> {
    let path = Path::new(csv_path);
    
    info!("Appending {} rows to CSV file: {}", rows.len(), csv_path);
    
    if rows.is_empty() {
        debug!("No rows to append, skipping");
        return Ok(());
    }
    
    // Ensure directory exists if requested
    if ensure_directories {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
            debug!("Created directory: {:?}", parent);
        }
    }
    
    let file_exists = path.exists();
    let needs_header = !file_exists;
    
    // Open file for appending
    let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    
    let mut writer = Writer::from_writer(file);
    
    // Write header if this is a new file
    if needs_header {
        info!("Writing CSV header to new file");
        writer.write_record(&WorkoutRecord::to_csv_headers())?;
    }
    
    // Write all rows
    for row in rows {
        writer.write_record(&row.to_csv_row())?;
    }
    
    writer.flush()?;
    info!("Successfully appended {} rows to {}", rows.len(), csv_path);
    
    Ok(())
}

pub fn validate_csv_path(path: &str) -> Result<PathBuf> {
    let path_buf = PathBuf::from(path);
    
    // TODO: Add validation logic for CSV path
    // - Check if parent directory is writable
    // - Validate file extension
    // - Check disk space if needed
    
    Ok(path_buf)
}

pub fn get_row_count(_csv_path: &str) -> Result<usize> {
    // TODO: Implement function to count existing rows in CSV
    // This can be useful for verification
    todo!("Implement row counting for existing CSV files")
} 