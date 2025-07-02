use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc, NaiveDate, Duration, Datelike};
use anyhow::Result;
use tracing::debug;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkoutRecord {
    pub id: String,
    pub block_name: String,
    pub week_start_date: String,  // e.g., "5/19/2025"
    pub week_number: u32,         // 1, 2, 3, etc.
    pub day_number: u32,          // 1, 2, 3, etc. 
    pub workout_date: String,     // Calculated actual workout date
    pub exercise_name: String,
    pub record_type: String,      // "prescribed" or "actual"
    
    // Workout data
    pub sets: Option<u32>,
    pub reps: Option<String>,     // Can be "7", "8-10", etc.
    pub load: Option<f64>,
    pub load_instruction: Option<String>, // "find", "base on max", etc.
    pub rpe: Option<String>,      // Can be "5", "5, 6", "easy 7", etc.
    pub notes: Option<String>,
    
    // Metadata
    pub processed_at: DateTime<Utc>,
}

impl WorkoutRecord {
    pub fn to_csv_headers() -> Vec<String> {
        vec![
            "id".to_string(),
            "block_name".to_string(),
            "week_start_date".to_string(),
            "week_number".to_string(),
            "day_number".to_string(),
            "workout_date".to_string(),
            "exercise_name".to_string(),
            "record_type".to_string(),
            "sets".to_string(),
            "reps".to_string(),
            "load".to_string(),
            "load_instruction".to_string(),
            "rpe".to_string(),
            "notes".to_string(),
            "processed_at".to_string(),
        ]
    }
    
    pub fn to_csv_row(&self) -> Vec<String> {
        vec![
            self.id.clone(),
            self.block_name.clone(),
            self.week_start_date.clone(),
            self.week_number.to_string(),
            self.day_number.to_string(),
            self.workout_date.clone(),
            self.exercise_name.clone(),
            self.record_type.clone(),
            self.sets.map(|s| s.to_string()).unwrap_or_default(),
            self.reps.clone().unwrap_or_default(),
            self.load.map(|l| l.to_string()).unwrap_or_default(),
            self.load_instruction.clone().unwrap_or_default(),
            self.rpe.clone().unwrap_or_default(),
            self.notes.clone().unwrap_or_default(),
            self.processed_at.to_rfc3339(),
        ]
    }
}

#[derive(Debug, Clone)]
struct WeekInfo {
    week_number: u32,
    start_date: String,
    start_col: usize,
    end_col: usize,
}

#[derive(Debug, Clone)]
struct DayInfo {
    day_number: u32,
    row_index: usize,
}

pub fn normalize_block_data(raw_rows: Vec<Vec<String>>, block_name: &str) -> Result<Vec<WorkoutRecord>> {
    if raw_rows.is_empty() {
        return Ok(Vec::new());
    }
    
    debug!("Processing block: {} with {} rows", block_name, raw_rows.len());
    
    // Step 1: Parse the header structure to identify weeks
    let weeks = parse_week_structure(&raw_rows)?;
    debug!("Found {} weeks in block {}", weeks.len(), block_name);
    
    // Step 2: Identify day rows and exercise rows
    let (day_rows, exercise_rows) = identify_row_types(&raw_rows)?;
    debug!("Found {} day markers and {} exercise rows", day_rows.len(), exercise_rows.len());
    
    // Step 3: Process each exercise for each week and day
    let mut workout_records = Vec::new();
    
    for week in &weeks {
        for day in &day_rows {
            let workout_date = calculate_workout_date(&week.start_date, day.day_number)?;
            
            // Find exercises for this day
            let day_exercises = find_exercises_for_day(&raw_rows, day.row_index, &exercise_rows);
            
            for exercise_row_idx in day_exercises {
                if let Some(exercise_row) = raw_rows.get(exercise_row_idx) {
                    if let Some(exercise_name) = exercise_row.get(1) {
                        if !exercise_name.trim().is_empty() && exercise_name != "Exercise" {
                            // Extract prescribed and actual data for this week
                            let prescribed = extract_prescribed_data(
                                exercise_row, week, block_name, &week.start_date, 
                                week.week_number, day.day_number, &workout_date, exercise_name
                            )?;
                            
                            let actual = extract_actual_data(
                                exercise_row, week, block_name, &week.start_date,
                                week.week_number, day.day_number, &workout_date, exercise_name
                            )?;
                            
                            if let Some(p) = prescribed {
                                workout_records.push(p);
                            }
                            if let Some(a) = actual {
                                workout_records.push(a);
                            }
                        }
                    }
                }
            }
        }
    }
    
    debug!("Generated {} workout records for block {}", workout_records.len(), block_name);
    Ok(workout_records)
}

