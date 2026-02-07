use jni::{JNIEnv, JavaVM, NativeMethod};
use jni::objects::{JClass, JObject, JString, GlobalRef, JValue};
use jni::sys::{jstring, jint, jlong, jboolean, JNI_VERSION_1_6};
use std::ffi::c_void;
use std::sync::Arc;
use log::{info, error};
use crate::server;

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
        NativeMethod {
            name: "getExamInfo".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_exam_info as *mut c_void,
        },
        NativeMethod {
            name: "getGrade".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_grade as *mut c_void,
        },
        NativeMethod {
            name: "downloadSchoolCalendar".into(),
            sig: "(Ljava/lang/String;)Z".into(),
            fn_ptr: download_school_calendar as *mut c_void,
        },
        NativeMethod {
            name: "getUpdateLog".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_update_log as *mut c_void,
        },
        NativeMethod {
            name: "getVersionName".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_version_name as *mut c_void,
        },
        NativeMethod {
            name: "getUpdateConfigUrl".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_update_config_url as *mut c_void,
        },
        NativeMethod {
            name: "downloadUpdate".into(),
            sig: "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Z".into(),
            fn_ptr: download_update as *mut c_void,
        },
        NativeMethod {
            name: "getApiServerIp".into(),
            sig: "()Ljava/lang/String;".into(),
            fn_ptr: get_api_server_ip as *mut c_void,
        },
        NativeMethod {
            name: "checkApkUpdate".into(),
            sig: "(J)Ljava/lang/String;".into(),
            fn_ptr: check_apk_update as *mut c_void,
        },
        NativeMethod {
            name: "downloadApkUpdate".into(),
            sig: "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)Z".into(),
            fn_ptr: download_apk_update as *mut c_void,
        },
        NativeMethod {
            name: "downloadApkUpdateWithProgress".into(),
            sig: "(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;Lcom/ahu/ahutong/sdk/ProgressCallback;)Z".into(),
            fn_ptr: download_apk_update_with_progress as *mut c_void,
        },
        NativeMethod {
            name: "startServer".into(),
            sig: "(I)Ljava/lang/String;".into(),
            fn_ptr: start_server as *mut c_void,
        },
        NativeMethod {
            name: "stopServer".into(),
            sig: "()V".into(),
            fn_ptr: stop_server as *mut c_void,
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
    crate::core::init_logger();
    info!("Rust SDK Initialized");

    let json: String = env.get_string(&cookies_json)
        .expect("Couldn't get java string!")
        .into();

    crate::core::load_or_clear_cookies(&json);
}

pub extern "system" fn dump_cookies(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::core::init_logger();
    let cookies = crate::core::crawler().client.dump_cookies_json();
    env.new_string(cookies).unwrap().into_raw()
}

