use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use tracing::{info, debug};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    pub last_processed_row: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub total_processed: usize,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_processed_row: 0,
            last_updated: chrono::Utc::now(),
            total_processed: 0,
        }
    }
}

impl State {
    pub fn new() -> Self {
        Self::default()
    }
    
    pub fn update_processed(&mut self, new_row_count: usize) {
        self.last_processed_row += new_row_count;
        self.total_processed += new_row_count;
        self.last_updated = chrono::Utc::now();
    }
}

pub fn load_state(state_path: &str) -> Result<State> {
    let path = Path::new(state_path);
    
    if !path.exists() {
        info!("State file not found, creating new state: {}", state_path);
        return Ok(State::new());
    }
    
    debug!("Loading state from: {}", state_path);
    let content = fs::read_to_string(path)?;
    let state: State = serde_json::from_str(&content)
        .map_err(|e| anyhow::anyhow!("Failed to parse state file: {}", e))?;
    
    info!("Loaded state: last_processed_row={}, total_processed={}", 
          state.last_processed_row, state.total_processed);
    
    Ok(state)
}

pub fn save_state(state_path: &str, state: &State) -> Result<()> {
    debug!("Saving state to: {}", state_path);
    
    // Ensure directory exists
    if let Some(parent) = Path::new(state_path).parent() {
        fs::create_dir_all(parent)?;
    }
    
    let json = serde_json::to_string_pretty(state)?;
    fs::write(state_path, json)?;
    
    info!("Saved state: last_processed_row={}, total_processed={}", 
          state.last_processed_row, state.total_processed);
    
    Ok(())
}

pub fn backup_state(state_path: &str) -> Result<()> {
    // TODO: Create a backup of the current state file
    // This could be useful for recovery scenarios
    let backup_path = format!("{}.backup", state_path);
    
    if Path::new(state_path).exists() {
        fs::copy(state_path, &backup_path)?;
        debug!("Created state backup: {}", backup_path);
    }
    
    Ok(())
} 