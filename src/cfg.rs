use anyhow::Result;
use config::{Config, File};
use serde::{Deserialize, Serialize};
use crate::args::Args;
use tracing::{info, debug};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cfg {
    pub sheet_id: String,
    pub block_range_template: String,
    pub state_path: String,
    pub output_csv: OutputCsvConfig,
    pub once: bool,
    
    // Optional: specify particular blocks to process (if None, auto-discover all)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub specific_blocks: Option<Vec<u32>>,
    
    // Legacy support for manual min/max blocks (deprecated in favor of auto-discovery)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_block: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_block: Option<u32>,
    
    // Legacy support for single range (deprecated)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub raw_range: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct OutputCsvConfig {
    pub path: String,
    pub ensure: bool,
}

impl Cfg {
    pub fn load(args: Args) -> Result<Self> {
        info!("Loading configuration from: {}", args.config);
        
        // Start with defaults
        let mut cfg = Cfg::default();
        
        // Try to load from config file if it exists
        if std::path::Path::new(&args.config).exists() {
            let config_builder = Config::builder()
                .add_source(File::with_name(&args.config).required(false));
            
            if let Ok(config) = config_builder.build() {
                // Get individual values, falling back to defaults if they don't exist
                if let Ok(sheet_id) = config.get_string("sheet_id") {
                    if sheet_id != "YOUR_SHEET_ID" {
                        cfg.sheet_id = sheet_id;
                    }
                }
                if let Ok(block_range_template) = config.get_string("block_range_template") {
                    cfg.block_range_template = block_range_template;
                }
                
                // Handle specific_blocks array
                if let Ok(specific_blocks) = config.get_array("specific_blocks") {
                    let blocks: Result<Vec<u32>, _> = specific_blocks
                        .iter()
                        .map(|v| v.clone().into_int().map(|i| i as u32))
                        .collect();
                    if let Ok(blocks) = blocks {
                        cfg.specific_blocks = Some(blocks);
                    }
                }
                
                // Legacy support for min/max blocks
                if let Ok(min_block) = config.get_int("min_block") {
                    cfg.min_block = Some(min_block as u32);
                }
                if let Ok(max_block) = config.get_int("max_block") {
                    cfg.max_block = Some(max_block as u32);
                }
                // Legacy support for single raw_range
                if let Ok(raw_range) = config.get_string("raw_range") {
                    cfg.raw_range = Some(raw_range);
                }
                if let Ok(state_path) = config.get_string("state_path") {
                    cfg.state_path = state_path;
                }
                if let Ok(output_path) = config.get_string("output_csv.path") {
                    cfg.output_csv.path = output_path;
                }
                if let Ok(ensure) = config.get_bool("output_csv.ensure") {
                    cfg.output_csv.ensure = ensure;
                }
                debug!("Loaded configuration from file");
            } else {
                debug!("Could not parse config file, using defaults");
            }
        } else {
            debug!("Config file not found, using defaults");
        }
        
        // Override with command line arguments if provided
        if let Some(sheet_id) = args.sheet_id {
            debug!("Overriding sheet_id from command line");
            cfg.sheet_id = sheet_id;
        }
        
        if let Some(raw_range) = args.raw_range {
            debug!("Overriding to legacy raw_range from command line");
            cfg.raw_range = Some(raw_range);
            // Clear block settings when using legacy mode
            cfg.block_range_template = "".to_string();
            cfg.specific_blocks = None;
            cfg.min_block = None;
            cfg.max_block = None;
        }
        
        if let Some(csv_path) = args.csv_path {
            debug!("Overriding csv_path from command line");
            cfg.output_csv.path = csv_path;
        }
        
        // Set once flag from command line
        cfg.once = args.once;
        
        debug!("Final configuration: {:?}", cfg);
        Ok(cfg)
    }
    
    pub fn validate(&self) -> Result<()> {
        if self.sheet_id.is_empty() || self.sheet_id == "YOUR_SHEET_ID" {
            anyhow::bail!("sheet_id must be set to a valid Google Sheets ID");
        }
        
        // Validate either block mode or legacy raw_range mode
        if let Some(ref raw_range) = self.raw_range {
            if raw_range.is_empty() {
                anyhow::bail!("raw_range cannot be empty when using legacy mode");
            }
            info!("Using legacy single range mode: {}", raw_range);
        } else {
            // Block mode validation
            if self.block_range_template.is_empty() {
                anyhow::bail!("block_range_template cannot be empty");
            }
            
            // Check configuration mode
            if let Some(ref specific_blocks) = self.specific_blocks {
                if specific_blocks.is_empty() {
                    anyhow::bail!("specific_blocks cannot be empty if specified");
                }
                
                // Validate block numbers
                for &block_num in specific_blocks {
                    if block_num == 0 {
                        anyhow::bail!("Block numbers must be >= 1, found: {}", block_num);
                    }
                }
                
                info!("Using specific blocks mode: {:?}", specific_blocks);
            } else if let (Some(min_block), Some(max_block)) = (self.min_block, self.max_block) {
                // Legacy min/max mode
                if min_block > max_block {
                    anyhow::bail!("min_block ({}) cannot be greater than max_block ({})", 
                                 min_block, max_block);
                }
                
                if min_block == 0 || max_block == 0 {
                    anyhow::bail!("Block numbers must be >= 1");
                }
                
                let block_count = max_block - min_block + 1;
                info!("Using legacy min/max block mode: {} blocks from {} to {}", block_count, min_block, max_block);
            } else {
                // Auto-discovery mode
                info!("Using auto-discovery mode: will discover all block tabs from the spreadsheet");
            }
        }
        
        if self.output_csv.path.is_empty() {
            anyhow::bail!("output_csv.path cannot be empty");
        }
        
        info!("Configuration validation passed");
        Ok(())
    }
    
    /// Get block ranges if using legacy min/max mode (deprecated - use auto-discovery instead)
    pub fn get_legacy_block_ranges(&self) -> Option<Vec<String>> {
        if let Some(ref raw_range) = self.raw_range {
            // Legacy mode: return single range
            Some(vec![raw_range.clone()])
        } else if let Some(ref specific_blocks) = self.specific_blocks {
            // Specific blocks mode
            Some(specific_blocks.iter()
                 .map(|&block_num| self.block_range_template.replace("{}", &block_num.to_string()))
                 .collect())
        } else if let (Some(min_block), Some(max_block)) = (self.min_block, self.max_block) {
            // Legacy min/max mode
            Some((min_block..=max_block)
                 .map(|block_num| self.block_range_template.replace("{}", &block_num.to_string()))
                 .collect())
        } else {
            // Auto-discovery mode - ranges will be determined dynamically
            None
        }
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            sheet_id: "YOUR_SHEET_ID".to_string(),
            block_range_template: "Block {}!A1:BZ".to_string(),
            state_path: "state.json".to_string(),
            output_csv: OutputCsvConfig {
                path: "normalized/normalized.csv".to_string(),
                ensure: true,
            },
            once: false,
            specific_blocks: None, // Auto-discover all blocks
            min_block: None,       // Legacy support
            max_block: None,       // Legacy support
            raw_range: None,       // Legacy support
        }
    }
} 