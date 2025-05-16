use std::{
    collections::{HashMap, HashSet},
    sync::RwLock,
};

use once_cell::sync::Lazy;

use crate::{cata_log, database::db, meltdown::*, structs::auth::users::Users};

#[derive(Debug, Default)]
struct TenantRegistry {
    token_versions: HashMap<i32, u32>,

    used_refresh_tokens: HashMap<i32, HashSet<String>>,
}

static TENANT_REGISTRIES: Lazy<RwLock<HashMap<String, TenantRegistry>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn initialize_token_registry() -> Result<(), MeltDown> {
    cata_log!(Info, "Initializing token registry for all tenants");

    let mut known_tenants = vec!["postgres".to_string()];

    if let Ok(tenant_env) = std::env::var("TENANT_LIST") {
        for tenant in tenant_env.split(',') {
            let tenant = tenant.trim();
            if !tenant.is_empty() && !known_tenants.contains(&tenant.to_string()) {
                known_tenants.push(tenant.to_string());
            }
        }
    }

    if let Ok(vessel_db) = std::env::var("VESSEL_DATABASE_URL") {
        if let Some(db_name) = vessel_db.split('/').last() {
            let vessel_tenant = db_name.trim();
            if !vessel_tenant.is_empty() && !known_tenants.contains(&vessel_tenant.to_string()) {
                known_tenants.push(vessel_tenant.to_string());
            }
        }
    }

    cata_log!(Info, format!("Discovered {} tenants: {:?}", known_tenants.len(), known_tenants));

    let mut total_users = 0;

    for tenant_name in known_tenants {
        cata_log!(Debug, format!("Initializing token registry for tenant: {}", tenant_name));

        match Users::get_all_users(&tenant_name).await {
            Ok(users) => {
                let user_count = users.len();
                cata_log!(Info, format!("Found {} users for tenant '{}'", user_count, tenant_name));

                {
                    let mut registries = TENANT_REGISTRIES.write().unwrap();

                    let tenant_registry = registries.entry(tenant_name.clone()).or_insert_with(TenantRegistry::default);

                    for user in users {
                        tenant_registry.token_versions.insert(user.id, 1);
                        tenant_registry.used_refresh_tokens.insert(user.id, HashSet::new());
                    }

                    let tenant_user_count = tenant_registry.token_versions.len();
                    total_users += tenant_user_count;

                    cata_log!(Info, format!("Initialized token registry for tenant '{}' with {} users", tenant_name, tenant_user_count));
                }
            }
            Err(e) => {
                cata_log!(Warning, format!("Failed to get users for tenant '{}': {}", tenant_name, e));

                let mut registries = TENANT_REGISTRIES.write().unwrap();
                registries.entry(tenant_name.clone()).or_insert_with(TenantRegistry::default);

                cata_log!(Info, format!("Created empty token registry for tenant '{}'", tenant_name));
            }
        }
    }

    cata_log!(
        Info,
        format!("Token registry initialization complete with {} total users across {} tenants", total_users, TENANT_REGISTRIES.read().unwrap().len())
    );
    Ok(())
}

pub fn get_token_version(tenant_name: &str, user_id: i32) -> u32 {
    let registries = TENANT_REGISTRIES.read().unwrap();

    match registries.get(tenant_name) {
        Some(tenant_registry) => match tenant_registry.token_versions.get(&user_id) {
            Some(version) => *version,
            None => 1,
        },
        None => 1,
    }
}

pub fn invalidate_user_tokens(tenant_name: &str, user_id: i32) -> u32 {
    let mut registries = TENANT_REGISTRIES.write().unwrap();

    let tenant_registry = registries.entry(tenant_name.to_string()).or_insert_with(TenantRegistry::default);

    let current_version = tenant_registry.token_versions.get(&user_id).copied().unwrap_or(1);
    let new_version = current_version + 1;

    tenant_registry.token_versions.insert(user_id, new_version);

    if let Some(user_tokens) = tenant_registry.used_refresh_tokens.get_mut(&user_id) {
        let count = user_tokens.len();
        user_tokens.clear();
        cata_log!(Debug, format!("Cleared {} used refresh tokens for user {} in tenant {}", count, user_id, tenant_name));
    }

    cata_log!(Info, format!("Invalidated tokens for user {} in tenant {}: version incremented to {}", user_id, tenant_name, new_version));

    new_version
}

