use anyhow::Result;
use config::{Config, File};
use serde::{Deserialize, Serialize};
use crate::args::Args;
use tracing::{info, debug};

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Cfg {
    pub sheet_id: String,
    pub raw_range: String,
    pub state_path: String,
    pub output_csv: OutputCsvConfig,
    pub once: bool,
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
                if let Ok(raw_range) = config.get_string("raw_range") {
                    cfg.raw_range = raw_range;
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
            debug!("Overriding raw_range from command line");
            cfg.raw_range = raw_range;
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
        // TODO: Add validation logic for configuration
        if self.sheet_id.is_empty() {
            anyhow::bail!("sheet_id cannot be empty");
        }
        
        if self.raw_range.is_empty() {
            anyhow::bail!("raw_range cannot be empty");
        }
        
        if self.output_csv.path.is_empty() {
            anyhow::bail!("output_csv.path cannot be empty");
        }
        
        info!("Configuration validation passed");
        Ok(())
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            sheet_id: "YOUR_SHEET_ID".to_string(),
            raw_range: "Raw!A2:Z".to_string(),
            state_path: "state.json".to_string(),
            output_csv: OutputCsvConfig {
                path: "normalized/normalized.csv".to_string(),
                ensure: true,
            },
            once: false,
        }
    }
} 