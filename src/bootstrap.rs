use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

use serde::{Deserialize, Serialize};
use toml::Value as TomlValue;

use crate::{cata_log, services::*};

pub static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AppConfig {
    pub settings: Settings,
    #[serde(default)]
    pub spark: HashMap<String, TomlValue>,
    #[serde(default)]
    pub required_env: RequiredEnv,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct RequiredEnv {
    #[serde(default)]
    pub variables: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Settings {
    #[serde(default = "default_environment")]
    pub environment: String,

    #[serde(default)]
    pub jwt: JwtSettings,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct JwtSettings {
    #[serde(default = "default_access_token_expiry_mins")]
    pub access_token_expiry_mins: u64,

    #[serde(default = "default_refresh_token_expiry_days")]
    pub refresh_token_expiry_days: u64,

    #[serde(default = "default_refresh_token_expiry_days_remember")]
    pub refresh_token_expiry_days_remember: u64,

    #[serde(default = "default_token_expiry_hours")]
    pub token_expiry_hours: u64,

    #[serde(default = "default_token_expiry_days_remember")]
    pub token_expiry_days_remember: u64,

    #[serde(default = "default_token_refresh_threshold_mins")]
    pub token_refresh_threshold_mins: u64,

    #[serde(default = "default_token_leeway_secs")]
    pub token_leeway_secs: u64,
}

impl Default for JwtSettings {
    fn default() -> Self {
        JwtSettings {
            access_token_expiry_mins: default_access_token_expiry_mins(),

            refresh_token_expiry_days: default_refresh_token_expiry_days(),
            refresh_token_expiry_days_remember: default_refresh_token_expiry_days_remember(),

            token_expiry_hours: default_token_expiry_hours(),
            token_expiry_days_remember: default_token_expiry_days_remember(),
            token_refresh_threshold_mins: default_token_refresh_threshold_mins(),

            token_leeway_secs: default_token_leeway_secs(),
        }
    }
}

fn default_environment() -> String {
    "prod".to_string()
}

fn default_access_token_expiry_mins() -> u64 {
    30
}

fn default_refresh_token_expiry_days() -> u64 {
    7
}

fn default_refresh_token_expiry_days_remember() -> u64 {
    30
}

fn default_token_expiry_hours() -> u64 {
    10
}

fn default_token_expiry_days_remember() -> u64 {
    7
}

fn default_token_refresh_threshold_mins() -> u64 {
    60
}

fn default_token_leeway_secs() -> u64 {
    5
}

impl AppConfig {
    pub fn load_from_file(path: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let contents = fs::read_to_string(path)?;
        let config: AppConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    pub fn is_development(&self) -> bool {
        self.settings.environment == "dev"
    }
}

pub async fn bootstrap() {
    cata_log!(Info, "Starting bootstrap process");

    dotenv::dotenv().ok();

    logger::setup_panic_hook();

    cata_log!(Debug, "Loading configuration from Catalyst.toml");
    let config = AppConfig::load_from_file("Catalyst.toml").unwrap_or_else(|e| {
        cata_log!(Error, format!("Failed to load Catalyst.toml: {}", e));
        std::process::exit(1);
    });

    validate_required_env_vars(&config);

    let _ = APP_CONFIG.set(config);

    if let Some(config) = APP_CONFIG.get() {
        cata_log!(Info, format!("Environment: {}", config.settings.environment));
    }

    cata_log!(Debug, "Initializing database connection pool");
    if let Err(e) = crate::database::db::init_connection_pool().await {
        cata_log!(Error, format!("Failed to initialize database connection pool: {}", e));
        panic!("Database initialization failed");
    }

    registry::init_registry();
    makeuse::init_spark_configs();
    makeuse::init_template_registry();

    cata_log!(Info, "Starting spark discovery and registration");
    load_spark_manifests();

    cata_log!(Info, "Initializing token version registry");
    if let Err(e) = token_registry::initialize_token_registry().await {
        cata_log!(Error, format!("Failed to initialize token registry: {}", e));
    }

    let spark_count = registry::get_available_sparks().len();
    cata_log!(Info, format!("Bootstrap complete: {} sparks registered", spark_count));
}

fn validate_required_env_vars(config: &AppConfig) {
    let mut invalid_vars = Vec::new();
    
    for var in &config.required_env.variables {
        match std::env::var(var) {
            Ok(value) if value.trim().is_empty() => invalid_vars.push(var.clone()),
            Err(_) => invalid_vars.push(var.clone()),
            _ => {}
        }
    }
    
    if !invalid_vars.is_empty() {
        cata_log!(Error, format!("Environment vars {} are missing or empty", invalid_vars.join(", ")));
        std::process::exit(1);
    }
    
    if !config.required_env.variables.is_empty() {
        cata_log!(Info, format!("All {} required environment variables are set", config.required_env.variables.len()));
    }
}

fn load_spark_manifests() {
    let sparks_dir = Path::new("src/services/sparks");

    if !sparks_dir.exists() {
        cata_log!(Warning, "Sparks directory not found");
        return;
    }

    let mut discovered_sparks = Vec::new();
    cata_log!(Debug, "Scanning sparks directory for manifests");

    if let Ok(entries) = fs::read_dir(sparks_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if !path.is_dir() || ["registry", "makeuse"].contains(&path.file_name().unwrap_or_default().to_string_lossy().as_ref()) {
                    continue;
                }

                let spark_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                let manifest_path = path.join("manifest.toml");
                if manifest_path.exists() {
                    match load_manifest(&manifest_path, &spark_name) {
                        Ok(_) => {
                            cata_log!(Debug, format!("Discovered manifest for spark '{}'", spark_name));
                            discovered_sparks.push(spark_name.clone());
                        }
                        Err(e) => cata_log!(Error, format!("Failed to load manifest for spark '{}': {}", spark_name, e)),
                    }
                } else {
                    cata_log!(Warning, format!("No manifest.toml found for spark '{}'", spark_name));
                }
            }
        }
    }

    cata_log!(Debug, format!("Found {} sparks to register: {:?}", discovered_sparks.len(), discovered_sparks));
    for spark_name in discovered_sparks {
        register_spark_dynamically(&spark_name);
    }
}

fn register_spark_dynamically(spark_name: &str) {
    cata_log!(Debug, format!("Registering spark '{}'", spark_name));
    registry::register_by_name(spark_name);
}

fn load_manifest(path: &Path, spark_name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let manifest_str = fs::read_to_string(path)?;
    let manifest: TomlValue = toml::from_str(&manifest_str)?;

    let defaults = manifest.get("config").and_then(|c| c.get("defaults")).cloned().unwrap_or_else(|| TomlValue::Table(toml::map::Map::new()));

    makeuse::register_spark_manifest(spark_name, manifest, defaults);

    if let Some(config) = APP_CONFIG.get() {
        if let Some(spark_overrides) = config.spark.get(spark_name) {
            makeuse::register_spark_overrides(spark_name, spark_overrides.clone());
        }
    }

    Ok(())
}
