use std::{collections::HashMap, fs, path::Path, sync::OnceLock};

use serde::{Deserialize, Serialize};
use toml::Value as TomlValue;

use crate::{cata_log, services::*};

pub static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();

#[derive(Debug, Deserialize, Serialize, Clone, Default)]
pub struct AppConfig {
    pub settings: Settings,
    #[serde(default)]
    pub sparks: HashMap<String, TomlValue>,
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

fn validate_configured_sparks(config: &AppConfig) {
    if config.sparks.is_empty() {
        cata_log!(Debug, "No sparks configured in Catalyst.toml");
        return;
    }

    cata_log!(Info, format!("Validating {} configured spark(s)", config.sparks.len()));

    let mut missing_sparks = Vec::new();
    let mut invalid_sparks = Vec::new();

    for (spark_name, spark_url) in &config.sparks {
        let spark_dir = Path::new("src/services/sparks").join(spark_name);
        let manifest_path = spark_dir.join("manifest.toml");

        if !spark_dir.exists() {
            missing_sparks.push((spark_name.clone(), spark_url.as_str().unwrap_or("unknown").to_string()));
            continue;
        }

        if !manifest_path.exists() {
            invalid_sparks.push(spark_name.clone());
            continue;
        }

        match validate_spark_manifest(&manifest_path, spark_name) {
            Ok(_) => {
                cata_log!(Debug, format!("Spark '{}' validated successfully", spark_name));
            }
            Err(e) => {
                cata_log!(Warning, format!("Spark '{}' validation failed: {}", spark_name, e));
                invalid_sparks.push(spark_name.clone());
            }
        }
    }

    if !missing_sparks.is_empty() {
        cata_log!(Error, "The following sparks are configured but not installed:");
        for (name, url) in &missing_sparks {
            cata_log!(Error, format!("  - {} ({})", name, url));
        }
        cata_log!(Error, "Install missing sparks using: blast spark add <repository_url>");
        std::process::exit(1);
    }

    if !invalid_sparks.is_empty() {
        cata_log!(Error, "The following sparks have invalid manifests:");
        for name in &invalid_sparks {
            cata_log!(Error, format!("  - {}", name));
        }
        cata_log!(Error, "Fix spark manifests or reinstall using: blast spark add <repository_url>");
        std::process::exit(1);
    }

    cata_log!(Info, "All configured sparks validated successfully");
}

fn validate_spark_manifest(manifest_path: &Path, spark_name: &str) -> Result<(), String> {
    let manifest_content = fs::read_to_string(manifest_path).map_err(|e| format!("Failed to read manifest.toml: {}", e))?;

    let manifest: TomlValue = toml::from_str(&manifest_content).map_err(|e| format!("Failed to parse manifest.toml: {}", e))?;

    let spark_section = manifest.get("spark").ok_or("Missing [spark] section in manifest.toml")?.as_table().ok_or("[spark] section must be a table")?;

    let required_fields = ["name", "version", "description", "author"];
    for field in &required_fields {
        if !spark_section.contains_key(*field) {
            return Err(format!("Missing required field '{}' in [spark] section", field));
        }
        if spark_section.get(*field).and_then(|v| v.as_str()).is_none() {
            return Err(format!("Field '{}' must be a string", field));
        }
    }

    let manifest_name = spark_section.get("name").and_then(|v| v.as_str()).unwrap();

    if manifest_name != spark_name {
        return Err(format!("Spark name mismatch: directory name '{}' does not match manifest name '{}'", spark_name, manifest_name));
    }

    Ok(())
}

pub async fn bootstrap() {
    cata_log!(Info, "Starting bootstrap process");

    if let Err(e) = run_custom_bootstrap(BootstrapPhase::PreConfig).await {
        cata_log!(Error, format!("Custom bootstrap PreConfig phase failed: {}", e));
        std::process::exit(1);
    }

    dotenv::dotenv().ok();

    logger::setup_panic_hook();

    cata_log!(Debug, "Loading configuration from Catalyst.toml");
    let config = AppConfig::load_from_file("Catalyst.toml").unwrap_or_else(|e| {
        cata_log!(Error, format!("Failed to load Catalyst.toml: {}", e));
        std::process::exit(1);
    });

    validate_required_env_vars(&config);
    validate_configured_sparks(&config);

    let _ = APP_CONFIG.set(config);

    if let Err(e) = run_custom_bootstrap(BootstrapPhase::PostConfig).await {
        cata_log!(Error, format!("Custom bootstrap PostConfig phase failed: {}", e));
        std::process::exit(1);
    }

    if let Some(config) = APP_CONFIG.get() {
        cata_log!(Info, format!("Environment: {}", config.settings.environment));
    }

    if let Err(e) = run_custom_bootstrap(BootstrapPhase::PreDatabase).await {
        cata_log!(Error, format!("Custom bootstrap PreDatabase phase failed: {}", e));
        std::process::exit(1);
    }

    cata_log!(Debug, "Initializing database connection pool");
    if let Err(e) = crate::database::db::init_connection_pool().await {
        cata_log!(Error, format!("Failed to initialize database connection pool: {}", e));
        panic!("Database initialization failed");
    }

    if let Err(e) = run_custom_bootstrap(BootstrapPhase::PostDatabase).await {
        cata_log!(Error, format!("Custom bootstrap PostDatabase phase failed: {}", e));
        std::process::exit(1);
    }

    if let Err(e) = run_custom_bootstrap(BootstrapPhase::PreSparks).await {
        cata_log!(Error, format!("Custom bootstrap PreSparks phase failed: {}", e));
        std::process::exit(1);
    }

    registry::init_registry();
    makeuse::init_spark_configs();
    makeuse::init_template_registry();

    cata_log!(Info, "Starting spark discovery and registration");
    validate_and_sync_spark_state();
    load_spark_manifests();

    if let Err(e) = run_custom_bootstrap(BootstrapPhase::PostSparks).await {
        cata_log!(Error, format!("Custom bootstrap PostSparks phase failed: {}", e));
        std::process::exit(1);
    }

    cata_log!(Info, "Initializing token version registry");
    if let Err(e) = token_registry::initialize_token_registry().await {
        cata_log!(Error, format!("Failed to initialize token registry: {}", e));
    }

    let spark_count = registry::get_available_sparks().len();
    cata_log!(Info, format!("Bootstrap complete: {} sparks registered", spark_count));

    if let Err(e) = run_custom_bootstrap(BootstrapPhase::PostBootstrap).await {
        cata_log!(Error, format!("Custom bootstrap PostBootstrap phase failed: {}", e));
        std::process::exit(1);
    }
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

fn validate_and_sync_spark_state() {
    cata_log!(Debug, "Validating spark state consistency");

    let configured_sparks = if let Some(config) = APP_CONFIG.get() {
        config.sparks.keys().cloned().collect::<std::collections::HashSet<String>>()
    } else {
        cata_log!(Warning, "Could not access app config for spark state validation");
        return;
    };

    let sparks_dir = Path::new("src/services/sparks");
    if !sparks_dir.exists() {
        if !configured_sparks.is_empty() {
            cata_log!(Error, format!("Sparks configured but sparks directory missing: {:?}", configured_sparks));
            cata_log!(Error, "Run 'blast spark sync' to install missing sparks");
            std::process::exit(1);
        }
        return;
    }

    let mut issues_found = false;
    let mut missing_directories = Vec::new();
    let mut incomplete_sparks = Vec::new();
    let mut unconfigured_directories = Vec::new();
    let mut orphaned_registry_entries = Vec::new();
    let mut orphaned_mod_entries = Vec::new();

    for spark_name in &configured_sparks {
        let spark_dir = sparks_dir.join(spark_name);
        let manifest_path = spark_dir.join("manifest.toml");

        if !spark_dir.exists() || !manifest_path.exists() {
            missing_directories.push(spark_name.clone());
            issues_found = true;
            continue;
        }

        let registry_missing = !check_spark_in_registry(spark_name);
        let mod_missing = !check_spark_in_mod_rs(spark_name);

        if registry_missing || mod_missing {
            incomplete_sparks.push((spark_name.clone(), registry_missing, mod_missing));
            issues_found = true;
        }
    }

    if let Ok(entries) = fs::read_dir(sparks_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if !path.is_dir() || ["registry", "makeuse"].contains(&path.file_name().unwrap_or_default().to_string_lossy().as_ref()) {
                    continue;
                }

                let spark_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                if !configured_sparks.contains(&spark_name) {
                    unconfigured_directories.push(spark_name);
                    issues_found = true;
                }
            }
        }
    }

