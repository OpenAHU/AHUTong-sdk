use std::sync::OnceLock;
use tokio::runtime::Runtime;

use crate::data::api::client::AHUClient;
use crate::data::auth::AuthManager;
use crate::data::crawler::Crawler;

static CRAWLER: OnceLock<Crawler> = OnceLock::new();
static AUTH_MANAGER: OnceLock<AuthManager> = OnceLock::new();
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn init_logger() {
    #[cfg(target_os = "android")]
    {
        use android_logger::Config;
        use log::LevelFilter;
        let max_level = if cfg!(debug_assertions) {
            LevelFilter::Debug
        } else {
            LevelFilter::Off
        };
        android_logger::init_once(
            Config::default()
                .with_max_level(max_level)
                .with_tag("RustSDK"),
        );
    }

    #[cfg(not(target_os = "android"))]
    {
        // 如果你要桌面端调试 main.rs，这里建议加 env_logger（见后面 Cargo.toml）
        if cfg!(debug_assertions) {
            let _ = env_logger::builder().is_test(false).try_init();
        }
    }
}

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
    } else {
        c.client.clear_cookies();
    }
}

pub fn dump_cookies_json() -> String {
    crawler().client.dump_cookies_json()
}

pub fn cookies_flat_json() -> String {
    crawler().client.get_cookies_flat_json()
}
