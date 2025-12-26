use std::sync::Arc;
use reqwest::{Client, ClientBuilder};
use reqwest_cookie_store::CookieStoreMutex;
use tokio::sync::RwLock;

#[derive(Clone)]
pub struct AHUClient {
    pub(crate) http: Client,
    pub(crate) cookie_store: Arc<CookieStoreMutex>,
    pub(crate) ycard_token: Arc<RwLock<Option<String>>>,
}

impl AHUClient {
    pub fn new() -> Self {
        let cookie_store = Arc::new(CookieStoreMutex::default());

        let http = ClientBuilder::new()
            .cookie_provider(cookie_store.clone())
            .redirect(reqwest::redirect::Policy::limited(10))
            .user_agent("Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Mobile Safari/537.36")
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            http,
            cookie_store,
            ycard_token: Arc::new(RwLock::new(None)),
        }
    }

    /// 导出 Cookie 为 JSON 字符串 (供 Android 保存)
    pub fn dump_cookies_json(&self) -> String {
        let store = self.cookie_store.lock().unwrap();

        let mut w = Vec::new();
        store.save_json(&mut w).unwrap_or_default();
        String::from_utf8(w).unwrap_or_default()
    }

    /// 从 JSON 字符串加载 Cookie (Android 启动时调用)
    pub fn load_cookies_json(&self, json: &str) {
        let mut store = self.cookie_store.lock().unwrap();
        // 尝试解析扁平化的 Cookie 列表 (与 Android 互通的格式)
        if let Ok(cookies) = serde_json::from_str::<Vec<serde_json::Value>>(json) {
             log::info!("[RustSDKCookie] Loading flat cookie list, count: {}", cookies.len());
             for c in cookies {
                 let name = c["name"].as_str().unwrap_or_default();
                 let value = c["value"].as_str().unwrap_or_default();
                 let domain = c["domain"].as_str().unwrap_or("jw.ahu.edu.cn"); // 默认 domain
                 let path = c["path"].as_str().unwrap_or("/");
                 let secure = c["secure"].as_bool().unwrap_or(false);
                 let http_only = c["http_only"].as_bool().unwrap_or(false);

                 // 构建 RawCookie
                 // 注意：cookie_store 的 insert_raw 需要特定的格式，或者我们手动构造 Cookie 对象
                 // 这里为了简单，我们构造 Set-Cookie 字符串并解析
                 // Set-Cookie: name=value; Domain=domain; Path=path; ...
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
                 
                 // 使用 url 解析，因为 cookie 通常绑定到特定 URL
                 // 这里我们用 domain 构建一个模拟的 URL
                 let url_str = if secure {
                     format!("https://{}{}", domain, path)
                 } else {
                     format!("http://{}{}", domain, path)
                 };
                 
                 if let Ok(url) = url::Url::parse(&url_str) {
                      if let Err(e) = store.parse(&cookie_str, &url) {
                          log::warn!("[RustSDKCookie] Failed to parse cookie '{}': {:?}", name, e);
                      }
                 } else {
                     log::warn!("[RustSDKCookie] Invalid URL for cookie domain: {}", url_str);
                 }
             }
        } else {
            // 回退到原来的 load_json (如果是 cookie_store 原生格式)
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

    /// 获取扁平化的 Cookie 列表 JSON (用于同步给 Android OkHttp)
    pub fn get_cookies_flat_json(&self) -> String {
        let store = self.cookie_store.lock().unwrap();
        let mut cookies = Vec::new();
        for cookie in store.iter_any() {
            // Android 端存在一个 Bug：如果 domain 为 null，会抛出 NullPointerException。
            // 既然无法修改 Android 代码，我们在 Rust 端进行兜底修复，手动推断 domain。
            let domain = cookie.domain().map(|s| s.to_string()).or_else(|| {
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
                    // 默认兜底，防止 Android 崩溃
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

    pub fn log_current_cookies(&self) {
        let store = self.cookie_store.lock().unwrap();
        log::info!("[RustSDKCookie] Current Cookies in Store:");
        for cookie in store.iter_any() {
            log::info!("[RustSDKCookie] Name: {}, Domain: {:?}, Path: {:?}, Secure: {:?}, Expires: {:?}", 
                cookie.name(), cookie.domain(), cookie.path(), cookie.secure(), cookie.expires());
        }
    }
}