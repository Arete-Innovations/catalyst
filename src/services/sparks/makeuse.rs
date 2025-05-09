use std::{
    collections::{HashMap, HashSet},
    sync::{Mutex, OnceLock},
};

use serde::Serialize;
use toml::Value as TomlValue;

use crate::cata_log;

static TEMPLATE_COMPONENTS: OnceLock<Mutex<TemplateComponents>> = OnceLock::new();

static SPARK_CONFIGS: OnceLock<Mutex<HashMap<String, SparkConfig>>> = OnceLock::new();

pub fn init_template_registry() {
    let _ = TEMPLATE_COMPONENTS.get_or_init(|| Mutex::new(TemplateComponents::default()));
    cata_log!(Debug, "Template component registry initialized");
}

pub fn init_spark_configs() {
    let _ = SPARK_CONFIGS.get_or_init(|| Mutex::new(HashMap::new()));
    cata_log!(Debug, "Spark config registry initialized");
}

pub fn register_spark_manifest(spark_name: &str, manifest: TomlValue, defaults: TomlValue) {
    if let Some(configs) = SPARK_CONFIGS.get() {
        if let Ok(mut configs) = configs.lock() {
            let manifest_clone = manifest.clone();
            let defaults_clone = defaults.clone();

            let entry = configs.entry(spark_name.to_string()).or_insert_with(|| SparkConfig {
                manifest: manifest_clone.clone(),
                defaults: defaults_clone.clone(),
                overrides: TomlValue::Table(toml::map::Map::new()),
            });

            entry.manifest = manifest;
            entry.defaults = defaults;

            cata_log!(Debug, format!("Registered manifest for spark '{}'", spark_name));
        }
    }
}

pub fn register_spark_overrides(spark_name: &str, overrides: TomlValue) {
    if let Some(configs) = SPARK_CONFIGS.get() {
        if let Ok(mut configs) = configs.lock() {
            let overrides_clone = overrides.clone();

            let entry = configs.entry(spark_name.to_string()).or_insert_with(|| SparkConfig {
                manifest: TomlValue::Table(toml::map::Map::new()),
                defaults: TomlValue::Table(toml::map::Map::new()),
                overrides: overrides_clone.clone(),
            });

            entry.overrides = overrides;

            cata_log!(Debug, format!("Registered config overrides for spark '{}'", spark_name));
        }
    }
}

pub fn get_spark_config<T: std::str::FromStr>(spark_name: &str, key: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    let env_key = format!("{}_{}", spark_name.to_uppercase(), key.to_uppercase());
    if let Ok(value) = std::env::var(&env_key) {
        if let Ok(parsed) = value.parse::<T>() {
            return Some(parsed);
        }
    }

    if let Some(configs) = SPARK_CONFIGS.get() {
        if let Ok(configs) = configs.lock() {
            if let Some(config) = configs.get(spark_name) {
                if let Some(value) = get_toml_value::<T>(&config.overrides, key) {
                    return Some(value);
                }

                if let Some(value) = get_toml_value::<T>(&config.defaults, key) {
                    return Some(value);
                }
            }
        }
    }

    None
}

