use crate::cata_log;
use crate::services::*;
use rocket::{Build, Rocket};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

pub struct SparkRegistry {
    registered_sparks: HashMap<String, fn() -> Box<dyn Spark>>,
    compatible_sparks: Vec<String>,
    incompatible_sparks: Vec<String>,
}

impl SparkRegistry {
    fn new() -> Self {
        SparkRegistry {
            registered_sparks: HashMap::new(),
            compatible_sparks: Vec::new(),
            incompatible_sparks: Vec::new(),
        }
    }

    fn register(&mut self, name: &str, creator: fn() -> Box<dyn Spark>) {
        if !self.registered_sparks.contains_key(name) {
            self.registered_sparks.insert(name.to_string(), creator);

            let spark = creator();

            if spark.is_available() {
                self.compatible_sparks.push(name.to_string());
            } else {
                self.incompatible_sparks.push(name.to_string());

                if !spark.is_compatible_with_environment() {
                    cata_log!(Warning, format!("Spark '{}' is present but not compatible with current environment", spark.name()));
                }

                if !spark.is_enabled() {
                    cata_log!(Warning, format!("Spark '{}' is disabled in configuration", spark.name()));
                }
            }

            cata_log!(Info, format!("Registered spark: {}", name));
        } else {
            cata_log!(Debug, format!("Spark '{}' already registered, skipping duplicate registration", name));
        }
    }

    fn get_compatible_sparks(&self) -> Vec<String> {
        self.compatible_sparks.clone()
    }

    fn get_creator(&self, name: &str) -> Option<&fn() -> Box<dyn Spark>> {
        self.registered_sparks.get(name)
    }
}

static SPARK_REGISTRY: OnceLock<Mutex<SparkRegistry>> = OnceLock::new();

pub trait Spark: Send + Sync + 'static {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    fn attach_to_rocket(&self, rocket: Rocket<Build>) -> Rocket<Build>;

    fn name(&self) -> &str;

    fn description(&self) -> &str {
        "A Catalyst Spark module"
    }

    fn is_compatible_with_environment(&self) -> bool {
        true
    }

    fn is_enabled(&self) -> bool {
        let name = self.name();
        makeuse::get_spark_config::<bool>(name, "enabled").unwrap_or(true)
    }

    fn is_available(&self) -> bool {
        self.is_compatible_with_environment() && self.is_enabled()
    }
}

pub fn register_spark(name: &str, creator: fn() -> Box<dyn Spark>) {
    let registry = SPARK_REGISTRY.get_or_init(|| Mutex::new(SparkRegistry::new()));
    let mut registry_guard = registry.lock().unwrap();
    registry_guard.register(name, creator);
}

pub fn init_registry() {
    if SPARK_REGISTRY.get().is_none() {
        let _ = SPARK_REGISTRY.get_or_init(|| Mutex::new(SparkRegistry::new()));
        cata_log!(Debug, "Spark registry initialized");
    } else {
        cata_log!(Debug, "Spark registry already initialized");
    }
}

pub fn register_by_name(name: &str) -> bool {
    cata_log!(Debug, format!("Attempting to register spark '{}'", name));

    match name {
        "plznohac" => {
            register_spark(name, plznohac::create_spark);
            true
        }

        "vigil" => {
            register_spark(name, vigil::create_spark);
            true
        }
        _ => {
            cata_log!(Warning, format!("Cannot register unknown spark '{}'", name));
            false
        }
    }
}

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

            if let Some(registry) = SPARK_REGISTRY.get() {
                let registry_guard = registry.lock().unwrap();
                for spark_name in sparks {
                    if let Some(creator) = registry_guard.get_creator(&spark_name) {
                        let spark = creator();
                        println!(
                            "   \x1b[1;38;2;255;255;255m>>\x1b[0m \x1b[38;2;76;11;227m{}\x1b[0m \x1b[38;2;255;255;255m({})\x1b[0m",
                            spark_name,
                            spark.description()
                        );
                    } else {
                        println!("   \x1b[1;38;2;255;255;255m>>\x1b[0m \x1b[38;2;76;11;227m{}\x1b[0m \x1b[38;2;255;255;255m(unknown description)\x1b[0m", spark_name);
                    }
                }
            } else {
                for spark in sparks {
                    println!("   \x1b[1;38;2;255;255;255m>>\x1b[0m \x1b[38;2;76;11;227m{}\x1b[0m \x1b[38;2;255;255;255m(initialized)\x1b[0m", spark);
                }
            }
        }
    }
}

pub fn get_available_sparks() -> Vec<String> {
    match SPARK_REGISTRY.get() {
        Some(registry) => {
            let registry_guard = registry.lock().unwrap();
            let result = registry_guard.get_compatible_sparks();
            cata_log!(Debug, format!("Available sparks from registry: {:?}", result));
            result
        }
        None => {
            cata_log!(Warning, "No sparks registered in registry!");
            Vec::new()
        }
    }
}

pub trait SparkExtension {
    fn sparks<I>(self, spark_names: I) -> Self
    where
        I: IntoIterator,
        I::Item: AsRef<str>;

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

            match registry_guard.get_creator(name) {
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
                    let available_sparks = get_available_sparks();
                    cata_log!(Error, format!("Unknown spark: '{}'. Available sparks: {:?}", name, available_sparks));
                }
            }
        }

        rocket
    }

    fn all_sparks(self) -> Self {
        let available = get_available_sparks();
        cata_log!(Info, format!("Activating all sparks: {}", available.len()));
        cata_log!(Debug, format!("Spark list: {:?}", available));
        self.sparks(available)
    }
}
