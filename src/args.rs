use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "sheet_watch")]
#[command(about = "A CLI tool to read new rows from Google Sheets and append to local CSV")]
#[command(version)]
pub struct Args {
    /// Google Sheets ID
    #[arg(long, value_name = "SHEET_ID")]
    pub sheet_id: Option<String>,
    
    /// Raw range to read from (e.g., "Raw!A2:Z")
    #[arg(long, value_name = "RANGE")]
    pub raw_range: Option<String>,
    
    /// Path to output CSV file
    #[arg(long, value_name = "PATH")]
    pub csv_path: Option<String>,
    
    /// Run once then exit (don't run as scheduler)
    #[arg(long)]
    pub once: bool,
    
    /// Log level
    #[arg(long, default_value = "info")]
    pub log_level: String,
    
    /// Path to config file
    #[arg(long, default_value = "config/config.toml")]
    pub config: String,
} 