    if let Ok(registry_sparks) = get_sparks_from_registry() {
        for registry_spark in registry_sparks {
            if !configured_sparks.contains(&registry_spark) {
                orphaned_registry_entries.push(registry_spark);
                issues_found = true;
            }
        }
    }

    if let Ok(mod_sparks) = get_sparks_from_mod_rs() {
        for mod_spark in mod_sparks {
            if !configured_sparks.contains(&mod_spark) {
                orphaned_mod_entries.push(mod_spark);
                issues_found = true;
            }
        }
    }

    if issues_found {
        cata_log!(Warning, "Spark state inconsistencies detected:");

        if !missing_directories.is_empty() {
            cata_log!(Error, format!("Missing spark directories: {:?}", missing_directories));
        }

        if !incomplete_sparks.is_empty() {
            for (spark_name, registry_missing, mod_missing) in &incomplete_sparks {
                cata_log!(
                    Warning,
                    format!(
                        "Incomplete spark '{}': registry={}, mod={}",
                        spark_name,
                        if *registry_missing { "MISSING" } else { "OK" },
                        if *mod_missing { "MISSING" } else { "OK" }
                    )
                );
            }
        }

        if !unconfigured_directories.is_empty() {
            cata_log!(Warning, format!("Unconfigured spark directories: {:?}", unconfigured_directories));
        }

        if !orphaned_registry_entries.is_empty() {
            cata_log!(Warning, format!("Orphaned registry entries: {:?}", orphaned_registry_entries));
        }

        if !orphaned_mod_entries.is_empty() {
            cata_log!(Warning, format!("Orphaned mod.rs entries: {:?}", orphaned_mod_entries));
        }

        cata_log!(Error, "Spark state is inconsistent with Catalyst.toml configuration");
        cata_log!(Error, "Run 'blast spark sync' to fix these issues before starting the server");
        std::process::exit(1);
    } else {
        if configured_sparks.is_empty() {
            cata_log!(Debug, "No sparks configured - spark state is consistent");
        } else {
            cata_log!(Info, format!("All {} configured sparks are properly synchronized", configured_sparks.len()));
        }
    }
}

