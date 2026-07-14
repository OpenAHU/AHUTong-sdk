use serde::Serialize;
use std::ffi::{CStr, CString, c_char};

fn response_string(
    ok: bool,
    port: Option<u16>,
    token: Option<String>,
    error: Option<String>,
) -> String {
    match (ok, port, token, error) {
        (true, Some(port), Some(token), _) => serde_json::json!({
            "ok": true,
            "port": port,
            "token": token,
        })
        .to_string(),
        (_, _, _, Some(error)) => serde_json::json!({
            "ok": false,
            "error": error,
        })
        .to_string(),
        _ => serde_json::json!({
            "ok": false,
            "error": "invalid FFI response state",
        })
        .to_string(),
    }
}

fn string_into_raw_ptr(s: String) -> *mut c_char {
    match CString::new(s) {
        Ok(c_string) => c_string.into_raw(),
        Err(err) => {
            let sanitized = err
                .into_vec()
                .into_iter()
                .map(|b| if b == 0 { b' ' } else { b })
                .collect::<Vec<_>>();

            match CString::new(sanitized) {
                Ok(c_string) => c_string.into_raw(),
                Err(_) => fallback_error_ptr(),
            }
        }
    }
}

fn fallback_error_ptr() -> *mut c_char {
    // SAFETY: This byte string is a fixed JSON response and contains no interior NUL bytes.
    unsafe {
        CString::from_vec_unchecked(
            br#"{"ok":false,"error":"failed to create FFI response string"}"#.to_vec(),
        )
        .into_raw()
    }
}

fn ffi_result<T: Serialize>(result: anyhow::Result<T>) -> *mut c_char {
    let response = match result {
        Ok(value) => serde_json::json!({
            "ok": true,
            "value": value,
        }),
        Err(error) => serde_json::json!({
            "ok": false,
            "error": error.to_string(),
        }),
    };
    string_into_raw_ptr(response.to_string())
}

fn required_string(pointer: *const c_char, name: &str) -> anyhow::Result<String> {
    if pointer.is_null() {
        anyhow::bail!("{name} pointer is null");
    }
    // SAFETY: FFI callers must pass a valid NUL-terminated string for the
    // duration of this call. The value is copied before the function returns.
    let value = unsafe { CStr::from_ptr(pointer) };
    Ok(value
        .to_str()
        .map_err(|error| anyhow::anyhow!("{name} is not UTF-8: {error}"))?
        .to_string())
}

fn ffi_call<T: Serialize>(operation: impl FnOnce() -> anyhow::Result<T>) -> *mut c_char {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(operation)) {
        Ok(result) => ffi_result(result),
        Err(_) => ffi_result::<serde_json::Value>(Err(anyhow::anyhow!(
            "panic while executing persistence operation"
        ))),
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_init_persistence(
    storage_path: *const c_char,
    seed_cookies_json: *const c_char,
    persist_session: u8,
) -> *mut c_char {
    crate::core::init_logger();
    ffi_call(|| {
        let storage_path = required_string(storage_path, "storage_path")?;
        let seed_cookies_json = required_string(seed_cookies_json, "seed_cookies_json")?;
        crate::core::init_persistence(&storage_path, &seed_cookies_json, persist_session != 0)?;
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_persist_current_cookies() -> *mut c_char {
    crate::core::init_logger();
    ffi_call(crate::core::persist_current_cookies)
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_kv_put_string(
    box_name: *const c_char,
    key: *const c_char,
    value: *const c_char,
) -> *mut c_char {
    crate::core::init_logger();
    ffi_call(|| {
        let box_name = required_string(box_name, "box_name")?;
        let key = required_string(key, "key")?;
        let value = required_string(value, "value")?;
        let initialized = crate::core::kv_put_string(&box_name, &key, &value)?;
        if !initialized {
            anyhow::bail!("persistence is not initialized");
        }
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_kv_get_string(
    box_name: *const c_char,
    key: *const c_char,
) -> *mut c_char {
    crate::core::init_logger();
    ffi_call(|| {
        let box_name = required_string(box_name, "box_name")?;
        let key = required_string(key, "key")?;
        crate::core::kv_get_string(&box_name, &key)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_kv_remove(box_name: *const c_char, key: *const c_char) -> *mut c_char {
    crate::core::init_logger();
    ffi_call(|| {
        let box_name = required_string(box_name, "box_name")?;
        let key = required_string(key, "key")?;
        let initialized = crate::core::kv_remove_key(&box_name, &key)?;
        if !initialized {
            anyhow::bail!("persistence is not initialized");
        }
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_kv_clear_box(box_name: *const c_char) -> *mut c_char {
    crate::core::init_logger();
    ffi_call(|| {
        let box_name = required_string(box_name, "box_name")?;
        let initialized = crate::core::kv_clear_box(&box_name)?;
        if !initialized {
            anyhow::bail!("persistence is not initialized");
        }
        Ok(true)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_start_server(port: u16) -> *mut c_char {
    crate::core::init_logger();

    #[cfg(feature = "server")]
    {
        let result = std::panic::catch_unwind(|| {
            crate::core::runtime().block_on(async { crate::server::start(port).await })
        });

        match result {
            Ok(Ok(info)) => string_into_raw_ptr(response_string(
                true,
                Some(info.addr.port()),
                Some(info.token),
                None,
            )),
            Ok(Err(e)) => {
                string_into_raw_ptr(response_string(false, None, None, Some(e.to_string())))
            }
            Err(_) => string_into_raw_ptr(response_string(
                false,
                None,
                None,
                Some("panic while starting server".to_string()),
            )),
        }
    }

    #[cfg(not(feature = "server"))]
    {
        let _ = port;
        string_into_raw_ptr(response_string(
            false,
            None,
            None,
            Some("Server feature not enabled in this build".to_string()),
        ))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_stop_server() {
    crate::core::init_logger();

    #[cfg(feature = "server")]
    {
        let _ = std::panic::catch_unwind(|| {
            let _ = crate::core::runtime().block_on(async { crate::server::stop().await });
        });
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn ahutong_free_string(ptr: *mut c_char) {
    if ptr.is_null() {
        return;
    }

    // SAFETY: The pointer must have been returned by this SDK via CString::into_raw.
    unsafe {
        let _ = CString::from_raw(ptr);
    }
}