fn parse_week_structure(raw_rows: &[Vec<String>]) -> Result<Vec<WeekInfo>> {
    let mut weeks = Vec::new();
    
    // Look for date headers (like "5/19/2025") in the first few rows
    for (row_idx, row) in raw_rows.iter().take(5).enumerate() {
        debug!("Row {} has {} columns: {:?}", row_idx, row.len(), row.iter().take(20).collect::<Vec<_>>());
        for (col_idx, cell) in row.iter().enumerate() {
            if is_date_header(cell) {
                // Look for week number in the row below
                let week_number = if let Some(next_row) = raw_rows.get(row_idx + 1) {
                    parse_week_number(next_row.get(col_idx).unwrap_or(&String::new()))
                } else {
                    weeks.len() as u32 + 1
                };
                
                weeks.push(WeekInfo {
                    week_number,
                    start_date: cell.clone(),
                    start_col: col_idx,
                    end_col: col_idx + 12, // Estimate, will refine
                });
            }
        }
    }
    
    // Refine end columns based on next week start or total columns
    for i in 0..weeks.len() {
        if i + 1 < weeks.len() {
            weeks[i].end_col = weeks[i + 1].start_col - 1;
        } else if let Some(first_row) = raw_rows.first() {
            weeks[i].end_col = first_row.len() - 1;
        }
    }
    
    Ok(weeks)
}

fn is_date_header(cell: &str) -> bool {
    // Check for date patterns like "5/19/2025", "5/26/2025"
    let trimmed = cell.trim();
    if trimmed.is_empty() {
        return false;
    }
    
    // Simple date pattern: M/D/YYYY or MM/DD/YYYY
    let parts: Vec<&str> = trimmed.split('/').collect();
    let is_date = parts.len() == 3 && 
        parts[0].parse::<u32>().is_ok() && 
        parts[1].parse::<u32>().is_ok() && 
        parts[2].parse::<u32>().is_ok();
    
    if is_date {
        debug!("Found date header: '{}'", trimmed);
    }
    
    is_date
}

fn parse_week_number(cell: &str) -> u32 {
    let cell = cell.to_lowercase();
    if cell.contains("week 1") { 1 }
    else if cell.contains("week 2") { 2 }
    else if cell.contains("week 3") { 3 }
    else if cell.contains("week 4") { 4 }
    else if cell.contains("week 5") { 5 }
    else if cell.contains("deload") { 6 }
    else { 1 }
}

fn identify_row_types(raw_rows: &[Vec<String>]) -> Result<(Vec<DayInfo>, Vec<usize>)> {
    let mut day_rows = Vec::new();
    let mut exercise_rows = Vec::new();
    
    for (row_idx, row) in raw_rows.iter().enumerate() {
        if let Some(first_cell) = row.get(1) { // Column B (index 1)
            let cell = first_cell.trim().to_uppercase();
            
            // Check for day markers
            if cell.starts_with("DAY ") {
                if let Some(day_num_str) = cell.strip_prefix("DAY ") {
                    let day_num_str = day_num_str.split_whitespace().next().unwrap_or("1");
                    if let Ok(day_num) = day_num_str.parse::<u32>() {
                        day_rows.push(DayInfo {
                            day_number: day_num,
                            row_index: row_idx,
                        });
                    }
                }
            }
            // Check for exercise rows (not empty, not "Exercise", not day markers)
            else if !cell.is_empty() && 
                    cell != "EXERCISE" && 
                    !cell.starts_with("DAY ") &&
                    !cell.starts_with("WEEK ") &&
                    !cell.contains("RPE") &&
                    !is_date_header(first_cell) {
                exercise_rows.push(row_idx);
            }
        }
    }
    
    Ok((day_rows, exercise_rows))
}

fn find_exercises_for_day(raw_rows: &[Vec<String>], day_row_idx: usize, exercise_rows: &[usize]) -> Vec<usize> {
    // Find exercise rows that come after this day marker but before the next day marker
    let next_day_idx = raw_rows.iter()
        .enumerate()
        .skip(day_row_idx + 1)
        .find(|(_, row)| {
            if let Some(cell) = row.get(1) {
                cell.trim().to_uppercase().starts_with("DAY ")
            } else {
                false
            }
        })
        .map(|(idx, _)| idx)
        .unwrap_or(raw_rows.len());
    
    exercise_rows.iter()
        .filter(|&&row_idx| row_idx > day_row_idx && row_idx < next_day_idx)
        .copied()
        .collect()
}