fn get_toml_value<T: std::str::FromStr>(value: &TomlValue, key: &str) -> Option<T>
where
    T::Err: std::fmt::Debug,
{
    match value {
        TomlValue::Table(map) => {
            if let Some(value) = map.get(key) {
                match value {
                    TomlValue::String(s) => s.parse::<T>().ok(),
                    TomlValue::Integer(i) => i.to_string().parse::<T>().ok(),
                    TomlValue::Float(f) => f.to_string().parse::<T>().ok(),
                    TomlValue::Boolean(b) => b.to_string().parse::<T>().ok(),
                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

pub fn get_spark_all_config(spark_name: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();

    if let Some(configs) = SPARK_CONFIGS.get() {
        if let Ok(configs) = configs.lock() {
            if let Some(config) = configs.get(spark_name) {
                if let TomlValue::Table(table) = &config.defaults {
                    for (k, v) in table {
                        result.insert(k.clone(), format!("{}", v));
                    }
                }

                if let TomlValue::Table(table) = &config.overrides {
                    for (k, v) in table {
                        result.insert(k.clone(), format!("{}", v));
                    }
                }
            }
        }
    }

    let prefix = format!("{}_", spark_name.to_uppercase());
    for (key, value) in std::env::vars() {
        if key.starts_with(&prefix) {
            let short_key = key[prefix.len()..].to_lowercase();
            result.insert(short_key, value);
        }
    }

    result
}

struct SparkConfig {
    manifest: TomlValue,
    defaults: TomlValue,
    overrides: TomlValue,
}

pub fn register_head_script(spark_name: &str, script: String, dev_only: bool) {
    if let Some(components) = TEMPLATE_COMPONENTS.get() {
        if let Ok(mut components) = components.lock() {
            components.head_scripts.push((spark_name.to_string(), script, dev_only));
            cata_log!(Debug, format!("Registered head script for spark '{}'", spark_name));
        }
    }
}

pub fn register_footer_script(spark_name: &str, script: String, dev_only: bool) {
    if let Some(components) = TEMPLATE_COMPONENTS.get() {
        if let Ok(mut components) = components.lock() {
            components.footer_scripts.push((spark_name.to_string(), script, dev_only));
            cata_log!(Debug, format!("Registered footer script for spark '{}'", spark_name));
        }
    }
}

pub fn register_head_style(spark_name: &str, style: String, dev_only: bool) {
    if let Some(components) = TEMPLATE_COMPONENTS.get() {
        if let Ok(mut components) = components.lock() {
            components.head_styles.push((spark_name.to_string(), style, dev_only));
            cata_log!(Debug, format!("Registered head style for spark '{}'", spark_name));
        }
    }
}

pub fn register_meta_tag(spark_name: &str, meta: String, dev_only: bool) {
    if let Some(components) = TEMPLATE_COMPONENTS.get() {
        if let Ok(mut components) = components.lock() {
            components.meta_tags.push((spark_name.to_string(), meta, dev_only));
            cata_log!(Debug, format!("Registered meta tag for spark '{}'", spark_name));
        }
    }
}

pub fn get_template_components(is_dev: bool) -> TemplateComponentsView {
    if let Some(components) = TEMPLATE_COMPONENTS.get() {
        if let Ok(components) = components.lock() {
            return components.get_view(is_dev);
        }
    }

    cata_log!(Warning, "Failed to get template components");
    TemplateComponentsView::default()
}

#[derive(Default)]
struct TemplateComponents {
    head_scripts: Vec<(String, String, bool)>,
    head_styles: Vec<(String, String, bool)>,
    meta_tags: Vec<(String, String, bool)>,
    footer_scripts: Vec<(String, String, bool)>,
}

impl TemplateComponents {
    fn get_view(&self, is_dev: bool) -> TemplateComponentsView {
        let mut view = TemplateComponentsView::default();
        let mut active_sparks = HashSet::new();

        for (spark, script, dev_only) in &self.head_scripts {
            if !dev_only || is_dev {
                view.head_scripts.push(script.clone());
                active_sparks.insert(spark.clone());
            }
        }

        for (spark, style, dev_only) in &self.head_styles {
            if !dev_only || is_dev {
                view.head_styles.push(style.clone());
                active_sparks.insert(spark.clone());
            }
        }

        for (spark, meta, dev_only) in &self.meta_tags {
            if !dev_only || is_dev {
                view.meta_tags.push(meta.clone());
                active_sparks.insert(spark.clone());
            }
        }

        for (spark, script, dev_only) in &self.footer_scripts {
            if !dev_only || is_dev {
                view.footer_scripts.push(script.clone());
                active_sparks.insert(spark.clone());
            }
        }

        view.active_sparks = active_sparks.into_iter().collect();

        view
    }
}

#[derive(Default, Serialize, Debug)]
pub struct TemplateComponentsView {
    pub head_scripts: Vec<String>,
    pub head_styles: Vec<String>,
    pub meta_tags: Vec<String>,
    pub footer_scripts: Vec<String>,
    pub active_sparks: Vec<String>,
}
