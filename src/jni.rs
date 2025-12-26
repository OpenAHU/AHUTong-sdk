use jni::JNIEnv;
use jni::objects::{JClass, JString};
use jni::sys::{jstring, jint, jboolean};
use std::sync::OnceLock;
use tokio::runtime::Runtime;
use crate::data::api::client::AHUClient;
use crate::data::crawler::Crawler;
use crate::data::auth::AuthManager;
use android_logger::Config;
use log::{info, error, LevelFilter};

static CRAWLER: OnceLock<Crawler> = OnceLock::new();
static AUTH_MANAGER: OnceLock<AuthManager> = OnceLock::new();
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn init_logger() {
    android_logger::init_once(
        Config::default()
            .with_max_level(LevelFilter::Debug)
            .with_tag("RustSDK"),
    );
}

fn get_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| Runtime::new().unwrap())
}

fn get_crawler() -> &'static Crawler {
    CRAWLER.get_or_init(|| {
        let client = AHUClient::new();
        Crawler::new(client)
    })
}

fn get_auth_manager() -> &'static AuthManager {
    AUTH_MANAGER.get_or_init(|| {
        let client = get_crawler().client.clone();
        AuthManager::new(client)
    })
}

/// 对应 Java: package com.ahu.ahutong.sdk.RustSDK; public static native void init(String cookiesJson);
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_init(
    mut env: JNIEnv,
    _class: JClass,
    cookies_json: JString,
) {
    init_logger();
    info!("Rust SDK Initialized");

    let json: String = env.get_string(&cookies_json).expect("Couldn't get java string!").into();

    // 初始化并加载 Cookie
    let crawler = get_crawler();
    if !json.is_empty() {
        crawler.client.load_cookies_json(&json);
    }
}

/// 对应 Java: public static native String dumpCookies();
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_dumpCookies(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let cookies = get_crawler().client.dump_cookies_json();
    let output = env.new_string(cookies).expect("Couldn't create java string!");
    output.into_raw()
}

/// 对应 Java: public static native String login(String username, String password);
/// 返回: JSON String (User 对象或错误信息)
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_login(
    mut env: JNIEnv,
    _class: JClass,
    username: JString,
    password: JString,
) -> jstring {
    init_logger();
    let username: String = env.get_string(&username).expect("Invalid username").into();
    let password: String = env.get_string(&password).expect("Invalid password").into();
    
    info!("Starting login for user: {}", username);

    let result = get_runtime().block_on(async {
        get_crawler().login(&username, &password).await
    });

    match result {
        Ok(user) => {
            info!("Login successful for user: {}", user.username);
            let json = serde_json::to_string(&user).unwrap();
            env.new_string(json).unwrap().into_raw()
        },
        Err(e) => {
            // 如果是因为已经登录（例如重定向到首页），也算成功，但不应该抛出错误
            // 这里我们已经在 crawler.rs 中处理了 "Already logged in" 返回 Ok(user)
            // 所以这里的 Err 确实是真正的错误
            error!("Login failed: {:?}", e);
            let err_json = serde_json::json!({ "error": e.to_string() });
            env.new_string(err_json.to_string()).unwrap().into_raw()
        }
    }
}

/// 对应 Java: public static native String getSchedule();
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_getSchedule(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let result = get_runtime().block_on(async {
        get_crawler().get_schedule().await
    });

    match result {
        Ok(courses) => {
            let json = serde_json::to_string(&courses).unwrap();
            env.new_string(json).unwrap().into_raw()
        },
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() });
            env.new_string(err_json.to_string()).unwrap().into_raw()
        }
    }
}

/// 对应 Java: public static native String refreshToken();
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_refreshToken(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let result = get_runtime().block_on(async {
        get_auth_manager().refresh_token().await
    });

    match result {
        Ok(token) => env.new_string(token).unwrap().into_raw(),
        Err(e) => env.new_string(format!("ERROR: {}", e)).unwrap().into_raw()
    }
}

/// 对应 Java: public static native String getCookiesList();
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_getCookiesList(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let json = get_crawler().client.get_cookies_flat_json();
    env.new_string(json).unwrap().into_raw()
}

/// 对应 Java: public static native String getQrcode();
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_getQrcode(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let result = get_runtime().block_on(async {
        get_crawler().get_qrcode().await
    });

    match result {
        Ok(val) => {
            let json = serde_json::to_string(&val).unwrap();
            env.new_string(json).unwrap().into_raw()
        },
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() });
            env.new_string(err_json.to_string()).unwrap().into_raw()
        }
    }
}

/// 对应 Java: public static native String getBalance();
#[unsafe(no_mangle)]
pub extern "system" fn Java_com_ahu_ahutong_sdk_RustSDK_getBalance(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let result = get_runtime().block_on(async {
        get_crawler().get_balance().await
    });

    match result {
        Ok(val) => {
            let json = serde_json::to_string(&val).unwrap();
            env.new_string(json).unwrap().into_raw()
        },
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() });
            env.new_string(err_json.to_string()).unwrap().into_raw()
        }
    }
}