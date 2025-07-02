use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NormalizedRow {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub category: String,
    pub value: f64,
    pub unit: String,
    pub notes: Option<String>,
    pub processed_at: DateTime<Utc>,
}

impl NormalizedRow {
    pub fn to_csv_headers() -> Vec<String> {
        vec![
            "id".to_string(),
            "timestamp".to_string(),
            "category".to_string(),
            "value".to_string(),
            "unit".to_string(),
            "notes".to_string(),
            "processed_at".to_string(),
        ]
    }
    
    pub fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.timestamp.to_rfc3339(),
            self.category.clone(),
            self.value.to_string(),
            self.unit.clone(),
            self.notes.as_ref().unwrap_or(&String::new()).clone(),
            self.processed_at.to_rfc3339(),
        ]
    }
}

pub fn normalize_row(raw_row: Vec<String>) -> Result<NormalizedRow> {
    debug!("Normalizing row with {} columns", raw_row.len());
    
    // TODO: Implement actual normalization logic based on your specific data format
    // This is a placeholder implementation
    if raw_row.is_empty() {
        anyhow::bail!("Cannot normalize empty row");
    }
    
    let now = Utc::now();
    
    // TODO: Parse actual columns from raw_row based on your sheet structure
    let normalized = NormalizedRow {
        id: format!("row_{}", now.timestamp_millis()),
        timestamp: now, // TODO: Parse from actual timestamp column
        category: raw_row.get(0).unwrap_or(&String::new()).clone(),
        value: parse_value(raw_row.get(1))?,
        unit: raw_row.get(2).unwrap_or(&"unknown".to_string()).clone(),
        notes: raw_row.get(3).map(|s| s.clone()),
        processed_at: now,
    };
    
    debug!("Normalized row: {:?}", normalized);
    Ok(normalized)
}

fn parse_value(value_str: Option<&String>) -> Result<f64> {
    match value_str {
        Some(s) if !s.is_empty() => {
            // TODO: Implement proper value parsing with regex cleaning
            s.trim().parse::<f64>()
                .map_err(|e| anyhow::anyhow!("Failed to parse value '{}': {}", s, e))
        }
        _ => Ok(0.0),
    }
}

pub fn clean_numeric_string(input: &str) -> String {
    // TODO: Use regex to clean numeric strings (remove currency symbols, etc.)
    input.trim().to_string()
} 