use std::{
    collections::{HashMap, HashSet},
    sync::RwLock,
};

use once_cell::sync::Lazy;

use crate::{cata_log, meltdown::*, structs::auth::users::Users};

static TOKEN_VERSIONS: Lazy<RwLock<HashMap<i32, u32>>> = Lazy::new(|| RwLock::new(HashMap::new()));

static USED_REFRESH_TOKENS: Lazy<RwLock<HashMap<i32, HashSet<String>>>> = Lazy::new(|| RwLock::new(HashMap::new()));

pub async fn initialize_token_registry() -> Result<(), MeltDown> {
    let users = Users::get_all_users().await?;
    let mut registry = TOKEN_VERSIONS.write().unwrap();
    let mut used_tokens = USED_REFRESH_TOKENS.write().unwrap();

    for user in users {
        registry.insert(user.id, 1);
        used_tokens.insert(user.id, HashSet::new());
    }

    cata_log!(Info, format!("Token registry initialized with {} users", registry.len()));
    Ok(())
}

pub fn get_token_version(user_id: i32) -> u32 {
    match TOKEN_VERSIONS.read().unwrap().get(&user_id) {
        Some(version) => *version,
        None => 1,
    }
}

pub fn invalidate_user_tokens(user_id: i32) -> u32 {
    let mut registry = TOKEN_VERSIONS.write().unwrap();
    let current_version = registry.get(&user_id).copied().unwrap_or(1);
    let new_version = current_version + 1;

    registry.insert(user_id, new_version);

    clear_used_refresh_tokens(user_id);

    cata_log!(Info, format!("Invalidated tokens for user {}: version incremented to {}", user_id, new_version));

    new_version
}

pub fn is_token_valid(user_id: i32, token_version: u32) -> bool {
    let current_version = get_token_version(user_id);

    token_version >= current_version
}

pub fn register_user(user_id: i32) {
    let mut registry = TOKEN_VERSIONS.write().unwrap();
    let mut used_tokens = USED_REFRESH_TOKENS.write().unwrap();

    if !registry.contains_key(&user_id) {
        registry.insert(user_id, 1);
        used_tokens.insert(user_id, HashSet::new());
        cata_log!(Debug, format!("Registered new user {} in token registry", user_id));
    }
}

pub fn remove_user(user_id: i32) {
    let mut registry = TOKEN_VERSIONS.write().unwrap();
    let mut used_tokens = USED_REFRESH_TOKENS.write().unwrap();

    if registry.remove(&user_id).is_some() {
        used_tokens.remove(&user_id);
        cata_log!(Debug, format!("Removed user {} from token registry", user_id));
    }
}

pub fn registry_size() -> usize {
    TOKEN_VERSIONS.read().unwrap().len()
}

pub fn mark_refresh_token_used(user_id: i32, token_jti: &str) -> bool {
    let mut used_tokens = USED_REFRESH_TOKENS.write().unwrap();

    if !used_tokens.contains_key(&user_id) {
        used_tokens.insert(user_id, HashSet::new());
    }

    if let Some(user_tokens) = used_tokens.get_mut(&user_id) {
        user_tokens.insert(token_jti.to_string())
    } else {
        false
    }
}

pub fn is_refresh_token_used(user_id: i32, token_jti: &str) -> bool {
    let used_tokens = USED_REFRESH_TOKENS.read().unwrap();

    used_tokens.get(&user_id).map(|tokens| tokens.contains(token_jti)).unwrap_or(false)
}

pub fn clear_used_refresh_tokens(user_id: i32) {
    let mut used_tokens = USED_REFRESH_TOKENS.write().unwrap();

    if let Some(user_tokens) = used_tokens.get_mut(&user_id) {
        let count = user_tokens.len();
        user_tokens.clear();
        cata_log!(Debug, format!("Cleared {} used refresh tokens for user {}", count, user_id));
    }
}

pub fn used_refresh_token_count(user_id: i32) -> usize {
    let used_tokens = USED_REFRESH_TOKENS.read().unwrap();

    used_tokens.get(&user_id).map(|tokens| tokens.len()).unwrap_or(0)
}
