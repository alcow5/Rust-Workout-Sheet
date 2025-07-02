use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use tracing::{info, debug};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct State {
    // Legacy single range support
    pub last_processed_row: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
    pub total_processed: usize,
    
    // Multi-block support: track last processed row per block
    pub block_states: HashMap<String, BlockState>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BlockState {
    pub last_processed_row: usize,
    pub total_processed: usize,
    pub last_updated: chrono::DateTime<chrono::Utc>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            last_processed_row: 0,
            last_updated: chrono::Utc::now(),
            total_processed: 0,
            block_states: HashMap::new(),
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
    
    pub fn get_block_state(&self, block_range: &str) -> BlockState {
        self.block_states.get(block_range).cloned().unwrap_or_else(|| {
            BlockState {
                last_processed_row: 0,
                total_processed: 0,
                last_updated: chrono::Utc::now(),
            }
        })
    }
    
    pub fn update_block_state(&mut self, block_range: &str, new_row_count: usize) {
        let now = chrono::Utc::now();
        let block_state = self.block_states.entry(block_range.to_string()).or_insert_with(|| {
            BlockState {
                last_processed_row: 0,
                total_processed: 0,
                last_updated: now,
            }
        });
        
        block_state.last_processed_row += new_row_count;
        block_state.total_processed += new_row_count;
        block_state.last_updated = now;
        
        // Also update global counters
        self.total_processed += new_row_count;
        self.last_updated = now;
    }
    
    pub fn get_next_row_for_block(&self, block_range: &str) -> usize {
        self.get_block_state(block_range).last_processed_row + 1
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