pub fn is_token_valid(tenant_name: &str, user_id: i32, token_version: u32) -> bool {
    let current_version = get_token_version(tenant_name, user_id);
    token_version >= current_version
}

pub fn register_user(tenant_name: &str, user_id: i32) {
    let mut registries = TENANT_REGISTRIES.write().unwrap();

    let tenant_registry = registries.entry(tenant_name.to_string()).or_insert_with(TenantRegistry::default);

    if !tenant_registry.token_versions.contains_key(&user_id) {
        tenant_registry.token_versions.insert(user_id, 1);
        tenant_registry.used_refresh_tokens.insert(user_id, HashSet::new());
        cata_log!(Debug, format!("Registered new user {} in token registry for tenant {}", user_id, tenant_name));
    }
}

pub fn remove_user(tenant_name: &str, user_id: i32) {
    let mut registries = TENANT_REGISTRIES.write().unwrap();

    if let Some(tenant_registry) = registries.get_mut(tenant_name) {
        if tenant_registry.token_versions.remove(&user_id).is_some() {
            tenant_registry.used_refresh_tokens.remove(&user_id);
            cata_log!(Debug, format!("Removed user {} from token registry for tenant {}", user_id, tenant_name));
        }
    }
}

pub fn registry_size() -> usize {
    let registries = TENANT_REGISTRIES.read().unwrap();
    registries.iter().map(|(_, registry)| registry.token_versions.len()).sum()
}

pub fn tenant_registry_size(tenant_name: &str) -> usize {
    let registries = TENANT_REGISTRIES.read().unwrap();
    match registries.get(tenant_name) {
        Some(registry) => registry.token_versions.len(),
        None => 0,
    }
}

pub fn mark_refresh_token_used(tenant_name: &str, user_id: i32, token_jti: &str) -> bool {
    let mut registries = TENANT_REGISTRIES.write().unwrap();

    let tenant_registry = registries.entry(tenant_name.to_string()).or_insert_with(TenantRegistry::default);

    if !tenant_registry.used_refresh_tokens.contains_key(&user_id) {
        tenant_registry.used_refresh_tokens.insert(user_id, HashSet::new());
    }

    if let Some(user_tokens) = tenant_registry.used_refresh_tokens.get_mut(&user_id) {
        user_tokens.insert(token_jti.to_string())
    } else {
        false
    }
}

pub fn is_refresh_token_used(tenant_name: &str, user_id: i32, token_jti: &str) -> bool {
    let registries = TENANT_REGISTRIES.read().unwrap();

    match registries.get(tenant_name) {
        Some(tenant_registry) => tenant_registry.used_refresh_tokens.get(&user_id).map(|tokens| tokens.contains(token_jti)).unwrap_or(false),
        None => false,
    }
}

pub fn clear_used_refresh_tokens(tenant_name: &str, user_id: i32) {
    let mut registries = TENANT_REGISTRIES.write().unwrap();

    if let Some(tenant_registry) = registries.get_mut(tenant_name) {
        if let Some(user_tokens) = tenant_registry.used_refresh_tokens.get_mut(&user_id) {
            let count = user_tokens.len();
            user_tokens.clear();
            cata_log!(Debug, format!("Cleared {} used refresh tokens for user {} in tenant {}", count, user_id, tenant_name));
        }
    }
}

pub fn used_refresh_token_count(tenant_name: &str, user_id: i32) -> usize {
    let registries = TENANT_REGISTRIES.read().unwrap();

    match registries.get(tenant_name) {
        Some(tenant_registry) => tenant_registry.used_refresh_tokens.get(&user_id).map(|tokens| tokens.len()).unwrap_or(0),
        None => 0,
    }
}

pub fn get_registered_tenants() -> Vec<String> {
    let registries = TENANT_REGISTRIES.read().unwrap();
    registries.keys().cloned().collect()
}

pub async fn register_tenant(tenant_name: &str) -> Result<(), MeltDown> {
    let users = Users::get_all_users(tenant_name).await?;

    let mut registries = TENANT_REGISTRIES.write().unwrap();

    let tenant_registry = registries.entry(tenant_name.to_string()).or_insert_with(TenantRegistry::default);

    for user in users {
        tenant_registry.token_versions.insert(user.id, 1);
        tenant_registry.used_refresh_tokens.insert(user.id, HashSet::new());
    }

    cata_log!(Info, format!("Registered new tenant '{}' with {} users", tenant_name, tenant_registry.token_versions.len()));
    Ok(())
}
