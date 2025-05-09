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
}

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct Settings {
    #[serde(default = "default_environment")]
    pub environment: String,
}

fn default_environment() -> String {
    "prod".to_string()
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
        cata_log!(Warning, format!("Failed to load Catalyst.toml: {}", e));
        AppConfig::default()
    });

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

    let spark_count = registry::get_available_sparks().len();
    cata_log!(Info, format!("Bootstrap complete: {} sparks registered", spark_count));
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
