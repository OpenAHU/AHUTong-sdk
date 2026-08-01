use anyhow::{Result, anyhow};
use reqwest::header::{CONTENT_TYPE, LOCATION};
use reqwest::{Client, ClientBuilder, RequestBuilder, Url};
use serde_json::Value;
use std::sync::Arc;
use tokio::sync::RwLock;

#[cfg(not(target_arch = "wasm32"))]
use reqwest_cookie_store::CookieStoreMutex;

#[derive(Clone)]
pub struct AHUClient {
    pub(crate) http: Client,
    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) cookie_store: Arc<CookieStoreMutex>,
    pub(crate) ycard_token: Arc<RwLock<Option<String>>>,
}

impl AHUClient {
    pub fn new() -> Self {
        #[cfg(not(target_arch = "wasm32"))]
        let cookie_store = Arc::new(CookieStoreMutex::default());

        #[cfg(not(target_arch = "wasm32"))]
        let http = ClientBuilder::new()
            .connect_timeout(std::time::Duration::from_secs(5))
            .timeout(std::time::Duration::from_secs(300))
            .cookie_provider(cookie_store.clone())
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        #[cfg(target_arch = "wasm32")]
        let http = ClientBuilder::new()
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            http,
            #[cfg(not(target_arch = "wasm32"))]
            cookie_store,
            ycard_token: Arc::new(RwLock::new(None)),
        }
    }

    /// 导出 Cookie 为 JSON 字符串 (供 Android 保存)
    pub fn dump_cookies_json(&self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let store = self.cookie_store.lock().unwrap();
            let mut w = Vec::new();
            store
                .save_incl_expired_and_nonpersistent_json(&mut w)
                .unwrap_or_default();
            String::from_utf8(w).unwrap_or_default()
        }
        #[cfg(target_arch = "wasm32")]
        {
            "[]".to_string()
        }
    }

    /// 从 JSON 字符串加载 Cookie (Android 启动时调用)
    pub fn load_cookies_json(&self, json: &str) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut store = self.cookie_store.lock().unwrap();
            if let Ok(cookies) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
                log::info!(
                    "[RustSDKCookie] Loading flat cookie list, count: {}",
                    cookies.len()
                );
                for c in cookies {
                    let name = c["name"].as_str().unwrap_or_default();
                    let value = c["value"].as_str().unwrap_or_default();
                    let domain = c["domain"].as_str().unwrap_or("jw.ahu.edu.cn");
                    let path = c["path"].as_str().unwrap_or("/");
                    let secure = c["secure"].as_bool().unwrap_or(false);
                    let http_only = c["http_only"].as_bool().unwrap_or(false);

                    let mut cookie_str = format!("{}={}", name, value);
                    if !domain.is_empty() {
                        cookie_str.push_str(&format!("; Domain={}", domain));
                    }
                    if !path.is_empty() {
                        cookie_str.push_str(&format!("; Path={}", path));
                    }
                    if secure {
                        cookie_str.push_str("; Secure");
                    }
                    if http_only {
                        cookie_str.push_str("; HttpOnly");
                    }

                    let url_str = if secure {
                        format!("https://{}{}", domain, path)
                    } else {
                        format!("http://{}{}", domain, path)
                    };

                    if let Ok(url) = url::Url::parse(&url_str) {
                        if store.parse(&cookie_str, &url).is_err() {
                            log::warn!("[RustSDKCookie] Failed to parse one restored cookie");
                        }
                    } else {
                        log::warn!("[RustSDKCookie] Invalid domain in restored cookie");
                    }
                }
            } else {
                log::info!("[RustSDKCookie] Failed to parse as flat list, trying native format...");
                let reader = std::io::Cursor::new(json.as_bytes());
                if let Ok(new_store) = cookie_store::CookieStore::load_json(reader) {
                    *store = new_store;
                    log::info!("[RustSDKCookie] Loaded native cookie store successfully.");
                } else {
                    log::error!("[RustSDKCookie] Failed to load cookies from JSON.");
                }
            }
        }
    }

    /// 获取扁平化的 Cookie 列表 JSON (用于同步给 Android OkHttp)
    pub fn get_cookies_flat_json(&self) -> String {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let store = self.cookie_store.lock().unwrap();
            let mut cookies = Vec::new();
            for cookie in store.iter_any() {
                let domain = cookie.domain().map(|s: &str| s.to_string()).or_else(|| {
                    let path = cookie.path().unwrap_or("/");
                    let name = cookie.name();
                    if path.contains("/cas") {
                        Some("one.ahu.edu.cn".to_string())
                    } else if path.contains("/student") {
                        Some("jw.ahu.edu.cn".to_string())
                    } else if name == "Language" {
                        Some("one.ahu.edu.cn".to_string())
                    } else if name == "JSESSIONID" && path == "/" {
                        Some("adwmh.ahu.edu.cn".to_string())
                    } else {
                        Some("jw.ahu.edu.cn".to_string())
                    }
                });

                cookies.push(serde_json::json!({
                    "name": cookie.name(),
                    "value": cookie.value(),
                    "domain": domain,
                    "path": cookie.path().unwrap_or("/"),
                    "secure": cookie.secure().unwrap_or(false),
                    "http_only": cookie.http_only().unwrap_or(false),
                }));
            }
            serde_json::to_string(&cookies).unwrap_or_else(|_| "[]".to_string())
        }
        #[cfg(target_arch = "wasm32")]
        {
            "[]".to_string()
        }
    }

    pub fn log_current_cookies(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let store = self.cookie_store.lock().unwrap();
            log::info!(
                "[RustSDKCookie] Current cookie count: {}",
                store.iter_any().count()
            );
        }
    }

    pub fn clear_cookies(&self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut store = self.cookie_store.lock().unwrap();
            *store = reqwest_cookie_store::CookieStore::default();
            log::info!("[RustSDKCookie] All cookies cleared.")
        }
    }

    /// Executes an authenticated request and inspects the response before any
    /// business parser runs. Error strings are stable and never include URLs,
    /// cookies, tokens, tickets, passwords, or request/response bodies.
    pub(crate) async fn authenticated_page(
        &self,
        request: RequestBuilder,
    ) -> Result<(String, Url)> {
        let response = request
            .send()
            .await
            .map_err(|_| anyhow!("campus_network_error"))?;
        let status = response.status();
        let final_url = response.url().clone();
        let content_type = response
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let redirect_location = response
            .headers()
            .get(LOCATION)
            .and_then(|value| value.to_str().ok())
            .map(str::to_owned);
        let body = response
            .text()
            .await
            .map_err(|_| anyhow!("campus_response_read_failed"))?;

        crate::data::session::ensure_authenticated_response(
            status,
            &final_url,
            content_type.as_deref(),
            redirect_location.as_deref(),
            &body,
        )?;

        if !status.is_success() {
            return Err(anyhow!("campus_upstream_http_status_{}", status.as_u16()));
        }

        Ok((body, final_url))
    }

    pub(crate) async fn authenticated_text(&self, request: RequestBuilder) -> Result<String> {
        self.authenticated_page(request).await.map(|(body, _)| body)
    }

    pub(crate) async fn authenticated_json(&self, request: RequestBuilder) -> Result<Value> {
        let body = self.authenticated_text(request).await?;
        serde_json::from_str(&body).map_err(|_| anyhow!("campus_json_parse_failed"))
    }
}