fn check_spark_in_registry(spark_name: &str) -> bool {
    let registry_path = Path::new("src/services/sparks/registry.rs");

    if !registry_path.exists() {
        return false;
    }

    match fs::read_to_string(&registry_path) {
        Ok(content) => {
            let pattern = format!("\"{spark_name}\" =>");
            content.contains(&pattern)
        }
        Err(_) => false,
    }
}

fn check_spark_in_mod_rs(spark_name: &str) -> bool {
    let mod_rs_path = Path::new("src/services/sparks/mod.rs");

    if !mod_rs_path.exists() {
        return false;
    }

    match fs::read_to_string(&mod_rs_path) {
        Ok(content) => {
            let module_line = format!("pub mod {};", spark_name);
            content.contains(&module_line)
        }
        Err(_) => false,
    }
}

fn get_sparks_from_registry() -> Result<Vec<String>, String> {
    let registry_path = Path::new("src/services/sparks/registry.rs");

    if !registry_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&registry_path).map_err(|e| format!("Failed to read registry.rs: {}", e))?;

    let mut spark_names = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('"') && trimmed.contains("\" =>") {
            if let Some(end_quote) = trimmed[1..].find('"') {
                let spark_name = &trimmed[1..end_quote + 1];
                if spark_name != "_" {
                    spark_names.push(spark_name.to_string());
                }
            }
        }
    }

    Ok(spark_names)
}

fn get_sparks_from_mod_rs() -> Result<Vec<String>, String> {
    let mod_rs_path = Path::new("src/services/sparks/mod.rs");

    if !mod_rs_path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&mod_rs_path).map_err(|e| format!("Failed to read mod.rs: {}", e))?;

    let mut spark_names = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("pub mod ") && trimmed.ends_with(';') {
            let module_name = &trimmed[8..trimmed.len() - 1];

            if !["makeuse", "registry"].contains(&module_name) {
                spark_names.push(module_name.to_string());
            }
        }
    }

    Ok(spark_names)
}

fn load_spark_manifests() {
    let sparks_dir = Path::new("src/services/sparks");

    if !sparks_dir.exists() {
        cata_log!(Warning, "Sparks directory not found");
        return;
    }

    let configured_sparks = if let Some(config) = APP_CONFIG.get() {
        if config.sparks.is_empty() {
            cata_log!(Info, "No sparks configured in Catalyst.toml - skipping spark loading");
            return;
        }
        config.sparks.keys().cloned().collect::<std::collections::HashSet<String>>()
    } else {
        cata_log!(Warning, "Could not access app config for spark loading");
        return;
    };

    let mut discovered_sparks = Vec::new();
    cata_log!(Debug, format!("Loading {} configured spark(s): {:?}", configured_sparks.len(), configured_sparks));

    for spark_name in &configured_sparks {
        let spark_path = sparks_dir.join(spark_name);

        if !spark_path.is_dir() {
            cata_log!(Warning, format!("Configured spark '{}' directory not found at {}", spark_name, spark_path.display()));
            continue;
        }

        let manifest_path = spark_path.join("manifest.toml");
        if manifest_path.exists() {
            match load_manifest(&manifest_path, spark_name) {
                Ok(_) => {
                    cata_log!(Debug, format!("Loaded manifest for configured spark '{}'", spark_name));
                    discovered_sparks.push(spark_name.clone());
                }
                Err(e) => cata_log!(Error, format!("Failed to load manifest for configured spark '{}': {}", spark_name, e)),
            }
        } else {
            cata_log!(Warning, format!("No manifest.toml found for configured spark '{}' at {}", spark_name, manifest_path.display()));
        }
    }

    if let Ok(entries) = fs::read_dir(sparks_dir) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();

                if !path.is_dir() || ["registry", "makeuse"].contains(&path.file_name().unwrap_or_default().to_string_lossy().as_ref()) {
                    continue;
                }

                let spark_name = path.file_name().unwrap_or_default().to_string_lossy().to_string();

                if !configured_sparks.contains(&spark_name) {
                    cata_log!(Debug, format!("Spark '{}' is installed but not configured in Catalyst.toml - skipping", spark_name));
                }
            }
        }
    }

    cata_log!(Info, format!("Registering {} configured sparks: {:?}", discovered_sparks.len(), discovered_sparks));
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
        if let Some(spark_overrides) = config.sparks.get(spark_name) {
            makeuse::register_spark_overrides(spark_name, spark_overrides.clone());
        }
    }

    Ok(())
}
