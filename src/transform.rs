use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use anyhow::Result;
use tracing::debug;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NormalizedRow {
    pub id: String,
    pub timestamp: DateTime<Utc>,
    pub exercise_name: String,
    pub category: String,
    pub sets: Option<u32>,
    pub reps: Option<u32>,
    pub weight: Option<f64>,
    pub weight_unit: String,
    pub duration: Option<u32>, // in seconds
    pub distance: Option<f64>,
    pub distance_unit: String,
    pub notes: Option<String>,
    pub processed_at: DateTime<Utc>,
}

impl NormalizedRow {
    pub fn to_csv_headers() -> Vec<String> {
        vec![
            "id".to_string(),
            "timestamp".to_string(),
            "exercise_name".to_string(),
            "category".to_string(),
            "sets".to_string(),
            "reps".to_string(),
            "weight".to_string(),
            "weight_unit".to_string(),
            "duration_seconds".to_string(),
            "distance".to_string(),
            "distance_unit".to_string(),
            "notes".to_string(),
            "processed_at".to_string(),
        ]
    }
    
    pub fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.timestamp.to_rfc3339(),
            self.exercise_name.clone(),
            self.category.clone(),
            self.sets.map_or(String::new(), |v| v.to_string()),
            self.reps.map_or(String::new(), |v| v.to_string()),
            self.weight.map_or(String::new(), |v| v.to_string()),
            self.weight_unit.clone(),
            self.duration.map_or(String::new(), |v| v.to_string()),
            self.distance.map_or(String::new(), |v| v.to_string()),
            self.distance_unit.clone(),
            self.notes.as_ref().unwrap_or(&String::new()).clone(),
            self.processed_at.to_rfc3339(),
        ]
    }
}

pub fn normalize_row(raw_row: Vec<String>) -> Result<NormalizedRow> {
    debug!("Normalizing row with {} columns: {:?}", raw_row.len(), raw_row);
    
    if raw_row.is_empty() {
        anyhow::bail!("Cannot normalize empty row");
    }
    
    let now = Utc::now();
    
    // Expected column format (adjust based on your actual sheet structure):
    // 0: Date/Time, 1: Exercise Name, 2: Category, 3: Sets, 4: Reps, 5: Weight, 6: Duration, 7: Distance, 8: Notes
    
    let timestamp = parse_timestamp(raw_row.get(0)).unwrap_or(now);
    let exercise_name = raw_row.get(1).unwrap_or(&String::new()).trim().to_string();
    let category = raw_row.get(2).unwrap_or(&"General".to_string()).trim().to_string();
    
    // Parse numeric values with error handling
    let sets = parse_optional_u32(raw_row.get(3));
    let reps = parse_optional_u32(raw_row.get(4));
    let weight = parse_optional_f64(raw_row.get(5));
    let duration = parse_duration(raw_row.get(6)); // Parse duration in various formats
    let distance = parse_optional_f64(raw_row.get(7));
    
    // Determine units based on common patterns or defaults
    let weight_unit = determine_weight_unit(raw_row.get(5)).unwrap_or("lbs".to_string());
    let distance_unit = determine_distance_unit(raw_row.get(7)).unwrap_or("miles".to_string());
    
    let notes = raw_row.get(8).filter(|s| !s.trim().is_empty()).map(|s| s.trim().to_string());
    
    let normalized = NormalizedRow {
        id: format!("row_{}_{}", timestamp.timestamp(), now.timestamp_millis()),
        timestamp,
        exercise_name,
        category,
        sets,
        reps,
        weight,
        weight_unit,
        duration,
        distance,
        distance_unit,
        notes,
        processed_at: now,
    };
    
    debug!("Normalized row: {:?}", normalized);
    Ok(normalized)
}

fn parse_timestamp(timestamp_str: Option<&String>) -> Option<DateTime<Utc>> {
    let s = timestamp_str?.trim();
    
    // Try parsing as RFC3339 first
    if let Ok(dt) = s.parse::<DateTime<Utc>>() {
        return Some(dt);
    }
    
    // Try common date formats
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S") {
        return Some(dt.and_utc());
    }
    
    if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(s, "%m/%d/%Y %H:%M") {
        return Some(dt.and_utc());
    }
    
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        return Some(dt.and_hms_opt(0, 0, 0)?.and_utc());
    }
    
    if let Ok(dt) = chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y") {
        return Some(dt.and_hms_opt(0, 0, 0)?.and_utc());
    }
    
    None
}

fn parse_optional_u32(value_str: Option<&String>) -> Option<u32> {
    value_str
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| clean_numeric_string(s.trim()).parse().ok())
}

fn parse_optional_f64(value_str: Option<&String>) -> Option<f64> {
    value_str
        .filter(|s| !s.trim().is_empty())
        .and_then(|s| clean_numeric_string(s.trim()).parse().ok())
}

fn parse_duration(duration_str: Option<&String>) -> Option<u32> {
    let s = duration_str?.trim();
    if s.is_empty() {
        return None;
    }
    
    // Try to parse duration in various formats
    // "30" (assume seconds), "5:30" (min:sec), "1:05:30" (hour:min:sec), "30s", "5m", "1h"
    
    if let Ok(seconds) = s.parse::<u32>() {
        return Some(seconds);
    }
    
    // Handle "30s", "5m", "1h" format
    if s.ends_with('s') || s.ends_with('S') {
        if let Ok(secs) = s[..s.len()-1].parse::<u32>() {
            return Some(secs);
        }
    }
    if s.ends_with('m') || s.ends_with('M') {
        if let Ok(mins) = s[..s.len()-1].parse::<u32>() {
            return Some(mins * 60);
        }
    }
    if s.ends_with('h') || s.ends_with('H') {
        if let Ok(hours) = s[..s.len()-1].parse::<u32>() {
            return Some(hours * 3600);
        }
    }
    
    // Handle "MM:SS" or "HH:MM:SS" format
    let parts: Vec<&str> = s.split(':').collect();
    match parts.len() {
        2 => {
            // MM:SS
            if let (Ok(mins), Ok(secs)) = (parts[0].parse::<u32>(), parts[1].parse::<u32>()) {
                return Some(mins * 60 + secs);
            }
        }
        3 => {
            // HH:MM:SS
            if let (Ok(hours), Ok(mins), Ok(secs)) = (
                parts[0].parse::<u32>(),
                parts[1].parse::<u32>(),
                parts[2].parse::<u32>(),
            ) {
                return Some(hours * 3600 + mins * 60 + secs);
            }
        }
        _ => {}
    }
    
    None
}

fn determine_weight_unit(weight_str: Option<&String>) -> Option<String> {
    let s = weight_str?.to_lowercase();
    if s.contains("kg") || s.contains("kilo") {
        Some("kg".to_string())
    } else if s.contains("lb") || s.contains("pound") {
        Some("lbs".to_string())
    } else {
        None
    }
}

fn determine_distance_unit(distance_str: Option<&String>) -> Option<String> {
    let s = distance_str?.to_lowercase();
    if s.contains("km") || s.contains("kilometer") {
        Some("km".to_string())
    } else if s.contains("mi") || s.contains("mile") {
        Some("miles".to_string())
    } else if s.contains("m") && !s.contains("mi") {
        Some("meters".to_string())
    } else if s.contains("ft") || s.contains("feet") {
        Some("feet".to_string())
    } else {
        None
    }
}

pub fn clean_numeric_string(input: &str) -> String {
    // Remove common non-numeric characters but keep decimal points and negative signs
    use regex::Regex;
    let re = Regex::new(r"[^\d.-]").unwrap();
    re.replace_all(input, "").to_string()
} 