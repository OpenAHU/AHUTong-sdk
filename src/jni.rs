use jni::{JNIEnv, JavaVM, NativeMethod};
use jni::objects::{JClass, JString};
use jni::sys::{jstring, jint, JNI_VERSION_1_6};
use std::ffi::c_void;
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

#[unsafe(no_mangle)]
pub extern "system" fn JNI_OnLoad(vm: JavaVM, _reserved: *mut c_void) -> jint {
    let mut env = vm.get_env().expect("Cannot get reference to the JNIEnv");

    let class_name = "com/ahu/ahutong/sdk/RustSDK";
    let clazz = env.find_class(class_name).expect("Cannot find RustSDK class");

    let methods = [
        NativeMethod {
            name: "init".into(),
            sig: "(Ljava/lang/String;)V".into(),
            fn_ptr: init as *mut c_void,
        },
        NativeMethod {
            name: "dumpCookies".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: dump_cookies as *mut c_void,
        },
        NativeMethod {
            name: "login".into(),
            sig: "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;".into(),
            fn_ptr: login as *mut c_void,
        },
        NativeMethod {
            name: "getSchedule".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_schedule as *mut c_void,
        },
        NativeMethod {
            name: "refreshToken".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: refresh_token as *mut c_void,
        },
        NativeMethod {
            name: "getCookiesList".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_cookies_list as *mut c_void,
        },
        NativeMethod {
            name: "getQrcode".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_qrcode as *mut c_void,
        },
        NativeMethod {
            name: "getBalance".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_balance as *mut c_void,
        },
    ];

    env.register_native_methods(clazz, &methods).expect("Failed to register native methods");

    JNI_VERSION_1_6
}

pub extern "system" fn init(
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
    } else {
        crawler.client.clear_cookies();
    }
}

pub extern "system" fn dump_cookies(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let cookies = get_crawler().client.dump_cookies_json();
    let output = env.new_string(cookies).expect("Couldn't create java string!");
    output.into_raw()
}

pub extern "system" fn login(
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

pub extern "system" fn get_schedule(
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

pub extern "system" fn refresh_token(
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

pub extern "system" fn get_cookies_list(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    init_logger();
    let json = get_crawler().client.get_cookies_flat_json();
    env.new_string(json).unwrap().into_raw()
}

pub extern "system" fn get_qrcode(
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

pub extern "system" fn get_balance(
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
