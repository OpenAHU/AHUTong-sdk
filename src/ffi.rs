use std::ffi::{CString, c_char};

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