pub extern "system" fn login(
    mut env: JNIEnv,
    _class: JClass,
    username: JString,
    password: JString,
) -> jstring {
    crate::core::init_logger();
    let username: String = env.get_string(&username).expect("Invalid username").into();
    let password: String = env.get_string(&password).expect("Invalid password").into();

    let result = crate::core::runtime().block_on(async {
        crate::core::crawler().login(&username, &password).await
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
    let result = crate::core::runtime().block_on(async {
        crate::core::crawler().get_schedule().await
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
    crate::core::init_logger();
    let result = crate::core::runtime().block_on(async {
        get_auth_manager().refresh_token().await
    });

    match result {
        Ok(token) => env.new_string(token).unwrap().into_raw(),
        Err(e) => {
            let err = serde_json::json!({ "error": e.to_string() });
            env.new_string(err.to_string()).unwrap().into_raw()
        }
    }
}

pub extern "system" fn get_cookies_list(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::core::init_logger();
    let json = crate::core::crawler().client.get_cookies_flat_json();
    env.new_string(json).unwrap().into_raw()
}

pub extern "system" fn get_qrcode(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::core::init_logger();
    let result = crate::core::runtime().block_on(async {
        crate::core::crawler().get_qrcode().await
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
    crate::core::init_logger();
    let result = crate::core::runtime().block_on(async {
        crate::core::crawler().get_balance().await
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

pub extern "system" fn get_exam_info(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::core::init_logger();
    let result = crate::core::runtime().block_on(async {
        crate::core::crawler().get_exam_info().await
    });

    match result {
        Ok(val) => {
            let json = serde_json::to_string(&val).unwrap();
            env.new_string(json).expect("Couldn't create java string").into_raw()
        },
        Err(e) => {
            let err_json = serde_json::json!({ "error": e.to_string() });
            env.new_string(err_json.to_string()).expect("Couldn't create java string").into_raw()
        }
    }
}

pub extern "system" fn get_grade(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::core::init_logger();
    let result = crate::core::runtime().block_on(async {
        // 传入 None 自动获取 ID
        crate::core::crawler().get_grade(None).await
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


#[unsafe(no_mangle)]
pub extern "system" fn download_school_calendar(
    mut env: JNIEnv,
    _class: JClass,
    save_path: JString,
) -> jboolean {
    crate::core::init_logger();

    let path: String = match env.get_string(&save_path) {
        Ok(s) => {
            let s_str: String = s.into();
            info!("JNI: Received save path from Java: {}", s_str);
            s_str
        },
        Err(e) => {
            error!("JNI: Failed to convert Java string to Rust string: {:?}", e);
            return 0;
        }
    };

    info!("JNI: Starting async download task...");

    let result = crate::core::runtime().block_on(async {
        let start_time = std::time::Instant::now();

        let task_result = tokio::time::timeout(
            std::time::Duration::from_secs(120),
            crate::core::crawler().download_calendar(&path)
        ).await;

        let duration = start_time.elapsed();
        info!("JNI: Async task finished in {:.2?}", duration);

        task_result
    });

    match result {
        Ok(Ok(_)) => {
            info!("JNI: Download function returned SUCCESS.");
            1
        },
        Ok(Err(e)) => {
            error!("JNI: Download logic failed!");
            error!("--- Error Chain Start ---");
            error!("Root Error: {:?}", e);
            error!("--- Error Chain End ---");
            0
        }
        Err(_) => {
            error!("JNI: CRITICAL - Download task TIMED OUT after 60 seconds!");
            error!("JNI: This usually indicates network blockage or very slow connection.");
            0
        }
    }
}
pub extern "system" fn get_update_log(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    crate::core::init_logger();

    let update_log = r#"
【2026-01-09 v3.0.1 更新内容】
1. 将服务器迁移至带宽更高、稳定性更强的新服务器
2. 全面将 HTTP 升级为 HTTPS，使用ED25519对动态下发的文件进行签名，防止中间人攻击，提升数据传输安全性
3. 更新校历查看逻辑，支持先预览后自主选择是否下载
4. 修改 allowBackup="false"，提高安全性
5. 修改课表教室名称对应 mmap
6. 增加电话本搜索功能
7. 充值改为数字键盘
8. 添加意见反馈功能
9. 增加app内请求更新功能，一次下载，终身使用，再也不用到qq群手动更新了

【2025-01-05 v3.0.0 更新内容】
1. 修复了上个版本遗留的一些 bug：
   - 充值异常问题
   - 考场查询异常
   - 课表显示问题
   （详见 commits 记录）
2. 更新并完善了免责声明说明
3. 完成热更新机制：
   - 使用 Rust 重写核心爬虫相关接口
   - 支持动态下发 .so 文件，实现无需发版的功能更新
"#;

    env
        .new_string(update_log)
        .expect("Couldn't create java string!")
        .into_raw()
}


pub extern "system" fn get_version_name(
    mut env: JNIEnv,
    _this: JObject,
) -> jstring {
    crate::core::init_logger();
    // 暂时不用这个
    let version = "1.0.0 (HotFix)"; 
    env.new_string(version).expect("Couldn't create java string!").into_raw()
}

pub extern "system" fn get_update_config_url(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::core::init_logger();

    let url = "https://openahu.org/api/check_update";
    env.new_string(url)
        .expect("Couldn't create java string!")
        .into_raw()
}

pub extern "system" fn download_update(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    save_path: JString,
    expected_sha256: JString,
    signature: JString,
) -> jboolean {
    crate::core::init_logger();
    let url: String = match env.get_string(&url) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let save_path: String = match env.get_string(&save_path) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let expected_sha256: String = match env.get_string(&expected_sha256) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let signature: String = match env.get_string(&signature) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    info!("Starting update download from {} to {}", url, save_path);

    let result = crate::core::runtime().block_on(async {
        crate::updater::download_and_verify_update(&url, &save_path, &expected_sha256, &signature).await
    });

    match result {
        Ok(_) => {
            info!("Update downloaded and verified successfully");
            1
        },
        Err(e) => {
            error!("Update failed: {:?}", e);
            0
        }
    }
}

pub extern "system" fn get_api_server_ip(
    mut env: JNIEnv,
    _class: JClass,
) -> jstring {
    crate::core::init_logger();
    let ip = "118.25.8.226";
    env.new_string(ip).expect("Couldn't create java string!").into_raw()
}

pub extern "system" fn check_apk_update(
    mut env: JNIEnv,
    _class: JClass,
    current_version_code: jlong,
) -> jstring {
    crate::core::init_logger();

    info!("checkApkUpdate called. current_version_code={}", current_version_code);

    let result = crate::core::runtime().block_on(async {
        crate::updater::check_apk_update(current_version_code as i64).await
    });

    match result {
        Ok(info_obj) => {
            let json = serde_json::to_string(&info_obj).unwrap_or_else(|e| {
                serde_json::json!({ "error": format!("serialize failed: {}", e) }).to_string()
            });
            env.new_string(json).unwrap().into_raw()
        }
        Err(e) => {
            error!("checkApkUpdate failed: {:?}", e);
            let err_json = serde_json::json!({ "error": e.to_string() }).to_string();
            env.new_string(err_json).unwrap().into_raw()
        }
    }
}

pub extern "system" fn download_apk_update(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    save_path: JString,
    expected_sha256: JString,
    signature: JString,
) -> jboolean {
    crate::core::init_logger();

    let url: String = match env.get_string(&url) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let save_path: String = match env.get_string(&save_path) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let expected_sha256: String = match env.get_string(&expected_sha256) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let signature: String = match env.get_string(&signature) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    info!("downloadApkUpdate: {} -> {}", url, save_path);

    let result = crate::core::runtime().block_on(async {
        crate::updater::download_and_verify_update(
            &url,
            &save_path,
            &expected_sha256,
            &signature,
        )
            .await
    });

    match result {
        Ok(_) => {
            info!("downloadApkUpdate success");
            1
        }
        Err(e) => {
            error!("downloadApkUpdate failed: {:?}", e);
            0
        }
    }
}

pub extern "system" fn download_apk_update_with_progress(
    mut env: JNIEnv,
    _class: JClass,
    url: JString,
    save_path: JString,
    expected_sha256: JString,
    signature: JString,
    callback: JObject,
) -> jboolean {
    crate::core::init_logger();

    let url: String = match env.get_string(&url) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let save_path: String = match env.get_string(&save_path) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let expected_sha256: String = match env.get_string(&expected_sha256) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };
    let signature: String = match env.get_string(&signature) {
        Ok(s) => s.into(),
        Err(_) => return 0,
    };

    let cb_global: GlobalRef = match env.new_global_ref(callback) {
        Ok(r) => r,
        Err(e) => {
            error!("downloadApkUpdateWithProgress: new_global_ref failed: {:?}", e);
            return 0;
        }
    };

    let jvm = match env.get_java_vm() {
        Ok(vm) => vm,
        Err(e) => {
            error!("downloadApkUpdateWithProgress: get_java_vm failed: {:?}", e);
            return 0;
        }
    };

    info!(
        "downloadApkUpdateWithProgress: {} -> {}",
        url, save_path
    );

    let cb_global = Arc::new(cb_global);

    let result = crate::core::runtime().block_on(async {
        let cb = cb_global.clone();

        crate::updater::download_and_verify_update_with_progress(
            &url,
            &save_path,
            &expected_sha256,
            &signature,
            move |downloaded: u64, total: i64| {
                let mut env = match jvm.attach_current_thread() {
                    Ok(e) => e,
                    Err(e) => {
                        error!("downloadApkUpdateWithProgress: attach_current_thread failed: {:?}", e);
                        return;
                    }
                };

                // callback.onProgress(downloaded, total)
                let args = &[
                    JValue::Long(downloaded as i64),
                    JValue::Long(total as i64),
                ];

                if let Err(e) = env.call_method(cb.as_obj(), "onProgress", "(JJ)V", args) {
                    error!("downloadApkUpdateWithProgress: call onProgress failed: {:?}", e);
                }
            },
        )
            .await
    });

    match result {
        Ok(_) => {
            info!("downloadApkUpdateWithProgress success");
            1
        }
        Err(e) => {
            error!("downloadApkUpdateWithProgress failed: {:?}", e);
            0
        }
    }
}

pub extern "system" fn start_server(
    mut env: JNIEnv,
    _class: JClass,
    port: jint,
) -> jstring {
    crate::core::init_logger();

    let result = crate::core::runtime().block_on(async {
        server::start(port as u16).await
    });

    match result {
        Ok(info) => {
            let resp = serde_json::json!({
                "port": info.addr.port(),
                "token": info.token,
            });
            env.new_string(resp.to_string()).unwrap().into_raw()
        }
        Err(e) => {
            let err = serde_json::json!({ "error": e.to_string() });
            env.new_string(err.to_string()).unwrap().into_raw()
        }
    }
}

pub extern "system" fn stop_server(
    _env: JNIEnv,
    _class: JClass,
) {
    crate::core::init_logger();
    let _ = crate::core::runtime().block_on(async {
        server::stop().await
    });
}