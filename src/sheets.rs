use anyhow::Result;
use google_sheets4::{Sheets, hyper_rustls, hyper, api::ValueRange};
use regex::Regex;
use tracing::{info, debug, warn};

/// Detect the optimal column range for a block by analyzing the week structure
pub async fn detect_block_extent(
    hub: &Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    sheet_id: &str,
    block_name: &str,
) -> Result<String> {
    info!("Detecting optimal column range for block: {}", block_name);
    
    // First, fetch a wide sample of the first few rows to analyze the structure
    let sample_range = format!("{}!A1:ZZ10", block_name);
    debug!("Fetching sample range: {}", sample_range);
    
    let result = hub
        .spreadsheets()
        .values_get(sheet_id, &sample_range)
        .doit()
        .await;
    
    match result {
        Ok((_, value_range)) => {
            let sample_rows = extract_rows_from_response(value_range)?;
            
            if sample_rows.is_empty() {
                anyhow::bail!("No data found in block: {}", block_name);
            }
            
            // Analyze the structure to find the rightmost week
            let max_column = find_rightmost_week_column(&sample_rows)?;
            
            // Convert column number to letter (A=1, B=2, ..., Z=26, AA=27, etc.)
            let end_column = column_number_to_letter(max_column + 5); // Add buffer for notes/data
            let optimized_range = format!("{}!A1:{}", block_name, end_column);
            
            info!("Detected optimal range for {}: {} (covers {} weeks)", 
                  block_name, optimized_range, count_weeks_in_sample(&sample_rows));
            
            Ok(optimized_range)
        }
        Err(e) => {
            warn!("Failed to detect block extent for {}, using fallback range: {}", block_name, e);
            // Fallback to a reasonable default
            Ok(format!("{}!A1:BZ", block_name))
        }
    }
}

pub async fn fetch_rows(
    hub: &Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    sheet_id: &str,
    range: &str,
    start_row: usize,
) -> Result<Vec<Vec<String>>> {
    info!("Fetching rows from sheet {} range {} starting at row {}", sheet_id, range, start_row);
    
    // Construct the actual range with start_row offset
    let adjusted_range = adjust_range_for_start_row(range, start_row)?;
    debug!("Adjusted range: {}", adjusted_range);
    
    // Make the API call to get values
    let result = hub
        .spreadsheets()
        .values_get(sheet_id, &adjusted_range)
        .doit()
        .await;
    
    match result {
        Ok((_, value_range)) => {
            let rows = extract_rows_from_response(value_range)?;
            info!("Successfully fetched {} rows from Google Sheets", rows.len());
            Ok(rows)
        }
        Err(e) => {
            anyhow::bail!("Failed to fetch rows from Google Sheets: {}", e);
        }
    }
}

fn extract_rows_from_response(value_range: ValueRange) -> Result<Vec<Vec<String>>> {
    let mut rows = Vec::new();
    
    if let Some(values) = value_range.values {
        for (row_index, row) in values.iter().enumerate() {
            let mut string_row = Vec::new();
            
            for cell in row {
                // Convert each cell value to string, handling different types
                let cell_string = match cell {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => String::new(),
                    _ => cell.to_string().trim_matches('"').to_string(),
                };
                string_row.push(cell_string);
            }
            
            // Only include non-empty rows
            if !string_row.iter().all(|cell| cell.trim().is_empty()) {
                let column_count = string_row.len();
                rows.push(string_row);
                debug!("Row {}: {} columns", row_index + 1, column_count);
            } else {
                debug!("Skipping empty row {}", row_index + 1);
            }
        }
    } else {
        warn!("No values found in the specified range");
    }
    
    Ok(rows)
}

