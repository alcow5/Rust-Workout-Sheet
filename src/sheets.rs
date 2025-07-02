use anyhow::Result;
use google_sheets4::{Sheets, hyper_rustls, hyper, api::ValueRange};
use tracing::{info, debug, warn};

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