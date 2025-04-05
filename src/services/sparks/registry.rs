use crate::cata_log;
use crate::services::*;
use rocket::{Build, Rocket};
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

static SPARK_REGISTRY: OnceLock<Mutex<HashMap<String, fn() -> Box<dyn Spark>>>> = OnceLock::new();

pub trait Spark: Send + Sync + 'static {
    fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error>>;

    fn attach_to_rocket(&self, rocket: Rocket<Build>) -> Rocket<Build>;

    fn name(&self) -> &str;

    fn description(&self) -> &str {
        "A Catalyst Spark module"
    }
}

pub fn register_spark(name: &str, creator: fn() -> Box<dyn Spark>) {
    let registry = SPARK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
    let mut registry_guard = registry.lock().unwrap();

    if !registry_guard.contains_key(name) {
        registry_guard.insert(name.to_string(), creator);
        cata_log!(Info, format!("Registered spark: {}", name));
    } else {
        cata_log!(Debug, format!("Spark '{}' already registered, skipping duplicate registration", name));
    }
}

pub fn init_registry() {
    if SPARK_REGISTRY.get().is_none() {
        let _ = SPARK_REGISTRY.get_or_init(|| Mutex::new(HashMap::new()));
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
                    if let Some(creator) = registry_guard.get(&spark_name) {
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
            let result = registry_guard.keys().cloned().collect::<Vec<String>>();
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
