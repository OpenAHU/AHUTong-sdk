use std::fmt::format;
use crate::data::api::client::AHUClient;
use serde_json::Value;
use reqwest::Result;
use reqwest::multipart;

const BASE_URL: &str = "https://adwmh.ahu.edu.cn";

impl AHUClient {
    // @GET("/remind/authcode")
    pub async fn get_auth_code(&self) -> Result<bytes::Bytes> {
        self.http
            .get(format!("{}/remind/authcode", BASE_URL))
            .send()
            .await?
            .bytes()
            .await
    }

    // @POST("/user/login")
    pub async fn login_with_captcha(
        &self,
        username: &str,
        password: &str,
        flag: i32,
        imgcode: &str
    ) -> Result<Value> {
        let params = [
            ("username", username),
            ("pwd", password),
            ("flag", &flag.to_string()),
            ("imgcode", imgcode),
        ];

        self.http
            .post(format!("{}/user/login", BASE_URL))
            .form(&params)
            .send()
            .await?
            .json::<Value>()
            .await
    }

    // @GET("/xzxcard/yue")
    pub async fn get_balance(&self) -> Result<Value> {
        self.http
            .get(format!("{}/xzxcard/yue", BASE_URL))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    // @GET("/xzxcard/qrcode")
    pub async fn get_qrcode(&self) -> Result<Value> {
        self.http
            .get(format!("{}/xzxcard/qrcode", BASE_URL))
            .send()
            .await?
            .json::<Value>()
            .await
    }

    pub async fn get_captcha_result(&self, url: &str, image_bytes: Vec<u8>) -> Result<Value> {
        let part = multipart::Part::bytes(image_bytes)
            .file_name("img.jpg")
            .mime_str("image/jpg")?;

        let form = multipart::Form::new()
            .part("captcha", part);

        // 使用临时的 Client，避免复用主 Client 的 Cookie 和配置
        // 强制使用 HTTP/1.1，因为 Flask 开发服务器可能不支持 HTTP/2
        // 添加 User-Agent 和 Connection: close 以提高兼容性
        let client = reqwest::Client::builder()
            .http1_only()
            .danger_accept_invalid_certs(true)
            .user_agent("AHUTong/Android")
            .build()?;

        client
            .post(url)
            .header("Connection", "close")
            .header("Host", "openahu.org")
            .multipart(form)
            .send()
            .await?
            .json::<Value>()
            .await
    }
}