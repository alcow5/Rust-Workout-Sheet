use anyhow::Result;
use google_sheets4::{Sheets, hyper, hyper_rustls};
use std::env;
use std::path::Path;
use tracing::{info, debug};
use yup_oauth2::{ServiceAccountAuthenticator, ServiceAccountKey};

const DEFAULT_SERVICE_ACCOUNT_KEY: &str = "service-account-key.json";

pub async fn create_sheets_hub() -> Result<Sheets<hyper_rustls::HttpsConnector<hyper::client::HttpConnector>>> {
    info!("Initializing Google Sheets authentication");
    
    // Create HTTP client
    let https = hyper_rustls::HttpsConnectorBuilder::new()
        .with_native_roots()?
        .https_or_http()
        .enable_http1()
        .build();
    let client = hyper::Client::builder().build::<_, hyper::Body>(https);
    
    // Get service account key path
    let key_path = get_service_account_key_path()?;
    debug!("Using service account key: {}", key_path);
    
    // Load service account key
    let service_account_key = load_service_account_key(&key_path).await?;
    
    // Create authenticator
    let auth = ServiceAccountAuthenticator::builder(service_account_key)
        .build()
        .await?;
    
    // Create Sheets hub with authentication
    let hub = Sheets::new(client, auth);
    
    info!("Google Sheets authentication initialized successfully");
    Ok(hub)
}

fn get_service_account_key_path() -> Result<String> {
    // First, check if GOOGLE_APPLICATION_CREDENTIALS is set
    if let Ok(path) = env::var("GOOGLE_APPLICATION_CREDENTIALS") {
        if Path::new(&path).exists() {
            return Ok(path);
        }
    }
    
    // Check if the default key file exists in the current directory
    if Path::new(DEFAULT_SERVICE_ACCOUNT_KEY).exists() {
        return Ok(DEFAULT_SERVICE_ACCOUNT_KEY.to_string());
    }
    
    // Check common locations and patterns
    let common_paths = [
        "credentials.json",
        "service-account.json", 
        "gcp-key.json",
        "key.json",
    ];
    
    // Also check for any JSON files that might be service account keys
    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            if let Some(filename) = entry.file_name().to_str() {
                if filename.ends_with(".json") && filename != "package.json" && filename != "config.json" {
                    if Path::new(filename).exists() {
                        return Ok(filename.to_string());
                    }
                }
            }
        }
    }
    
    for path in &common_paths {
        if Path::new(path).exists() {
            return Ok(path.to_string());
        }
    }
    
         anyhow::bail!(
         "Could not find service account key file. Please either:\n\
         1. Set GOOGLE_APPLICATION_CREDENTIALS environment variable to point to your key file\n\
         2. Place your service account JSON key file in the current directory\n\
         3. Use one of these common names: {:?}\n\
         4. Any JSON file in the current directory will be detected automatically",
         common_paths
     );
}

async fn load_service_account_key(key_path: &str) -> Result<ServiceAccountKey> {
    info!("Loading service account key from: {}", key_path);
    
    let key_content = tokio::fs::read_to_string(key_path).await
        .map_err(|e| anyhow::anyhow!("Failed to read service account key file '{}': {}", key_path, e))?;
    
    let service_account_key: ServiceAccountKey = serde_json::from_str(&key_content)
        .map_err(|e| anyhow::anyhow!("Failed to parse service account key file '{}': {}", key_path, e))?;
    
    debug!("Successfully loaded service account key for: {}", 
           service_account_key.client_email);
    
    Ok(service_account_key)
}

pub async fn get_access_token() -> Result<String> {
    // TODO: Implement access token retrieval
    todo!("Implement access token retrieval for service account")
} 