fn calculate_workout_date(week_start_date: &str, day_number: u32) -> Result<String> {
    // Parse the date string (e.g., "5/19/2025")
    let parts: Vec<&str> = week_start_date.split('/').collect();
    if parts.len() != 3 {
        return Ok(week_start_date.to_string());
    }
    
    let month: u32 = parts[0].parse().unwrap_or(1);
    let day: u32 = parts[1].parse().unwrap_or(1);
    let year: i32 = parts[2].parse().unwrap_or(2025);
    
    if let Some(start_date) = NaiveDate::from_ymd_opt(year, month, day) {
        // Add days based on workout day (Day 1 = Monday = +0, Day 2 = Tuesday = +1, etc.)
        let workout_date = start_date + Duration::days((day_number - 1) as i64);
        Ok(format!("{}/{}/{}", workout_date.month(), workout_date.day(), workout_date.year()))
    } else {
        Ok(week_start_date.to_string())
    }
}

fn extract_prescribed_data(
    row: &[String], week: &WeekInfo, block_name: &str, week_start_date: &str,
    week_number: u32, day_number: u32, workout_date: &str, exercise_name: &str
) -> Result<Option<WorkoutRecord>> {
    
    // Prescribed data columns within this week's range
    let sets_col = week.start_col + 1;
    let reps_col = week.start_col + 2;
    let load_instruction_col = week.start_col + 3;
    let rpe_col = week.start_col + 4;
    
    let sets = row.get(sets_col).and_then(|s| s.trim().parse::<u32>().ok());
    let reps = row.get(reps_col).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let load_instruction = row.get(load_instruction_col).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let rpe = row.get(rpe_col).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    
    // Only create record if we have some meaningful prescribed data
    if sets.is_some() || reps.is_some() || load_instruction.is_some() || rpe.is_some() {
        let id = format!("{}_{}_w{}_d{}_prescribed_{}", 
                        block_name.replace(" ", ""), 
                        exercise_name.replace(" ", "").replace("/", ""), 
                        week_number, day_number, Utc::now().timestamp_millis());
        
        Ok(Some(WorkoutRecord {
            id,
            block_name: block_name.to_string(),
            week_start_date: week_start_date.to_string(),
            week_number,
            day_number,
            workout_date: workout_date.to_string(),
            exercise_name: exercise_name.to_string(),
            record_type: "prescribed".to_string(),
            sets,
            reps,
            load: None,
            load_instruction,
            rpe,
            notes: None,
            processed_at: Utc::now(),
        }))
    } else {
        Ok(None)
    }
}

fn extract_actual_data(
    row: &[String], week: &WeekInfo, block_name: &str, week_start_date: &str,
    week_number: u32, day_number: u32, workout_date: &str, exercise_name: &str
) -> Result<Option<WorkoutRecord>> {
    
    // Actual data columns within this week's range
    let load_col = week.start_col + 6;
    let sets_col = week.start_col + 7;
    let reps_col = week.start_col + 8;
    let rpe_col = week.start_col + 9;
    let notes_col = week.start_col + 10;
    
    let load = row.get(load_col).and_then(|s| s.trim().parse::<f64>().ok());
    let sets = row.get(sets_col).and_then(|s| s.trim().parse::<u32>().ok());
    let reps = row.get(reps_col).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let rpe = row.get(rpe_col).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let notes = row.get(notes_col).map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    
    // Only create record if we have some meaningful actual data
    if load.is_some() || sets.is_some() || reps.is_some() || rpe.is_some() || notes.is_some() {
        let id = format!("{}_{}_w{}_d{}_actual_{}", 
                        block_name.replace(" ", ""), 
                        exercise_name.replace(" ", "").replace("/", ""), 
                        week_number, day_number, Utc::now().timestamp_millis());
        
        Ok(Some(WorkoutRecord {
            id,
            block_name: block_name.to_string(),
            week_start_date: week_start_date.to_string(),
            week_number,
            day_number,
            workout_date: workout_date.to_string(),
            exercise_name: exercise_name.to_string(),
            record_type: "actual".to_string(),
            sets,
            reps,
            load,
            load_instruction: None,
            rpe,
            notes,
            processed_at: Utc::now(),
        }))
    } else {
        Ok(None)
    }
}

// Legacy function for backwards compatibility
pub fn normalize_row(raw_row: Vec<String>) -> Result<WorkoutRecord> {
    // For now, create a simple record - this will be replaced by the block processor
    let id = format!("legacy_{}", Utc::now().timestamp_millis());
    
    Ok(WorkoutRecord {
        id,
        block_name: "Legacy".to_string(),
        week_start_date: "".to_string(),
        week_number: 1,
        day_number: 1,
        workout_date: "".to_string(),
        exercise_name: raw_row.get(1).cloned().unwrap_or_default(),
        record_type: "legacy".to_string(),
        sets: None,
        reps: None,
        load: None,
        load_instruction: None,
        rpe: None,
        notes: None,
        processed_at: Utc::now(),
    })
} 