use std::sync::OnceLock;

#[cfg(not(target_arch = "wasm32"))]
use tokio::runtime::Runtime;

use crate::data::api::client::AHUClient;
use crate::data::auth::AuthManager;
use crate::data::crawler::Crawler;
use crate::persistence;

static CRAWLER: OnceLock<Crawler> = OnceLock::new();
static AUTH_MANAGER: OnceLock<AuthManager> = OnceLock::new();

#[cfg(not(target_arch = "wasm32"))]
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn init_logger() {
    #[cfg(target_os = "android")]
    {
        use android_logger::Config;
        use log::LevelFilter;
        android_logger::init_once(
            Config::default()
                .with_max_level(LevelFilter::Info)
                .with_tag("RustSDK"),
        );
    }

    #[cfg(all(not(target_os = "android"), not(target_arch = "wasm32")))]
    {
        // 如果你要桌面端调试 main.rs，这里建议加 env_logger
        if cfg!(debug_assertions) {
            let _ = env_logger::builder().is_test(false).try_init();
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        // Initialize simple logger for Wasm if we add one later, or just do nothing for now
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().expect("create tokio runtime failed"))
}

pub fn crawler() -> &'static Crawler {
    CRAWLER.get_or_init(|| {
        let client = AHUClient::new();
        Crawler::new(client)
    })
}

pub fn auth_manager() -> &'static AuthManager {
    AUTH_MANAGER.get_or_init(|| {
        let client = crawler().client.clone();
        AuthManager::new(client)
    })
}

/// 复用 JNI init 逻辑
pub fn load_or_clear_cookies(cookies_json: &str) {
    let c = crawler();
    if !cookies_json.is_empty() {
        c.client.load_cookies_json(cookies_json);
        if let Err(e) = persistence::save_cookies(cookies_json) {
            log::warn!("Failed to persist provided Rust cookies: {:?}", e);
        }
    } else {
        c.client.clear_cookies();
        if let Err(e) = persistence::clear_cookies() {
            log::warn!("Failed to clear persisted Rust cookies: {:?}", e);
        }
    }
}

pub fn dump_cookies_json() -> String {
    crawler().client.dump_cookies_json()
}

pub fn cookies_flat_json() -> String {
    crawler().client.get_cookies_flat_json()
}

pub fn init_persistence(storage_path: &str, seed_cookies_json: &str) -> anyhow::Result<()> {
    if let Some(cookies) = persistence::init(storage_path, seed_cookies_json)? {
        crawler().client.load_cookies_json(&cookies);
        log::info!("Restored Rust cookies from persistence.");
    }
    Ok(())
}

pub fn persist_current_cookies() {
    let cookies = dump_cookies_json();
    match persistence::save_cookies(&cookies) {
        Ok(true) => log::info!("Persisted Rust cookies."),
        Ok(false) => {}
        Err(e) => log::warn!("Failed to persist Rust cookies: {:?}", e),
    }
}

pub fn kv_put_string(box_name: &str, key: &str, value: &str) -> anyhow::Result<bool> {
    persistence::put_string(box_name, key, value)
}

pub fn kv_get_string(box_name: &str, key: &str) -> anyhow::Result<Option<String>> {
    persistence::get_string(box_name, key)
}

pub fn kv_remove_key(box_name: &str, key: &str) -> anyhow::Result<bool> {
    persistence::remove_key(box_name, key)
}

pub fn kv_clear_box(box_name: &str) -> anyhow::Result<bool> {
    persistence::clear_box(box_name)
}