fn adjust_range_for_start_row(range: &str, start_row: usize) -> Result<String> {
    if start_row == 0 {
        return Ok(range.to_string());
    }
    
    // Parse range like "Raw!A2:Z" and adjust the start row
    // For example, if range is "Raw!A2:Z" and start_row is 5, return "Raw!A7:Z"
    
    if let Some((sheet_part, range_part)) = range.split_once('!') {
        // Split the range part into start and end
        if let Some((start_cell, end_cell)) = range_part.split_once(':') {
            // Extract the starting row number from start_cell (e.g., "A2" -> 2)
            let start_col = start_cell.chars().take_while(|c| c.is_alphabetic()).collect::<String>();
            let start_row_num: usize = start_cell.chars()
                .skip_while(|c| c.is_alphabetic())
                .collect::<String>()
                .parse()
                .unwrap_or(1);
            
            // Calculate the new starting row
            let new_start_row = start_row_num + start_row;
            
            // Construct the new range
            let new_range = format!("{}!{}{}:{}", sheet_part, start_col, new_start_row, end_cell);
            debug!("Adjusted range from '{}' to '{}' (offset: {})", range, new_range, start_row);
            Ok(new_range)
        } else {
            // Single cell reference, just add the offset
            let start_col = range_part.chars().take_while(|c| c.is_alphabetic()).collect::<String>();
            let start_row_num: usize = range_part.chars()
                .skip_while(|c| c.is_alphabetic())
                .collect::<String>()
                .parse()
                .unwrap_or(1);
            
            let new_start_row = start_row_num + start_row;
            let new_range = format!("{}!{}{}", sheet_part, start_col, new_start_row);
            debug!("Adjusted single cell range from '{}' to '{}' (offset: {})", range, new_range, start_row);
            Ok(new_range)
        }
    } else {
        // No sheet name, assume current sheet
        if let Some((start_cell, end_cell)) = range.split_once(':') {
            let start_col = start_cell.chars().take_while(|c| c.is_alphabetic()).collect::<String>();
            let start_row_num: usize = start_cell.chars()
                .skip_while(|c| c.is_alphabetic())
                .collect::<String>()
                .parse()
                .unwrap_or(1);
            
            let new_start_row = start_row_num + start_row;
            let new_range = format!("{}{}:{}", start_col, new_start_row, end_cell);
            debug!("Adjusted range from '{}' to '{}' (offset: {})", range, new_range, start_row);
            Ok(new_range)
        } else {
            anyhow::bail!("Invalid range format: {}", range);
        }
    }
}

pub fn parse_range(range: &str) -> Result<(String, u32, Option<String>)> {
    // Parse sheet range format like "Raw!A2:Z" into (sheet_name, start_row, end_column)
    
    if let Some((sheet_part, range_part)) = range.split_once('!') {
        let sheet_name = sheet_part.to_string();
        
        if let Some((start_cell, end_cell)) = range_part.split_once(':') {
            // Extract the starting row number
            let start_row_str: String = start_cell.chars()
                .skip_while(|c| c.is_alphabetic())
                .collect();
            let start_row: u32 = start_row_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid row number in range: {}", start_cell))?;
            
            // Extract end column (e.g., "Z" from "Z100" or just "Z")
            let end_column = end_cell.chars()
                .take_while(|c| c.is_alphabetic())
                .collect::<String>();
            
            Ok((sheet_name, start_row, Some(end_column)))
        } else {
            // Single cell reference
            let start_row_str: String = range_part.chars()
                .skip_while(|c| c.is_alphabetic())
                .collect();
            let start_row: u32 = start_row_str.parse()
                .map_err(|_| anyhow::anyhow!("Invalid row number in range: {}", range_part))?;
            
            Ok((sheet_name, start_row, None))
        }
    } else {
        anyhow::bail!("Range must include sheet name (e.g., 'Raw!A2:Z'): {}", range);
    }
}

#[derive(Debug, Clone)]
pub struct BlockInfo {
    pub name: String,
    pub block_number: u32,
}

/// Discover all block tabs in the spreadsheet by querying sheet metadata
pub async fn discover_block_tabs(
    hub: &Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>,
    sheet_id: &str,
) -> Result<Vec<BlockInfo>> {
    info!("Discovering block tabs in spreadsheet: {}", sheet_id);
    
    // Get spreadsheet metadata including all sheets
    let result = hub
        .spreadsheets()
        .get(sheet_id)
        .doit()
        .await;
    
    match result {
        Ok((_, spreadsheet)) => {
            let mut blocks = Vec::new();
            
            // Regex to match "Block X" patterns (case insensitive)
            let block_regex = Regex::new(r"(?i)^block\s+(\d+)$")
                .map_err(|e| anyhow::anyhow!("Failed to compile regex: {}", e))?;
            
            if let Some(sheets) = spreadsheet.sheets {
                for sheet in sheets {
                    if let Some(properties) = sheet.properties {
                        if let Some(title) = properties.title {
                            debug!("Found sheet: '{}'", title);
                            
                            // Check if this sheet matches the Block pattern
                            if let Some(captures) = block_regex.captures(&title) {
                                if let Some(number_match) = captures.get(1) {
                                    if let Ok(block_number) = number_match.as_str().parse::<u32>() {
                                        let block_info = BlockInfo {
                                            name: title.clone(),
                                            block_number,
                                        };
                                        blocks.push(block_info);
                                        info!("Discovered block: {} (number: {})", title, block_number);
                                    }
                                }
                            } else {
                                debug!("Sheet '{}' does not match block pattern", title);
                            }
                        }
                    }
                }
            }
            
            // Sort blocks by number for consistent processing order
            blocks.sort_by_key(|b| b.block_number);
            
            info!("Discovered {} block tabs: {:?}", blocks.len(), blocks.iter().map(|b| &b.name).collect::<Vec<_>>());
            Ok(blocks)
        }
        Err(e) => {
            anyhow::bail!("Failed to get spreadsheet metadata: {}", e);
        }
    }
}

