use crate::cata_log;
use crate::services::*;
use rocket::{Build, Rocket};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::{Mutex, OnceLock};
use toml;

static SPARK_REGISTRY: OnceLock<Mutex<HashMap<&'static str, fn() -> Box<dyn Spark>>>> = OnceLock::new();
static SPARK_DESCRIPTIONS: OnceLock<Mutex<HashMap<&'static str, &'static str>>> = OnceLock::new();

/// Trait representing a Catalyst spark (module extension)
pub trait Spark: Send + Sync + 'static {
    /// Initialize the spark
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    /// Attach the spark to a Rocket instance
    fn attach_to_rocket(&self, rocket: Rocket<Build>) -> Rocket<Build>;

    /// Name of the spark
    fn name(&self) -> &str;
    
    /// Description of the spark
    fn description(&self) -> &str {
        "No description available"
    }
}

/// Register a spark creator function
pub fn register_spark(name: &'static str, creator: fn() -> Box<dyn Spark>) {
    let registry = SPARK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry_guard = registry.lock().unwrap();
    registry_guard.insert(name, creator);
    cata_log!(Debug, format!("Registered spark: {}", name));
}

include!(concat!(env!("OUT_DIR"), "/spark_registry.rs"));

/// Load spark descriptions from manifest files
pub fn load_spark_descriptions() {
    let descriptions = SPARK_DESCRIPTIONS.get_or_init(|| Mutex::new(HashMap::new()));
    let mut descriptions_guard = descriptions.lock().unwrap();
    
    // Path to sparks directory (relative to the project root)
    let sparks_dir = "../sparks";
    
    for spark_name in AVAILABLE_SPARKS {
        let manifest_path = format!("{}/{}/manifest.toml", sparks_dir, spark_name);
        let path = Path::new(&manifest_path);
        
        if path.exists() {
            match fs::read_to_string(path) {
                Ok(content) => {
                    match toml::from_str::<toml::Value>(&content) {
                        Ok(toml_value) => {
                            if let Some(spark_section) = toml_value.get("spark") {
                                if let Some(description) = spark_section.get("description") {
                                    if let Some(desc_str) = description.as_str() {
                                        // Convert to static string (safe for this use case as descriptions won't change at runtime)
                                        let static_desc: &'static str = Box::leak(desc_str.to_string().into_boxed_str());
                                        descriptions_guard.insert(*spark_name, static_desc);
                                        cata_log!(Debug, format!("Loaded description for spark '{}': {}", spark_name, static_desc));
                                    }
                                }
                            }
                        },
                        Err(e) => cata_log!(Warning, format!("Failed to parse manifest for spark '{}': {}", spark_name, e)),
                    }
                },
                Err(e) => cata_log!(Warning, format!("Failed to read manifest for spark '{}': {}", spark_name, e)),
            }
        } else {
            cata_log!(Warning, format!("Manifest file not found for spark '{}' at '{}'", spark_name, manifest_path));
        }
    }
}

/// Get a spark's description
pub fn get_spark_description(name: &str) -> &'static str {
    if let Some(descriptions) = SPARK_DESCRIPTIONS.get() {
        let descriptions_guard = descriptions.lock().unwrap();
        if let Some(desc) = descriptions_guard.get(name) {
            return *desc;
        }
    }
    "No description available"
}

/// Initialize the registry with all available sparks
pub fn init_registry() {
    if SPARK_REGISTRY.get().is_none() {
        let _ = SPARK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
        register_all_discovered_sparks();
        load_spark_descriptions();
        cata_log!(Info, format!("Registered sparks: {:?}", get_available_sparks()));
    }
}

/// Attaches a fairing that logs loaded sparks during Rocket startup
pub struct SparkLoggingFairing;

#[rocket::async_trait]
impl rocket::fairing::Fairing for SparkLoggingFairing {
    fn info(&self) -> rocket::fairing::Info {
        rocket::fairing::Info {
            name: "Spark Logger",
            kind: rocket::fairing::Kind::Liftoff,
        }
    }

    async fn on_liftoff(&self, _rocket: &rocket::Rocket<rocket::Orbit>) {
        let sparks = get_available_sparks();
        if sparks.is_empty() {
            println!("\x1b[38;2;148;22;127m✨ No Sparks\x1b[34m:\x1b[0m Loaded");
        } else {
            println!("\x1b[38;2;148;22;127m✨ Sparks\x1b[34m:\x1b[0m");
            for spark in sparks {
                let description = get_spark_description(spark);
                println!("   \x1b[1;38;2;255;255;255m>>\x1b[0m \x1b[38;2;76;11;227m{}\x1b[0m \x1b[38;2;255;255;255m{}\x1b[0m", 
                         spark, description);
            }
        }
    }
}

/// Get list of available spark names
pub fn get_available_sparks() -> Vec<&'static str> {
    match SPARK_REGISTRY.get() {
        Some(registry) => {
            let registry_guard = registry.lock().unwrap();
            registry_guard.keys().copied().collect()
        }
        None => Vec::new(),
    }
}

/// Extension trait for Rocket to easily add sparks
pub trait SparkExtension {
    /// Add specific sparks to the Rocket instance
    fn sparks<I>(self, spark_names: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>;

    /// Add all available sparks to the Rocket instance
    fn all_sparks(self) -> Self;
}

impl SparkExtension for Rocket<Build> {
    fn sparks<I>(self, spark_names: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>,
    {
        init_registry();

        let registry = SPARK_REGISTRY.get().unwrap();
        let registry_guard = registry.lock().unwrap();
        let mut rocket = self;

        for name_ref in spark_names.into_iter() {
            let name = name_ref.as_ref();

            match registry_guard.get(name) {
                Some(creator) => {
                    let mut spark = creator();
                    match spark.initialize() {
                        Ok(_) => {
                            cata_log!(Info, format!("Spark '{}' initialized successfully", spark.name()));
                            rocket = spark.attach_to_rocket(rocket);
                        }
                        Err(e) => {
                            cata_log!(Error, format!("Failed to initialize spark '{}': {}", spark.name(), e));
                        }
                    }
                }
                None => {
                    cata_log!(Error, format!("Unknown spark: '{}'. Available sparks: {:?}", name, AVAILABLE_SPARKS));
                }
            }
        }

        rocket
    }

    fn all_sparks(self) -> Self {
        self.sparks(AVAILABLE_SPARKS)
    }
}