/// Find the rightmost column that contains week data (date headers or exercise data)
fn find_rightmost_week_column(sample_rows: &[Vec<String>]) -> Result<usize> {
    let mut max_column = 0;
    
    // Look for date patterns in the first few rows to find week boundaries
    for (_row_idx, row) in sample_rows.iter().take(5).enumerate() {
        for (col_idx, cell) in row.iter().enumerate() {
            let trimmed = cell.trim();
            
            // Check for date headers (like "5/19/2025")
            if is_date_header(trimmed) {
                debug!("Found date header '{}' at column {}", trimmed, col_idx);
                max_column = max_column.max(col_idx);
            }
            
            // Also look for week indicators (like "week 1", "week 2")
            if is_week_header(trimmed) {
                debug!("Found week header '{}' at column {}", trimmed, col_idx);
                max_column = max_column.max(col_idx);
            }
            
            // Look for exercise data columns (sets, reps, load, etc.)
            if is_exercise_data_header(trimmed) {
                debug!("Found exercise data header '{}' at column {}", trimmed, col_idx);
                max_column = max_column.max(col_idx);
            }
        }
    }
    
    // Look for the rightmost non-empty data in exercise rows
    for row in sample_rows.iter().skip(3) { // Skip header rows
        for (col_idx, cell) in row.iter().enumerate() {
            if !cell.trim().is_empty() && has_workout_data(cell.trim()) {
                max_column = max_column.max(col_idx);
            }
        }
    }
    
    if max_column == 0 {
        // Default to a reasonable minimum if no structure detected
        max_column = 25; // Column Z
    }
    
    debug!("Rightmost data column detected: {}", max_column);
    Ok(max_column)
}

/// Check if a cell contains a date header pattern
fn is_date_header(cell: &str) -> bool {
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    
    // Simple date pattern: M/D/YYYY or MM/DD/YYYY
    let parts: Vec<&str> = trimmed.split('/').collect();
    let is_date = parts.len() == 3 && 
        parts[0].parse::<u32>().is_ok() && 
        parts[1].parse::<u32>().is_ok() && 
        parts[2].parse::<u32>().map(|y| y > 2020 && y < 2030).unwrap_or(false);
    
    if is_date {
        debug!("Detected date header: '{}'", trimmed);
    }
    
    is_date
}

/// Check if a cell looks like a week header (e.g., "week 1", "week 2", "deload")
fn is_week_header(cell: &str) -> bool {
    let lower = cell.to_lowercase();
    lower.starts_with("week") || lower == "deload" || 
    lower.contains("week") || lower.contains("phase")
}

/// Check if a cell looks like an exercise data header
fn is_exercise_data_header(cell: &str) -> bool {
    let lower = cell.to_lowercase();
    matches!(lower.as_str(), 
        "sets" | "reps" | "load" | "rpe" | "notes" | "notes:" | 
        "load /" | "exercise" | "day" | "weight" | "duration")
}

/// Check if a cell contains workout data
fn has_workout_data(cell: &str) -> bool {
    // Look for numbers, workout-related text, or RPE values
    if cell.parse::<f64>().is_ok() {
        return true;
    }
    
    let lower = cell.to_lowercase();
    lower.contains("find") || lower.contains("base") || lower.contains("max") ||
    lower.contains("rpe") || lower.contains("kg") || lower.contains("lbs") ||
    lower.contains("-") || lower.contains("/") || lower.contains("x") ||
    lower.contains("easy") || lower.contains("hard")
}

/// Count the number of weeks detected in the sample
fn count_weeks_in_sample(sample_rows: &[Vec<String>]) -> usize {
    let mut week_count = 0;
    
    for row in sample_rows.iter().take(5) {
        for cell in row {
            if is_date_header(cell.trim()) {
                week_count += 1;
            }
        }
    }
    
    week_count
}

/// Convert column number to Excel column letter (A=1, B=2, ..., Z=26, AA=27, etc.)
fn column_number_to_letter(mut col_num: usize) -> String {
    if col_num == 0 {
        return "A".to_string();
    }
    
    let mut result = String::new();
    
    while col_num > 0 {
        col_num -= 1; // Make it 0-based
        let remainder = col_num % 26;
        result = char::from(b'A' + remainder as u8).to_string() + &result;
        col_num /= 26;
    }
    
    result
}

 